use std::{
    collections::HashMap,
    io::{Read, Write},
};

use mcml_names::i18_items::error_type::{CoreResult, ErrorData, ErrorType};

use crate::{NbtStream, NbtType, get_nbt, get_num, io_error, nbt_read, nbt_write};

pub struct NbtEnd {}

impl NbtEnd {
    pub fn new() -> Self {
        Self {}
    }
}

impl NbtStream for NbtEnd {
    fn read<R: Read>(&mut self, _stream: &mut R) -> CoreResult<()> {
        Ok(())
    }

    fn write<W: Write>(&self, _stream: &mut W) -> CoreResult<()> {
        Ok(())
    }
}

pub struct NbtByte {
    pub data: u8,
}

impl NbtByte {
    pub fn new(data: u8) -> Self {
        Self { data }
    }

    pub fn eq(&self, nbt: &NbtType) -> bool {

    }
}

impl NbtStream for NbtByte {
    fn read<R: Read>(&mut self, stream: &mut R) -> CoreResult<()> {
        let mut temp = [0u8; 1];
        stream.read_exact(&mut temp).map_err(|err| io_error(err))?;

        self.data = temp[0];

        Ok(())
    }

    fn write<W: Write>(&self, stream: &mut W) -> CoreResult<()> {
        let temp = [self.data];
        stream.write_all(&temp).map_err(|err| io_error(err))?;

        Ok(())
    }
}

pub struct NbtShort {
    pub data: i16,
}

impl NbtShort {
    pub fn new(data: i16) -> Self {
        Self { data }
    }
}

impl NbtStream for NbtShort {
    fn read<R: Read>(&mut self, stream: &mut R) -> CoreResult<()> {
        let mut temp = [0u8; 2];
        stream.read_exact(&mut temp).map_err(|err| io_error(err))?;

        self.data = i16::from_be_bytes(temp);

        Ok(())
    }

    fn write<W: Write>(&self, stream: &mut W) -> CoreResult<()> {
        let temp = i16::to_be_bytes(self.data);
        stream.write_all(&temp).map_err(|err| io_error(err))?;

        Ok(())
    }
}

pub struct NbtInt {
    pub data: i32,
}

impl NbtInt {
    pub fn new(data: i32) -> Self {
        Self { data }
    }
}

impl NbtStream for NbtInt {
    fn read<R: Read>(&mut self, stream: &mut R) -> CoreResult<()> {
        let mut temp = [0u8; 4];
        stream.read_exact(&mut temp).map_err(|err| io_error(err))?;

        self.data = i32::from_be_bytes(temp);

        Ok(())
    }

    fn write<W: Write>(&self, stream: &mut W) -> CoreResult<()> {
        let temp = i32::to_be_bytes(self.data);
        stream.write_all(&temp).map_err(|err| io_error(err))?;

        Ok(())
    }
}

pub struct NbtLong {
    pub data: i64,
}

impl NbtLong {
    pub fn new(data: i64) -> Self {
        Self { data }
    }
}

impl NbtStream for NbtLong {
    fn read<R: Read>(&mut self, stream: &mut R) -> CoreResult<()> {
        let mut temp = [0u8; 8];
        stream.read_exact(&mut temp).map_err(|err| io_error(err))?;

        self.data = i64::from_be_bytes(temp);

        Ok(())
    }

    fn write<W: Write>(&self, stream: &mut W) -> CoreResult<()> {
        let temp = i64::to_be_bytes(self.data);
        stream.write_all(&temp).map_err(|err| io_error(err))?;

        Ok(())
    }
}

pub struct NbtFloat {
    pub data: f32,
}

impl NbtFloat {
    pub fn new(data: f32) -> Self {
        Self { data }
    }
}

impl NbtStream for NbtFloat {
    fn read<R: Read>(&mut self, stream: &mut R) -> CoreResult<()> {
        let mut temp = [0u8; 4];
        stream.read_exact(&mut temp).map_err(|err| io_error(err))?;

        self.data = f32::from_be_bytes(temp);

        Ok(())
    }

    fn write<W: Write>(&self, stream: &mut W) -> CoreResult<()> {
        let temp = f32::to_be_bytes(self.data);
        stream.write_all(&temp).map_err(|err| io_error(err))?;

        Ok(())
    }
}

pub struct NbtDouble {
    pub data: f64,
}

impl NbtDouble {
    pub fn new(data: f64) -> Self {
        Self { data }
    }
}

