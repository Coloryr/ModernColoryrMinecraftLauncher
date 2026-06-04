use std::io::{Read, Seek, SeekFrom, Write};

use flate2::{
    read::{GzDecoder, ZlibDecoder},
    write::{GzEncoder, ZlibEncoder},
};
use mcml_names::i18_items::error_type::{CoreResult, ErrorType};

use crate::{NbtType, get_nbt, get_num, io_error, nbt_read, nbt_write};

#[derive(Clone, Debug, PartialEq)]
pub enum CompressType {
    None,
    GZip,
    Zlib,
    Lz4,
}

pub struct Nbt {
    pub nbt: NbtType,
    pub compress: CompressType,
}

impl Nbt {
    pub fn new(nbt: NbtType, compress: CompressType) -> Self {
        Self { nbt, compress }
    }

    /// 从流中读取NBT文件
    /// 
    /// - `stream`: 流
    pub fn read<R: Read + Seek>(stream: &mut R) -> CoreResult<Self> {
        let mut temp = [0u8; 3];
        stream.read_exact(&mut temp).map_err(|err| io_error(err))?;
        stream
            .seek(SeekFrom::Start(0))
            .map_err(|err| io_error(err))?;

        let mut compress_type = CompressType::None;

        let mut stream: Box<dyn Read> = if temp[0] == 0x1F && temp[1] == 0x8B {
            compress_type = CompressType::GZip;
            Box::new(GzDecoder::new(stream))
        } else if temp[0] == 0x78 && (temp[1] == 0x01 || temp[1] == 0x9C || temp[1] == 0xDA) {
            compress_type = CompressType::Zlib;
            Box::new(ZlibDecoder::new(stream))
        } else if temp[0] == 0x4C && temp[1] == 0x5A && temp[2] == 0x34 {
            compress_type = CompressType::Lz4;
            Box::new(lz4_flex::frame::FrameDecoder::new(stream))
        } else {
            Box::new(stream)
        };

        let mut temp = [0u8; 1];
        stream.read_exact(&mut temp).map_err(|err| io_error(err))?;
        let nbt = get_nbt(temp[0]);
        if nbt.is_none() {
            return Err(ErrorType::NbtReadError);
        }

        let mut nbt = nbt.unwrap();

        nbt_read(&mut nbt, &mut stream)?;

        Ok(Nbt {
            nbt: nbt,
            compress: compress_type,
        })
    }

    /// 从流中保存NBT文件
    /// 
    /// - `stream`: 流
    pub fn save<W: Write>(&self, stream: &mut W) -> CoreResult<()> {
        let mut stream: Box<dyn Write> = match self.compress {
            CompressType::None => Box::new(stream),
            CompressType::GZip => Box::new(GzEncoder::new(stream, Default::default())),
            CompressType::Zlib => Box::new(ZlibEncoder::new(stream, Default::default())),
            CompressType::Lz4 => Box::new(lz4_flex::frame::FrameEncoder::new(stream)),
        };

        let temp = [get_num(&self.nbt)];
        stream.write_all(&temp).map_err(|err| io_error(err))?;

        nbt_write(&self.nbt, &mut stream)
    }
}
