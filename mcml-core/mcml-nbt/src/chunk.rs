/// 存档区块
use std::{
    io::{Cursor, Read, Seek, SeekFrom, Write},
    sync::Mutex,
};

use chrono::Utc;
use mcml_names::i18_items::error_type::{CoreResult, ErrorType};
use rayon::iter::*;

use crate::{
    io_error,
    nbt_file::{CompressType, NbtFile},
};

/// 区块坐标
pub struct PointI32 {
    /// X 坐标
    pub x: i32,
    /// Y 坐标
    pub y: i32,
}

/// 区块NBT
pub struct ChunkNbt {
    /// NBT 数据
    pub nbt: NbtFile,
    /// 区块坐标
    pub point: PointI32,
}

/// 区块头数据
#[derive(Debug, PartialEq, Eq)]
pub struct ChunkInfo {
    /// 序号
    pub index: u32,
    /// 位置
    pub pos: u32,
    /// 总计扇区数
    pub count: u8,
    /// 时间
    pub time: u32,
    /// 实际大小
    pub size: u32,
}

impl Default for ChunkInfo {
    fn default() -> Self {
        Self {
            index: Default::default(),
            pos: Default::default(),
            count: Default::default(),
            time: Default::default(),
            size: Default::default(),
        }
    }
}

/// 区块数据
pub struct ChunkData {
    /// NBT标签
    pub nbt: Vec<Option<ChunkNbt>>,
    /// 区块地址数据
    pub pos: Vec<ChunkInfo>,
}

/// 读区块头
///
/// - `stream`: 文件流
fn read_chunk_head<R: Read>(stream: &mut R) -> CoreResult<Vec<ChunkInfo>> {
    let mut temp = vec![0u8; 8192];

    stream.read_exact(&mut temp).map_err(|err| io_error(err))?;

    let mut pos = Vec::<ChunkInfo>::new();
    pos.resize_with(1024, || Default::default());
    for index in 0..1024 {
        let po: u32 = (temp[index * 4] as u32) << 16
            | (temp[(index * 4) + 1] as u32) << 8
            | temp[(index * 4) + 2] as u32;
        let time: u32 = (temp[(index * 4) + 4096] as u32) << 24
            | (temp[(index * 4) + 4097] as u32) << 16
            | (temp[(index * 4) + 4098] as u32) << 8
            | temp[(index * 4) + 4099] as u32;

        pos[index].pos = po * 4096;
        pos[index].count = temp[(index * 4) + 3];
        pos[index].time = time;
        pos[index].index = index as u32;
    }

    Ok(pos)
}

impl ChunkData {
    /// 读取区块
    ///
    /// - `stream`: 文件流
    pub fn read_chunk<R: Read + Seek + Send + Sync>(stream: &mut R) -> CoreResult<ChunkData> {
        let head = read_chunk_head(stream)?;
        let file_mutex = Mutex::new(stream);

        let nbts: Vec<Option<ChunkNbt>> = (0..head.len())
            .into_par_iter()
            .map(|idx| {
                let item = &head[idx];

                if item.count == 0 {
                    return Ok::<_, ErrorType>(None);
                }

                // 加锁读取文件
                let buffer = {
                    let mut file_guard = file_mutex.lock().unwrap();
                    file_guard
                        .seek(SeekFrom::Start(item.pos as u64))
                        .map_err(|err| io_error(err))?;

                    let mut temp = [0u8; 5];
                    file_guard
                        .read_exact(&mut temp)
                        .map_err(|err| io_error(err))?;

                    let item_size =
                        u32::from_be_bytes([temp[0], temp[1], temp[2], temp[3]]) as usize;
                    let mut buffer = vec![0u8; item_size - 1];
                    file_guard
                        .read_exact(&mut buffer)
                        .map_err(|err| io_error(err))?;

                    buffer
                };

                let mut cursor = Cursor::new(buffer);
                let nbt_file = NbtFile::read(&mut cursor)?;
                let nbt = nbt_file.nbt.as_compound();

                match nbt {
                    None => Err(ErrorType::NbtTypeError),
                    Some(nbt) => {
                        let mut x = 0;
                        let mut z = 0;
                        if let Some(nbt) = nbt.data.get("xPos").and_then(|v| v.as_int()) {
                            x = nbt.data;
                        }
                        if let Some(nbt) = nbt.data.get("zPos").and_then(|v| v.as_int()) {
                            z = nbt.data;
                        }
                        if let Some(value) = nbt.data.get("Position").and_then(|v| v.as_int_array())
                        {
                            if value.data.len() >= 2 {
                                x = value.data[0];
                                z = value.data[1];
                            }
                        }

                        Ok(Some(ChunkNbt {
                            nbt: nbt_file,
                            point: PointI32 { x, y: z },
                        }))
                    }
                }
            })
            .collect::<Vec<Result<_, _>>>()
            .into_iter()
            .collect::<Result<Vec<_>, _>>()?;

        Ok(ChunkData {
            nbt: nbts,
            pos: head,
        })
    }