impl NbtStream for NbtDouble {
    fn read<R: Read>(&mut self, stream: &mut R) -> CoreResult<()> {
        let mut temp = [0u8; 8];
        stream.read_exact(&mut temp).map_err(|err| io_error(err))?;

        self.data = f64::from_be_bytes(temp);

        Ok(())
    }

    fn write<W: Write>(&self, stream: &mut W) -> CoreResult<()> {
        let temp = f64::to_be_bytes(self.data);
        stream.write_all(&temp).map_err(|err| io_error(err))?;

        Ok(())
    }
}

pub struct NbtByteArray {
    pub data: Vec<u8>,
}

impl NbtByteArray {
    pub fn new(data: Vec<u8>) -> Self {
        Self { data }
    }
}

impl NbtStream for NbtByteArray {
    fn read<R: Read>(&mut self, stream: &mut R) -> CoreResult<()> {
        let mut temp = [0u8; 4];
        stream.read_exact(&mut temp).map_err(|err| io_error(err))?;

        let len = i32::from_be_bytes(temp);

        let mut temp = vec![0; len as usize];
        stream.read_exact(&mut temp).map_err(|err| io_error(err))?;

        self.data = temp;

        Ok(())
    }

    fn write<W: Write>(&self, stream: &mut W) -> CoreResult<()> {
        let temp = i32::to_be_bytes(self.data.len() as i32);
        stream.write_all(&temp).map_err(|err| io_error(err))?;
        stream.write_all(&self.data).map_err(|err| io_error(err))?;

        Ok(())
    }
}

pub struct NbtString {
    pub data: String,
}

impl NbtString {
    pub fn new(data: String) -> Self {
        Self { data }
    }
}

impl NbtStream for NbtString {
    fn read<R: Read>(&mut self, stream: &mut R) -> CoreResult<()> {
        let mut temp = [0u8; 2];
        stream.read_exact(&mut temp).map_err(|err| io_error(err))?;

        let len = i16::from_be_bytes(temp);

        let mut temp = vec![0; len as usize];
        stream.read_exact(&mut temp).map_err(|err| io_error(err))?;

        self.data = String::from_utf8(temp).map_err(|err| {
            ErrorType::StreamError(ErrorData {
                error: err.to_string(),
            })
        })?;

        Ok(())
    }

    fn write<W: Write>(&self, stream: &mut W) -> CoreResult<()> {
        let temp = i16::to_be_bytes(self.data.len() as i16);
        stream.write_all(&temp).map_err(|err| io_error(err))?;
        stream
            .write_all(&self.data.as_bytes())
            .map_err(|err| io_error(err))?;

        Ok(())
    }
}

pub struct NbtList {
    pub data: Vec<NbtType>,
    pub base: Box<NbtType>,
}

impl NbtList {
    pub fn new(base: NbtType) -> Self {
        Self {
            base: Box::new(base),
            data: Vec::new(),
        }
    }
}

impl NbtStream for NbtList {
    fn read<R: Read>(&mut self, stream: &mut R) -> CoreResult<()> {
        let mut temp = [0u8; 1];
        stream.read_exact(&mut temp).map_err(|err| io_error(err))?;

        let nbt_type = temp[0];
        let base = get_nbt(nbt_type);
        if base.is_some() {
            self.base = Box::new(base.unwrap());
        } else {
            return Err(ErrorType::NbtTypeError);
        }

        let mut temp = [0u8; 4];
        stream.read_exact(&mut temp).map_err(|err| io_error(err))?;

        let len = i32::from_be_bytes(temp);

        for _i in 0..len {
            let mut nbt = get_nbt(nbt_type).unwrap();
            nbt_read(&mut nbt, stream)?;
            self.data.push(nbt);
        }

        Ok(())
    }

    fn write<W: Write>(&self, stream: &mut W) -> CoreResult<()> {
        let nbt_type = if self.data.len() == 0 {
            0
        } else {
            get_num(&*self.base)
        };

        let temp = [nbt_type];
        stream.write_all(&temp).map_err(|err| io_error(err))?;

        let temp = i32::to_be_bytes(self.data.len() as i32);
        stream.write_all(&temp).map_err(|err| io_error(err))?;

        for nbt in &self.data {
            nbt_write(&nbt, stream)?;
        }

        Ok(())
    }
}

pub struct NbtCompound {
    pub data: HashMap<String, NbtType>,
}

