//! NBT 文件的读写模块
//!
//! 本模块实现了 Minecraft NBT（Named Binary Tag）文件格式的完整读写支持。
//! NBT 文件可以以未压缩格式存储，也可以使用 GZip、Zlib 或 Lz4 压缩。
//! 文件格式遵循 Minecraft 规范：文件头为 NBT 根标签类型序号和根标签名称，
//! 后跟实际的 NBT 数据。
//!
//! # 支持的文件格式
//!
//! - `.dat` / `.nbt` — 未压缩的 NBT 文件
//! - GZip 压缩（Minecraft 默认格式，检测魔数 `1F 8B`）
//! - Zlib 压缩（检测魔数 `78`）
//! - Lz4 压缩（检测魔数 `4C 5A 34` 或 `04 22 4D`）
//!
//! # 支持的功能
//!
//! - 自动检测压缩格式（通过读取文件头魔数）
//! - 读取 Compound 根标签名称
//! - 读取和写入所有 NBT 标签类型
//! - 支持的 NBT 标签类型：End, Byte, Short, Int, Long, Float, Double,
//!   ByteArray, String, List, Compound, IntArray, LongArray

use std::fmt;
use std::io::{Read, Seek, SeekFrom, Write};

use flate2::{
    read::{GzDecoder, ZlibDecoder},
    write::{GzEncoder, ZlibEncoder},
};
use mcml_names::i18_items::error_type::{CoreResult, ErrorData, ErrorType};

use crate::nbt_types::NbtCompound;
use crate::{NBT_BYTE_ORDER, NBT_END_ORDER, NbtType, io_error, nbt_types};

/// NBT 文件的压缩类型
///
/// 表示 NBT 文件在存储时所使用的压缩格式。
/// 支持无压缩、GZip、Zlib 和 Lz4 四种模式。
#[derive(Clone, Debug, PartialEq)]
pub enum CompressType {
    /// 不进行压缩，原始数据直接读写
    None,
    /// GZip 压缩格式（魔数 `1F 8B`），Minecraft Java 版默认使用
    GZip,
    /// Zlib 压缩格式（魔数 `78`），用于部分旧版 Minecraft 数据
    Zlib,
    /// Lz4 压缩格式（魔数 `4C 5A 34` 或 `04 22 4D`），Minecraft 基岩版常用
    Lz4,
}

/// 以字符串形式展示压缩类型名称
impl fmt::Display for CompressType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CompressType::None => write!(f, "none"),
            CompressType::GZip => write!(f, "gzip"),
            CompressType::Zlib => write!(f, "zlib"),
            CompressType::Lz4 => write!(f, "lz4"),
        }
    }
}

/// NBT 文件结构体
///
/// 表示一个完整的 NBT 文件，包含：
/// - 根 NBT 标签（`nbt`），通常是 `Compound` 类型
/// - 压缩类型（`compress`），指示文件的压缩格式
///
/// # 示例
///
/// ```ignore
/// use mcml_nbt::nbt_file::{NbtFile, CompressType};
/// use mcml_nbt::NbtType;
///
/// // 从文件读取
/// let mut file = std::fs::File::open("level.dat").unwrap();
/// let nbt_file = NbtFile::read(&mut file).unwrap();
///
/// // 写入文件
/// let nbt_file = NbtFile::new(NbtType::compound(), CompressType::GZip);
/// let mut file = std::fs::File::create("output.dat").unwrap();
/// nbt_file.write(&mut file).unwrap();
/// ```
pub struct NbtFile {
    /// 文件的根 NBT 标签
    pub nbt: NbtType,
    /// 文件的压缩格式
    pub compress: CompressType,
}

/// NBT 文件的默认值
///
/// 默认创建一个空的 End 标签、无压缩的 NBT 文件。
impl Default for NbtFile {
    fn default() -> Self {
        Self {
            nbt: NbtType::end(),
            compress: CompressType::None,
        }
    }
}

impl NbtFile {
    /// 创建一个新的 NBT 文件实例
    ///
    /// # 参数
    ///
    /// - `nbt`: 根 NBT 标签
    /// - `compress`: 压缩格式
    ///
    /// # 返回值
    ///
    /// 返回配置好的 `NbtFile` 实例
    pub fn new(nbt: NbtType, compress: CompressType) -> Self {
        Self { nbt, compress }
    }

