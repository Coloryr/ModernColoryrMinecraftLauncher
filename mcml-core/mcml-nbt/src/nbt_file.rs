use std::fmt;
use std::io::{Read, Seek, SeekFrom, Write};

use flate2::{
    read::{GzDecoder, ZlibDecoder},
    write::{GzEncoder, ZlibEncoder},
};
use mcml_names::i18_items::error_type::{CoreResult, ErrorData, ErrorType};

use crate::nbt_types::NbtCompound;
use crate::{NBT_BYTE_ORDER, NBT_END_ORDER, NbtType, io_error, nbt_types};

#[derive(Clone, Debug, PartialEq)]
pub enum CompressType {
    None,
    GZip,
    Zlib,
    Lz4,
}

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

pub struct NbtFile {
    pub nbt: NbtType,
    pub compress: CompressType,
}

impl Default for NbtFile {
    fn default() -> Self {
        Self {
            nbt: NbtType::end(),
            compress: CompressType::None,
        }
    }
}

impl NbtFile {
    pub fn new(nbt: NbtType, compress: CompressType) -> Self {
        Self { nbt, compress }
    }

    /// 从流中读取NBT文件
    ///
    /// - `stream`: 流
    pub fn read<R: Read + Seek>(stream: &mut R) -> CoreResult<Self> {
        let mut temp = [0u8; 3];
        let size = stream.read(&mut temp).map_err(|err| io_error(err))?;
        stream
            .seek(SeekFrom::Start(0))
            .map_err(|err| io_error(err))?;

        if size == 1 && temp[0] == NBT_END_ORDER {
            return Ok(NbtFile {
                nbt: NbtType::end(),
                compress: CompressType::None,
            });
        } else if size == 2 && temp[0] == NBT_BYTE_ORDER {
            return Ok(NbtFile {
                nbt: nbt_types::byte(temp[1]).to_nbt(),
                compress: CompressType::None,
            });
        }

        if size != 3 {
            return Err(ErrorType::NbtReadError);
        }

        let mut compress_type = CompressType::None;

        let mut stream: Box<dyn Read> = if temp[0] == 0x1F && temp[1] == 0x8B {
            compress_type = CompressType::GZip;
            Box::new(GzDecoder::new(stream))
        } else if temp[0] == 0x78 && (temp[1] == 0x01 || temp[1] == 0x9C || temp[1] == 0xDA) {
            compress_type = CompressType::Zlib;
            Box::new(ZlibDecoder::new(stream))
        } else if (temp[0] == 0x4C && temp[1] == 0x5A && temp[2] == 0x34)
            || (temp[0] == 0x04 && temp[1] == 0x22 && temp[2] == 0x4D)
        {
            compress_type = CompressType::Lz4;
            Box::new(lz4_flex::frame::FrameDecoder::new(stream))
        } else {
            Box::new(stream)
        };

        let mut temp = [0u8; 1];
        stream.read_exact(&mut temp).map_err(|err| io_error(err))?;
        let nbt = NbtType::get_nbt(temp[0]);
        if nbt.is_none() {
            return Err(ErrorType::NbtReadError);
        }

        let mut nbt_inner = nbt.unwrap();
        let mut root_name = String::new();
        if matches!(nbt_inner, NbtType::Compound(_)) {
            let mut temp = [0u8; 2];
            stream.read_exact(&mut temp).map_err(|err| io_error(err))?;
            let len = u16::from_be_bytes(temp);
            if len > 0 {
                let mut temp = vec![0; len as usize];
                stream.read_exact(&mut temp).map_err(|err| io_error(err))?;
                root_name = String::from_utf8(temp).map_err(|err| {
                    ErrorType::StreamError(ErrorData {
                        error: err.to_string(),
                    })
                })?;
            }
        }
        nbt_inner.read(&mut stream)?;

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

    /// 从流中保存NBT文件
    ///
    /// - `stream`: 流
    pub fn write<W: Write>(&self, stream: &mut W) -> CoreResult<()> {
        let mut stream: Box<dyn Write> = match self.compress {
            CompressType::None => Box::new(stream),
            CompressType::GZip => Box::new(GzEncoder::new(stream, Default::default())),
            CompressType::Zlib => Box::new(ZlibEncoder::new(stream, Default::default())),
            CompressType::Lz4 => Box::new(lz4_flex::frame::FrameEncoder::new(stream)),
        };

        let temp = [self.nbt.get_num()];
        stream.write_all(&temp).map_err(|err| io_error(err))?;
        if matches!(self.nbt, NbtType::Compound(_)) {
            let mut temp = [0u8; 2];
            stream.write_all(&mut temp).map_err(|err| io_error(err))?;
        }
        self.nbt.write(&mut stream)?;

        stream.flush().map_err(|err| io_error(err))?;

        Ok(())
    }
}

impl fmt::Display for NbtFile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} (compress: {})", self.nbt, self.compress)
    }
}
