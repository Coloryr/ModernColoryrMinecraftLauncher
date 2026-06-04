use std::io::{Read, Write};

use mcml_names::i18_items::error_type::{CoreResult, ErrorData, ErrorType};

use crate::nbt_type::{
    NbtByte, NbtByteArray, NbtCompound, NbtDouble, NbtEnd, NbtFloat, NbtInt, NbtIntArray, NbtList,
    NbtLong, NbtLongArray, NbtShort, NbtString,
};

pub mod nbt;
pub mod nbt_type;

pub(crate) trait NbtStream {
    fn read<R: Read>(&mut self, stream: &mut R) -> CoreResult<()>;
    fn write<W: Write>(&self, stream: &mut W) -> CoreResult<()>;
}

pub enum NbtType {
    End(NbtEnd),
    Byte(NbtByte),
    Short(NbtShort),
    Int(NbtInt),
    Long(NbtLong),
    Float(NbtFloat),
    Double(NbtDouble),
    ByteArray(NbtByteArray),
    String(NbtString),
    List(NbtList),
    Compound(NbtCompound),
    IntArray(NbtIntArray),
    LongArray(NbtLongArray),
}

impl NbtType {
    pub fn eq(&self, nbt: &NbtType) -> bool {
        if get_num(self) != get_num(nbt) {
            return false;
        }

        match self {
            NbtType::End(nbt_end) => todo!(),
            NbtType::Byte(nbt_byte) => todo!(),
            NbtType::Short(nbt_short) => todo!(),
            NbtType::Int(nbt_int) => todo!(),
            NbtType::Long(nbt_long) => todo!(),
            NbtType::Float(nbt_float) => todo!(),
            NbtType::Double(nbt_double) => todo!(),
            NbtType::ByteArray(nbt_byte_array) => todo!(),
            NbtType::String(nbt_string) => todo!(),
            NbtType::List(nbt_list) => todo!(),
            NbtType::Compound(nbt_compound) => todo!(),
            NbtType::IntArray(nbt_int_array) => todo!(),
            NbtType::LongArray(nbt_long_array) => todo!(),
        }
    }
}

impl Default for NbtType {
    fn default() -> Self {
        get_nbt(0).unwrap()
    }
}

pub fn get_nbt(nbt_type: u8) -> Option<NbtType> {
    if nbt_type > 12 {
        None
    } else {
        Some(match nbt_type {
            1 => NbtType::Byte(NbtByte::new(Default::default())),
            2 => NbtType::Short(NbtShort::new(Default::default())),
            3 => NbtType::Int(NbtInt::new(Default::default())),
            4 => NbtType::Long(NbtLong::new(Default::default())),
            5 => NbtType::Float(NbtFloat::new(Default::default())),
            6 => NbtType::Double(NbtDouble::new(Default::default())),
            7 => NbtType::ByteArray(NbtByteArray::new(Default::default())),
            8 => NbtType::String(NbtString::new(Default::default())),
            9 => NbtType::List(NbtList::new(Default::default())),
            10 => NbtType::Compound(NbtCompound::new()),
            11 => NbtType::IntArray(NbtIntArray::new(Default::default())),
            12 => NbtType::LongArray(NbtLongArray::new(Default::default())),
            _ => NbtType::End(NbtEnd::new()),
        })
    }
}

pub fn get_num(nbt: &NbtType) -> u8 {
    match nbt {
        NbtType::End(_) => 0,
        NbtType::Byte(_) => 1,
        NbtType::Short(_) => 2,
        NbtType::Int(_) => 3,
        NbtType::Long(_) => 4,
        NbtType::Float(_) => 5,
        NbtType::Double(_) => 6,
        NbtType::ByteArray(_) => 7,
        NbtType::String(_) => 8,
        NbtType::List(_) => 9,
        NbtType::Compound(_) => 10,
        NbtType::IntArray(_) => 11,
        NbtType::LongArray(_) => 12,
    }
}

pub(crate) fn io_error(e: std::io::Error) -> ErrorType {
    ErrorType::StreamError(ErrorData {
        error: e.to_string(),
    })
}

pub(crate) fn nbt_read<R: Read>(nbt: &mut NbtType, stream: &mut R) -> CoreResult<()> {
    match nbt {
        NbtType::End(nbt_end) => nbt_end.read(stream),
        NbtType::Byte(nbt_byte) => nbt_byte.read(stream),
        NbtType::Short(nbt_short) => nbt_short.read(stream),
        NbtType::Int(nbt_int) => nbt_int.read(stream),
        NbtType::Long(nbt_long) => nbt_long.read(stream),
        NbtType::Float(nbt_float) => nbt_float.read(stream),
        NbtType::Double(nbt_double) => nbt_double.read(stream),
        NbtType::ByteArray(nbt_byte_array) => nbt_byte_array.read(stream),
        NbtType::String(nbt_string) => nbt_string.read(stream),
        NbtType::List(nbt_list) => nbt_list.read(stream),
        NbtType::Compound(nbt_compound) => nbt_compound.read(stream),
        NbtType::IntArray(nbt_int_array) => nbt_int_array.read(stream),
        NbtType::LongArray(nbt_long_array) => nbt_long_array.read(stream),
    }?;

    Ok(())
}

pub(crate) fn nbt_write<W: Write>(nbt: &NbtType, stream: &mut W) -> CoreResult<()> {
    match nbt {
        NbtType::End(nbt_end) => nbt_end.write(stream),
        NbtType::Byte(nbt_byte) => nbt_byte.write(stream),
        NbtType::Short(nbt_short) => nbt_short.write(stream),
        NbtType::Int(nbt_int) => nbt_int.write(stream),
        NbtType::Long(nbt_long) => nbt_long.write(stream),
        NbtType::Float(nbt_float) => nbt_float.write(stream),
        NbtType::Double(nbt_double) => nbt_double.write(stream),
        NbtType::ByteArray(nbt_byte_array) => nbt_byte_array.write(stream),
        NbtType::String(nbt_string) => nbt_string.write(stream),
        NbtType::List(nbt_list) => nbt_list.write(stream),
        NbtType::Compound(nbt_compound) => nbt_compound.write(stream),
        NbtType::IntArray(nbt_int_array) => nbt_int_array.write(stream),
        NbtType::LongArray(nbt_long_array) => nbt_long_array.write(stream),
    }?;

    Ok(())
}