    /// 从流中读取 NBT 文件
    ///
    /// 自动检测压缩格式（通过读取文件头魔数），解压后解析 NBT 数据。
    /// 支持以下格式：
    /// - 单个 `End` 标签（1 字节 `0x00`）
    /// - 单个 `Byte` 标签（2 字节：类型 ID + 值）
    /// - 标准 NBT 文件（至少 3 字节头 + NBT 数据），可带压缩
    ///
    /// # 参数
    ///
    /// - `stream`: 实现了 `Read + Seek` 的文件流
    ///
    /// # 返回值
    ///
    /// 成功时返回 `NbtFile`，失败时返回错误
    ///
    /// # 错误
    ///
    /// - IO 错误 — 读取流失败
    /// - `NbtReadError` — 数据格式无效或 NBT 标签类型序号不合法
    pub fn read<R: Read + Seek>(stream: &mut R) -> CoreResult<Self> {
        // 读取前 3 个字节用于检测压缩格式
        let mut temp = [0u8; 3];
        let size = stream.read(&mut temp).map_err(|err| io_error(err))?;
        // 将流指针重置到起始位置
        stream
            .seek(SeekFrom::Start(0))
            .map_err(|err| io_error(err))?;

        // 处理仅包含 End 标签的文件（1 字节）
        if size == 1 && temp[0] == NBT_END_ORDER {
            return Ok(NbtFile {
                nbt: NbtType::end(),
                compress: CompressType::None,
            });
        }
        // 处理仅包含单个 Byte 标签的文件（2 字节）
        else if size == 2 && temp[0] == NBT_BYTE_ORDER {
            return Ok(NbtFile {
                nbt: nbt_types::byte(temp[1]).to_nbt(),
                compress: CompressType::None,
            });
        }

        // 文件头至少需要 3 字节
        if size != 3 {
            return Err(ErrorType::NbtReadError);
        }

        let mut compress_type = CompressType::None;

        // 通过魔数检测压缩格式并创建对应的解压流
        let mut stream: Box<dyn Read> = if temp[0] == 0x1F && temp[1] == 0x8B {
            // GZip 魔数：1F 8B
            compress_type = CompressType::GZip;
            Box::new(GzDecoder::new(stream))
        } else if temp[0] == 0x78 && (temp[1] == 0x01 || temp[1] == 0x9C || temp[1] == 0xDA) {
            // Zlib 魔数：78（标准/最佳/最大压缩级别）
            compress_type = CompressType::Zlib;
            Box::new(ZlibDecoder::new(stream))
        } else if (temp[0] == 0x4C && temp[1] == 0x5A && temp[2] == 0x34)
            || (temp[0] == 0x04 && temp[1] == 0x22 && temp[2] == 0x4D)
        {
            // Lz4 魔数：4C 5A 34（"LZ4"）或 04 22 4D（legacy 帧头）
            compress_type = CompressType::Lz4;
            Box::new(lz4_flex::frame::FrameDecoder::new(stream))
        } else {
            // 未检测到已知压缩格式，作为无压缩数据处理
            Box::new(stream)
        };

        // 读取根标签的类型序号
        let mut temp = [0u8; 1];
        stream.read_exact(&mut temp).map_err(|err| io_error(err))?;
        let nbt = NbtType::get_nbt(temp[0]);
        if nbt.is_none() {
            return Err(ErrorType::NbtReadError);
        }

        let mut nbt_inner = nbt.unwrap();
        let mut root_name = String::new();
        // 如果根标签是 Compound 类型，读取标签名称
        if matches!(nbt_inner, NbtType::Compound(_)) {
            // 读取名称长度（大端序 u16）
            let mut temp = [0u8; 2];
            stream.read_exact(&mut temp).map_err(|err| io_error(err))?;
            let len = u16::from_be_bytes(temp);
            if len > 0 {
                // 读取名称字符串（UTF-8 编码）
                let mut temp = vec![0; len as usize];
                stream.read_exact(&mut temp).map_err(|err| io_error(err))?;
                root_name = String::from_utf8(temp).map_err(|err| {
                    ErrorType::StreamError(ErrorData {
                        error: err.to_string(),
                    })
                })?;
            }
        }
        // 递归读取 NBT 标签数据
        nbt_inner.read(&mut stream)?;

        // 如果有根标签名称，将其包装在 Compound 中
        if root_name.is_empty() {
            Ok(NbtFile {
                nbt: nbt_inner,
                compress: compress_type,
            })
        } else {
            let mut com = NbtCompound::new();
            com.data.insert(root_name, nbt_inner);
            Ok(NbtFile {
                nbt: com.to_nbt(),
                compress: compress_type,
            })
        }
    }

    /// 将 NBT 文件写入流中
    ///
    /// 根据配置的压缩类型，先将 NBT 数据序列化，再用对应的压缩编码器
    /// 包装后写入目标流。写入的内容遵循 Minecraft NBT 文件格式规范。
    ///
    /// # 参数
    ///
    /// - `stream`: 实现了 `Write` 的目标流
    ///
    /// # 返回值
    ///
    /// 成功时返回 `Ok(())`，失败时返回 IO 错误
    pub fn write<W: Write>(&self, stream: &mut W) -> CoreResult<()> {
        // 根据压缩类型创建对应的编码器包装流
        let mut stream: Box<dyn Write> = match self.compress {
            CompressType::None => Box::new(stream),
            CompressType::GZip => Box::new(GzEncoder::new(stream, Default::default())),
            CompressType::Zlib => Box::new(ZlibEncoder::new(stream, Default::default())),
            CompressType::Lz4 => Box::new(lz4_flex::frame::FrameEncoder::new(stream)),
        };

        // 写入根标签类型序号
        let temp = [self.nbt.get_num()];
        stream.write_all(&temp).map_err(|err| io_error(err))?;

        // 如果是 Compound 类型，写入空的根标签名称（2 字节的 0）
        if matches!(self.nbt, NbtType::Compound(_)) {
            let mut temp = [0u8; 2];
            stream.write_all(&mut temp).map_err(|err| io_error(err))?;
        }

        // 写入 NBT 标签数据
        self.nbt.write(&mut stream)?;

        // 刷新缓冲区，确保所有数据写入底层流
        stream.flush().map_err(|err| io_error(err))?;

        Ok(())
    }
}

/// 以调试友好的格式展示 NBT 文件信息
///
/// 输出格式：`<NBT 内容> (compress: <压缩类型>)`
impl fmt::Display for NbtFile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} (compress: {})", self.nbt, self.compress)
    }
}