    /// 写区块数据
    ///
    /// - `stream`: 文件流
    fn write_chunk_data<W: Write + Seek>(&mut self, stream: &mut W) -> CoreResult<()> {
        if self.nbt.len() == 0 {
            return Ok(());
        }

        let buf = vec![0u8; 8192];
        let mut now = 8192;
        stream.write_all(&buf).map_err(|err| io_error(err))?;

        let time = Utc::now().timestamp() as u32;

        for item in self.nbt.iter() {
            if item.is_none() {
                continue;
            }
            let nbt = item.as_ref().unwrap();
            let nbt_data = {
                let mut stream = Cursor::new(Vec::<u8>::new());
                nbt.nbt.write(&mut stream)?;
                stream.into_inner()
            };

            let len = nbt_data.len() + 5;
            let pos = chunk_to_head_pos(&nbt.point) as usize;

            self.pos[pos].time = time;
            self.pos[pos].pos = now / 4096;
            self.pos[pos].count = (len as f64 / 4096.0).ceil() as u8;

            let buf = u32::to_be_bytes((nbt_data.len() + 1) as u32);
            stream.write_all(&buf).map_err(|err| io_error(err))?;
            let buf = [get_chunk_compress_type(&nbt.nbt.compress)];
            stream.write_all(&buf).map_err(|err| io_error(err))?;
            stream.write_all(&nbt_data).map_err(|err| io_error(err))?;
            now += len as u32;
            let less = len % 4096;
            if less > 0 {
                let buf = vec![0u8; 4096 - less];
                stream.write_all(&buf).map_err(|err| io_error(err))?;
                now += buf.len() as u32;
            }
        }

        Ok(())
    }

    /// 写区块头
    ///
    /// - `stream`: 文件流
    fn write_head<W: Write + Seek>(&mut self, stream: &mut W) -> CoreResult<()> {
        stream
            .seek(SeekFrom::Start(0))
            .map_err(|err| io_error(err))?;

        for item in self.pos.iter() {
            if item.count == 0 {
                let data = [0u8; 4];
                stream.write_all(&data).map_err(|err| io_error(err))?;
            } else {
                let mut data = u32::to_be_bytes(item.pos);
                data[0] = data[1];
                data[1] = data[2];
                data[2] = data[3];
                data[3] = item.count;
                stream.write_all(&data).map_err(|err| io_error(err))?;
            }
        }
        for item in self.pos.iter() {
            if item.count == 0 {
                let data = [0u8; 4];
                stream.write_all(&data).map_err(|err| io_error(err))?;
            } else {
                let data = u32::to_be_bytes(item.time);
                stream.write_all(&data).map_err(|err| io_error(err))?;
            }
        }

        Ok(())
    }

    /// 写区块数据
    ///
    /// - `stream`: 文件流
    pub fn write_chunk<W: Write + Seek>(&mut self, stream: &mut W) -> CoreResult<()> {
        self.write_chunk_data(stream)?;
        self.write_head(stream)?;

        Ok(())
    }
}

/// 坐标转区块坐标
pub fn pos_to_chunk(pos: &PointI32) -> PointI32 {
    PointI32 {
        x: pos.x >> 4,
        y: pos.y >> 4,
    }
}

/// 区块转MCA坐标
pub fn chunk_to_region(pos: &PointI32) -> PointI32 {
    PointI32 {
        x: pos.x >> 5,
        y: pos.y >> 5,
    }
}

/// 区块坐标转文件头位置
pub fn chunk_to_head_pos(pos: &PointI32) -> i32 {
    (pos.x & 31) + (pos.y & 31) * 32
}

/// 获取区块压缩类型
/// 
/// - `compress`: 压缩类型
fn get_chunk_compress_type(compress: &CompressType) -> u8 {
    match compress {
        CompressType::None => 3,
        CompressType::GZip => 1,
        CompressType::Zlib => 2,
        CompressType::Lz4 => 4,
    }
}
