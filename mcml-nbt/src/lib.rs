use std::io::{Read, Write};

use mcml_names::i18_items::error_type::{CoreResult, ErrorData, ErrorType};

use crate::nbt_types::{
    NbtByte, NbtByteArray, NbtCompound, NbtDouble, NbtEnd, NbtFloat, NbtInt, NbtIntArray, NbtList,
    NbtLong, NbtLongArray, NbtShort, NbtString,
};

pub mod nbt_file;
pub mod nbt_types;
pub mod chunky;

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
    pub fn get_nbt(nbt_type: u8) -> Option<NbtType> {
        if nbt_type > 12 {
            None
        } else {
            Some(match nbt_type {
                1 => NbtType::Byte(nbt_types::byte()),
                2 => NbtType::Short(nbt_types::short()),
                3 => NbtType::Int(nbt_types::int()),
                4 => NbtType::Long(nbt_types::long()),
                5 => NbtType::Float(nbt_types::float()),
                6 => NbtType::Double(nbt_types::double()),
                7 => NbtType::ByteArray(nbt_types::byte_array()),
                8 => NbtType::String(nbt_types::string()),
                9 => NbtType::List(nbt_types::list()),
                10 => NbtType::Compound(nbt_types::compound()),
                11 => NbtType::IntArray(nbt_types::int_array()),
                12 => NbtType::LongArray(nbt_types::long_array()),
                _ => NbtType::End(nbt_types::end()),
            })
        }
    }

    pub fn end() -> NbtType {
        NbtType::End(nbt_types::end())
    }

    pub fn byte() -> NbtType {
        NbtType::Byte(nbt_types::byte())
    }

    pub fn short() -> NbtType {
        NbtType::Short(nbt_types::short())
    }

    pub fn int() -> NbtType {
        NbtType::Int(nbt_types::int())
    }

    pub fn long() -> NbtType {
        NbtType::Long(nbt_types::long())
    }

    pub fn float() -> NbtType {
        NbtType::Float(nbt_types::float())
    }

    pub fn double() -> NbtType {
        NbtType::Double(nbt_types::double())
    }

    pub fn byte_array() -> NbtType {
        NbtType::ByteArray(nbt_types::byte_array())
    }

    pub fn string() -> NbtType {
        NbtType::String(nbt_types::string())
    }

    pub fn list() -> NbtType {
        NbtType::List(nbt_types::list())
    }

    pub fn compound() -> NbtType {
        NbtType::Compound(nbt_types::compound())
    }

    pub fn int_array() -> NbtType {
        NbtType::IntArray(nbt_types::int_array())
    }

    pub fn long_array() -> NbtType {
        NbtType::LongArray(nbt_types::long_array())
    }

    pub fn get_num(&self) -> u8 {
        match self {
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

    pub fn as_end(&self) -> Option<&NbtEnd> {
        if let NbtType::End(nbt) = self {
            Some(nbt)
        } else {
            None
        }
    }

    pub fn as_byte(&self) -> Option<&NbtByte> {
        if let NbtType::Byte(nbt) = self {
            Some(nbt)
        } else {
            None
        }
    }

    pub fn as_short(&self) -> Option<&NbtShort> {
        if let NbtType::Short(nbt) = self {
            Some(nbt)
        } else {
            None
        }
    }

    pub fn as_int(&self) -> Option<&NbtInt> {
        if let NbtType::Int(nbt) = self {
            Some(nbt)
        } else {
            None
        }
    }

    pub fn as_long(&self) -> Option<&NbtLong> {
        if let NbtType::Long(nbt) = self {
            Some(nbt)
        } else {
            None
        }
    }

    pub fn as_float(&self) -> Option<&NbtFloat> {
        if let NbtType::Float(nbt) = self {
            Some(nbt)
        } else {
            None
        }
    }

    pub fn as_double(&self) -> Option<&NbtDouble> {
        if let NbtType::Double(nbt) = self {
            Some(nbt)
        } else {
            None
        }
    }

    pub fn as_byte_array(&self) -> Option<&NbtByteArray> {
        if let NbtType::ByteArray(nbt) = self {
            Some(nbt)
        } else {
            None
        }
    }

    pub fn as_string(&self) -> Option<&NbtString> {
        if let NbtType::String(nbt) = self {
            Some(nbt)
        } else {
            None
        }
    }

    pub fn as_list(&self) -> Option<&NbtList> {
        if let NbtType::List(nbt) = self {
            Some(nbt)
        } else {
            None
        }
    }

    pub fn as_compound(&self) -> Option<&NbtCompound> {
        if let NbtType::Compound(nbt) = self {
            Some(nbt)
        } else {
            None
        }
    }

    pub fn as_int_array(&self) -> Option<&NbtIntArray> {
        if let NbtType::IntArray(nbt) = self {
            Some(nbt)
        } else {
            None
        }
    }

    pub fn as_long_array(&self) -> Option<&NbtLongArray> {
        if let NbtType::LongArray(nbt) = self {
            Some(nbt)
        } else {
            None
        }
    }

    pub fn eq(&self, nbt: &NbtType) -> bool {
        match self {
            NbtType::End(nbt_end) => nbt_end.eq(nbt),
            NbtType::Byte(nbt_byte) => nbt_byte.eq(nbt),
            NbtType::Short(nbt_short) => nbt_short.eq(nbt),
            NbtType::Int(nbt_int) => nbt_int.eq(nbt),
            NbtType::Long(nbt_long) => nbt_long.eq(nbt),
            NbtType::Float(nbt_float) => nbt_float.eq(nbt),
            NbtType::Double(nbt_double) => nbt_double.eq(nbt),
            NbtType::ByteArray(nbt_byte_array) => nbt_byte_array.eq(nbt),
            NbtType::String(nbt_string) => nbt_string.eq(nbt),
            NbtType::List(nbt_list) => nbt_list.eq(nbt),
            NbtType::Compound(nbt_compound) => nbt_compound.eq(nbt),
            NbtType::IntArray(nbt_int_array) => nbt_int_array.eq(nbt),
            NbtType::LongArray(nbt_long_array) => nbt_long_array.eq(nbt),
        }
    }

    pub(crate) fn nbt_read<R: Read>(&mut self, stream: &mut R) -> CoreResult<()> {
        match self {
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

    pub(crate) fn nbt_write<W: Write>(&self, stream: &mut W) -> CoreResult<()> {
        match self {
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
}

pub fn is_nbt_type(nbt_type: u8) -> bool {
    nbt_type > 0 && nbt_type <= 12
}

pub(crate) fn io_error(e: std::io::Error) -> ErrorType {
    ErrorType::StreamError(ErrorData {
        error: e.to_string(),
    })
}
