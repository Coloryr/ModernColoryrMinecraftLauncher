use std::{
    collections::HashMap,
    io::{Read, Write},
};

use mcml_names::i18_items::error_type::{CoreResult, ErrorData, ErrorType};

use crate::{NbtStream, NbtType, io_error, is_nbt_num};

pub struct NbtEnd {}

impl Default for NbtEnd {
    fn default() -> Self {
        Self::new()
    }
}

impl NbtEnd {
    pub fn new() -> Self {
        Self {}
    }

    pub fn eq(&self, nbt: &NbtType) -> bool {
        matches!(nbt, NbtType::End(_))
    }

    pub fn to_nbt(self) -> NbtType {
        NbtType::End(self)
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

impl Default for NbtByte {
    fn default() -> Self {
        Self {
            data: Default::default(),
        }
    }
}

impl NbtByte {
    pub fn new(data: u8) -> Self {
        Self { data }
    }

    pub fn eq(&self, nbt: &NbtType) -> bool {
        match nbt {
            NbtType::Byte(nbt) => nbt.data == self.data,
            _ => false,
        }
    }

    pub fn to_nbt(self) -> NbtType {
        NbtType::Byte(self)
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

impl Default for NbtShort {
    fn default() -> Self {
        Self {
            data: Default::default(),
        }
    }
}

impl NbtShort {
    pub fn new(data: i16) -> Self {
        Self { data }
    }

    pub fn eq(&self, nbt: &NbtType) -> bool {
        match nbt {
            NbtType::Short(nbt) => nbt.data == self.data,
            _ => false,
        }
    }

    pub fn to_nbt(self) -> NbtType {
        NbtType::Short(self)
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

impl Default for NbtInt {
    fn default() -> Self {
        Self {
            data: Default::default(),
        }
    }
}

impl NbtInt {
    pub fn new(data: i32) -> Self {
        Self { data }
    }

    pub fn eq(&self, nbt: &NbtType) -> bool {
        match nbt {
            NbtType::Int(nbt) => nbt.data == self.data,
            _ => false,
        }
    }

    pub fn to_nbt_type(self) -> NbtType {
        NbtType::Int(self)
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

impl Default for NbtLong {
    fn default() -> Self {
        Self {
            data: Default::default(),
        }
    }
}

impl NbtLong {
    pub fn new(data: i64) -> Self {
        Self { data }
    }

    pub fn eq(&self, nbt: &NbtType) -> bool {
        match nbt {
            NbtType::Long(nbt) => nbt.data == self.data,
            _ => false,
        }
    }

    pub fn to_nbt(self) -> NbtType {
        NbtType::Long(self)
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

impl Default for NbtFloat {
    fn default() -> Self {
        Self {
            data: Default::default(),
        }
    }
}

impl NbtFloat {
    pub fn new(data: f32) -> Self {
        Self { data }
    }

    pub fn eq(&self, nbt: &NbtType) -> bool {
        match nbt {
            NbtType::Float(nbt) => nbt.data == self.data,
            _ => false,
        }
    }

    pub fn to_nbt(self) -> NbtType {
        NbtType::Float(self)
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

impl Default for NbtDouble {
    fn default() -> Self {
        Self {
            data: Default::default(),
        }
    }
}

impl NbtDouble {
    pub fn new(data: f64) -> Self {
        Self { data }
    }

    pub fn eq(&self, nbt: &NbtType) -> bool {
        match nbt {
            NbtType::Double(nbt) => nbt.data == self.data,
            _ => false,
        }
    }

    pub fn to_nbt(self) -> NbtType {
        NbtType::Double(self)
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

impl Default for NbtByteArray {
    fn default() -> Self {
        Self {
            data: Default::default(),
        }
    }
}

impl NbtByteArray {
    pub fn new(data: Vec<u8>) -> Self {
        Self { data }
    }

    pub fn eq(&self, nbt: &NbtType) -> bool {
        match nbt {
            NbtType::ByteArray(nbt) => nbt.data == self.data,
            _ => false,
        }
    }

    pub fn to_nbt(self) -> NbtType {
        NbtType::ByteArray(self)
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

impl Default for NbtString {
    fn default() -> Self {
        Self {
            data: Default::default(),
        }
    }
}

impl NbtString {
    pub fn new(data: String) -> Self {
        Self { data }
    }

    pub fn eq(&self, nbt: &NbtType) -> bool {
        match nbt {
            NbtType::String(nbt) => nbt.data == self.data,
            _ => false,
        }
    }

    pub fn to_nbt(self) -> NbtType {
        NbtType::String(self)
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
    /// 数据列表
    data: Vec<NbtType>,
    /// 存入的类型
    nbt_num: u8,
}

impl Default for NbtList {
    fn default() -> Self {
        Self {
            data: Default::default(),
            nbt_num: Default::default(),
        }
    }
}

impl NbtList {
    pub fn new(nbt_num: u8) -> Self {
        Self {
            nbt_num,
            data: Vec::new(),
        }
    }

    pub fn set_type(&mut self, nbt_type: NbtType) {
        self.nbt_num = nbt_type.get_num();
        self.data.clear();
    }

    pub fn set_num(&mut self, nbt_num: u8) {
        if is_nbt_num(nbt_num) {
            self.nbt_num = nbt_num;
            self.data.clear();
        }
    }

    pub fn add_item(&mut self, nbt: NbtType) -> bool {
        if nbt.get_num() != self.nbt_num {
            false
        } else {
            self.data.push(nbt);

            true
        }
    }

    pub fn get_item(&self, index: usize) -> Option<&NbtType> {
        self.data.get(index)
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = &NbtType> {
        self.data.iter()
    }

    pub fn eq(&self, nbt: &NbtType) -> bool {
        match nbt {
            NbtType::List(nbt) => {
                if self.nbt_num != nbt.nbt_num {
                    return false;
                }
                if self.data.len() != nbt.data.len() {
                    return false;
                }

                for index in 0..self.data.len() {
                    let item1 = self.data.get(index).unwrap();
                    let item2 = nbt.data.get(index).unwrap();

                    if !item1.eq(item2) {
                        return false;
                    }
                }

                return true;
            }
            _ => false,
        }
    }

    pub fn to_nbt(self) -> NbtType {
        NbtType::List(self)
    }
}

impl NbtStream for NbtList {
    fn read<R: Read>(&mut self, stream: &mut R) -> CoreResult<()> {
        let mut temp = [0u8; 1];
        stream.read_exact(&mut temp).map_err(|err| io_error(err))?;

        self.nbt_num = temp[0];
        if !is_nbt_num(self.nbt_num) {
            return Err(ErrorType::NbtTypeError);
        }

        let mut temp = [0u8; 4];
        stream.read_exact(&mut temp).map_err(|err| io_error(err))?;

        let len = i32::from_be_bytes(temp);

        for _i in 0..len {
            let mut nbt = NbtType::get_nbt(self.nbt_num).unwrap();
            nbt.read(stream)?;
            self.data.push(nbt);
        }

        Ok(())
    }

    fn write<W: Write>(&self, stream: &mut W) -> CoreResult<()> {
        let nbt_type = if self.data.len() == 0 {
            0
        } else {
            self.nbt_num
        };

        let temp = [nbt_type];
        stream.write_all(&temp).map_err(|err| io_error(err))?;

        let temp = i32::to_be_bytes(self.data.len() as i32);
        stream.write_all(&temp).map_err(|err| io_error(err))?;

        for nbt in &self.data {
            nbt.write(stream)?;
        }

        Ok(())
    }
}

pub struct NbtCompound {
    pub data: HashMap<String, NbtType>,
}

impl Default for NbtCompound {
    fn default() -> Self {
        Self {
            data: Default::default(),
        }
    }
}

impl NbtCompound {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    pub fn eq(&self, nbt: &NbtType) -> bool {
        match nbt {
            NbtType::Compound(nbt) => {
                if self.data.len() != nbt.data.len() {
                    return false;
                }

                for (index, item1) in self.data.iter() {
                    let item2 = nbt.data.get(index);
                    if item2.is_none() {
                        return false;
                    }
                    let item2 = item2.unwrap();

                    if !item1.eq(item2) {
                        return false;
                    }
                }

                return true;
            }
            _ => false,
        }
    }

    pub fn get(&self, key: &str) -> Option<&NbtType> {
        self.data.get(key)
    }

    /// 从 NbtCompound 中提取 `&NbtByteArray`
    pub fn get_byte_array(&self, key: &str) -> Option<&NbtByteArray> {
        match self.get(key) {
            Some(NbtType::ByteArray(v)) => Some(v),
            _ => None,
        }
    }

    /// 从 NbtCompound 中提取 `&NbtLongArray`
    pub fn get_long_array(&self, key: &str) -> Option<&NbtLongArray> {
        match self.get(key) {
            Some(NbtType::LongArray(v)) => Some(v),
            _ => None,
        }
    }

    /// 从 NbtCompound 中提取 `&NbtCompound`
    pub fn get_compound(&self, key: &str) -> Option<&NbtCompound> {
        match self.get(key) {
            Some(NbtType::Compound(v)) => Some(v),
            _ => None,
        }
    }

    /// 从 NbtCompound 中提取 i64
    pub fn get_long(&self, key: &str) -> Option<i64> {
        match self.get(key) {
            Some(NbtType::Long(v)) => Some(v.data),
            _ => None,
        }
    }

    /// 从 NbtCompound 中提取 i16
    pub fn get_short(&self, key: &str) -> Option<i16> {
        match self.get(key) {
            Some(NbtType::Short(v)) => Some(v.data),
            _ => None,
        }
    }

    /// 从 NbtCompound 中提取 i32
    pub fn get_int(&self, key: &str) -> Option<i32> {
        match self.get(key) {
            Some(NbtType::Int(v)) => Some(v.data),
            _ => None,
        }
    }

    /// 从 NbtCompound 中提取 u8
    pub fn get_byte(&self, key: &str) -> Option<u8> {
        match self.get(key) {
            Some(NbtType::Byte(v)) => Some(v.data),
            _ => None,
        }
    }

    /// 从 NbtCompound 中提取 &str
    pub fn get_string(&self, key: &str) -> Option<String> {
        match self.get(key) {
            Some(NbtType::String(v)) => Some(v.data.clone()),
            _ => None,
        }
    }

    /// 从 NbtCompound 中提取 list
    pub fn get_list(&self, key: &str) -> Option<&NbtList> {
        match self.get(key) {
            Some(NbtType::List(v)) => Some(v),
            _ => None,
        }
    }

    pub fn to_nbt(self) -> NbtType {
        NbtType::Compound(self)
    }
}

impl NbtStream for NbtCompound {
    fn read<R: Read>(&mut self, stream: &mut R) -> CoreResult<()> {
        loop {
            let mut temp = [0u8; 1];
            stream.read_exact(&mut temp).map_err(|err| io_error(err))?;

            if temp[0] == 0 {
                return Ok(());
            }

            let nbt = NbtType::get_nbt(temp[0]);
            if nbt.is_none() {
                return Err(ErrorType::NbtTypeError);
            }

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

            let mut nbt = nbt.unwrap();
            nbt.read(stream)?;

            self.data.insert(key, nbt);
        }
    }

    fn write<W: Write>(&self, stream: &mut W) -> CoreResult<()> {
        for (key, nbt) in &self.data {
            let temp = [nbt.get_num()];
            stream.write_all(&temp).map_err(|err| io_error(err))?;

            if !matches!(nbt, NbtType::End(_)) {
                let temp = i16::to_be_bytes(key.len() as i16);
                stream.write_all(&temp).map_err(|err| io_error(err))?;
                stream
                    .write_all(key.as_bytes())
                    .map_err(|err| io_error(err))?;

                nbt.write(stream)?;
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

impl Default for NbtIntArray {
    fn default() -> Self {
        Self {
            data: Default::default(),
        }
    }
}

impl NbtIntArray {
    pub fn new(data: Vec<i32>) -> Self {
        Self { data }
    }

    pub fn eq(&self, nbt: &NbtType) -> bool {
        match nbt {
            NbtType::IntArray(nbt) => nbt.data == self.data,
            _ => false,
        }
    }

    pub fn to_nbt(self) -> NbtType {
        NbtType::IntArray(self)
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

impl Default for NbtLongArray {
    fn default() -> Self {
        Self {
            data: Default::default(),
        }
    }
}

impl NbtLongArray {
    pub fn new(data: Vec<i64>) -> Self {
        Self { data }
    }

    pub fn eq(&self, nbt: &NbtType) -> bool {
        match nbt {
            NbtType::LongArray(nbt) => nbt.data == self.data,
            _ => false,
        }
    }

    pub fn to_nbt(self) -> NbtType {
        NbtType::LongArray(self)
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

pub fn end() -> NbtEnd {
    NbtEnd::new()
}

pub fn byte(data: u8) -> NbtByte {
    NbtByte::new(data)
}

pub fn short(data: i16) -> NbtShort {
    NbtShort::new(data)
}

pub fn int(data: i32) -> NbtInt {
    NbtInt::new(data)
}

pub fn long(data: i64) -> NbtLong {
    NbtLong::new(data)
}

pub fn float(data: f32) -> NbtFloat {
    NbtFloat::new(data)
}

pub fn double(data: f64) -> NbtDouble {
    NbtDouble::new(data)
}

pub fn byte_array(data: Vec<u8>) -> NbtByteArray {
    NbtByteArray::new(data)
}

pub fn string(data: &str) -> NbtString {
    NbtString::new(String::from(data))
}

pub fn list(nbt_num: u8) -> NbtList {
    NbtList::new(nbt_num)
}

pub fn compound() -> NbtCompound {
    NbtCompound::new()
}

pub fn int_array(data: Vec<i32>) -> NbtIntArray {
    NbtIntArray::new(data)
}

pub fn long_array(data: Vec<i64>) -> NbtLongArray {
    NbtLongArray::new(data)
}