impl NbtCompound {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }
}

impl NbtStream for NbtCompound {
    fn read<R: Read>(&mut self, stream: &mut R) -> CoreResult<()> {
        let mut temp = [0u8; 1];
        stream.read_exact(&mut temp).map_err(|err| io_error(err))?;

        if temp[0] == 0 {
            return Ok(());
        }

        let nbt_type = temp[0];
        let nbt = get_nbt(nbt_type);
        if nbt.is_none() {
            return Err(ErrorType::NbtTypeError);
        }

        let mut nbt = nbt.unwrap();

        let mut temp = [0u8; 2];
        stream.read_exact(&mut temp).map_err(|err| io_error(err))?;

        let len = i16::from_be_bytes(temp);

        let mut temp = vec![0; len as usize];
        stream.read_exact(&mut temp).map_err(|err| io_error(err))?;

        let key = String::from_utf8(temp).map_err(|err| {
            ErrorType::StreamError(ErrorData {
                error: err.to_string(),
            })
        })?;

        nbt_read(&mut nbt, stream)?;

        self.data.insert(key, nbt);

        Ok(())
    }

    fn write<W: Write>(&self, stream: &mut W) -> CoreResult<()> {
        for (key, nbt) in &self.data {
            let temp = [get_num(nbt)];
            stream.write_all(&temp).map_err(|err| io_error(err))?;

            if !matches!(nbt, NbtType::End(_)) {
                let temp = i16::to_be_bytes(key.len() as i16);
                stream.write_all(&temp).map_err(|err| io_error(err))?;
                stream
                    .write_all(key.as_bytes())
                    .map_err(|err| io_error(err))?;

                nbt_write(&nbt, stream)?;
            }
        }

        let temp = [0];
        stream.write_all(&temp).map_err(|err| io_error(err))?;

        Ok(())
    }
}

pub struct NbtIntArray {
    pub data: Vec<i32>,
}

impl NbtIntArray {
    pub fn new(data: Vec<i32>) -> Self {
        Self { data }
    }
}

impl NbtStream for NbtIntArray {
    fn read<R: Read>(&mut self, stream: &mut R) -> CoreResult<()> {
        let mut temp = [0u8; 4];
        stream.read_exact(&mut temp).map_err(|err| io_error(err))?;

        let len = i32::from_be_bytes(temp) * 4;

        let mut temp = vec![0; len as usize];
        stream.read_exact(&mut temp).map_err(|err| io_error(err))?;

        self.data = temp
            .chunks_exact(4)
            .map(|chunk| i32::from_be_bytes(chunk.try_into().unwrap()))
            .collect();

        Ok(())
    }

    fn write<W: Write>(&self, stream: &mut W) -> CoreResult<()> {
        let temp = i32::to_be_bytes(self.data.len() as i32);
        stream.write_all(&temp).map_err(|err| io_error(err))?;
        let temp: Vec<u8> = self
            .data
            .iter()
            .flat_map(|&value| value.to_be_bytes())
            .collect();
        stream.write_all(&temp).map_err(|err| io_error(err))?;

        Ok(())
    }
}

pub struct NbtLongArray {
    pub data: Vec<i64>,
}

impl NbtLongArray {
    pub fn new(data: Vec<i64>) -> Self {
        Self { data }
    }
}

impl NbtStream for NbtLongArray {
    fn read<R: Read>(&mut self, stream: &mut R) -> CoreResult<()> {
        let mut temp = [0u8; 4];
        stream.read_exact(&mut temp).map_err(|err| io_error(err))?;

        let len = i32::from_be_bytes(temp) * 8;

        let mut temp = vec![0; len as usize];
        stream.read_exact(&mut temp).map_err(|err| io_error(err))?;

        self.data = temp
            .chunks_exact(8)
            .map(|chunk| i64::from_be_bytes(chunk.try_into().unwrap()))
            .collect();

        Ok(())
    }

    fn write<W: Write>(&self, stream: &mut W) -> CoreResult<()> {
        let temp = i32::to_be_bytes(self.data.len() as i32);
        stream.write_all(&temp).map_err(|err| io_error(err))?;
        let temp: Vec<u8> = self
            .data
            .iter()
            .flat_map(|&value| value.to_be_bytes())
            .collect();
        stream.write_all(&temp).map_err(|err| io_error(err))?;

        Ok(())
    }
}
