use std::fmt;
use std::io::{Read, Write};

use mcml_names::i18_items::error_type::{CoreResult, ErrorData, ErrorType};

use crate::nbt_types::{
    NbtByte, NbtByteArray, NbtCompound, NbtDouble, NbtEnd, NbtFloat, NbtInt, NbtIntArray, NbtList,
    NbtLong, NbtLongArray, NbtShort, NbtString,
};

pub mod chunk;
pub mod nbt_file;
pub mod nbt_types;

pub const NBT_END_ORDER: u8 = 0;
pub const NBT_BYTE_ORDER: u8 = 1;
pub const NBT_SHORT_ORDER: u8 = 2;
pub const NBT_INT_ORDER: u8 = 3;
pub const NBT_LONG_ORDER: u8 = 4;
pub const NBT_FLOAT_ORDER: u8 = 5;
pub const NBT_DOUBLE_ORDER: u8 = 6;
pub const NBT_BYTE_ARRAY_ORDER: u8 = 7;
pub const NBT_STRING_ORDER: u8 = 8;
pub const NBT_LIST_ORDER: u8 = 9;
pub const NBT_COMPOUND_ORDER: u8 = 10;
pub const NBT_INT_ARRAY_ORDER: u8 = 11;
pub const NBT_LONG_ARRAY_ORDER: u8 = 12;

/// NBT读写流接口
pub(crate) trait NbtStream {
    /// NBT标签读
    fn read<R: Read>(&mut self, stream: &mut R) -> CoreResult<()>;
    /// NBT标签写
    fn write<W: Write>(&self, stream: &mut W) -> CoreResult<()>;
}

/// NBT类型
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
    /// 从数字序号创建NBT标签
    pub fn get_nbt(nbt_type: u8) -> Option<NbtType> {
        if nbt_type > 12 {
            None
        } else {
            Some(match nbt_type {
                NBT_BYTE_ORDER => Self::byte(),
                NBT_SHORT_ORDER => Self::short(),
                NBT_INT_ORDER => Self::int(),
                NBT_LONG_ORDER => Self::long(),
                NBT_FLOAT_ORDER => Self::float(),
                NBT_DOUBLE_ORDER => Self::double(),
                NBT_BYTE_ARRAY_ORDER => Self::byte_array(),
                NBT_STRING_ORDER => Self::string(),
                NBT_LIST_ORDER => Self::list(),
                NBT_COMPOUND_ORDER => Self::compound(),
                NBT_INT_ARRAY_ORDER => Self::int_array(),
                NBT_LONG_ARRAY_ORDER => Self::long_array(),
                _ => Self::end(),
            })
        }
    }

    pub fn end() -> NbtType {
        NbtType::End(Default::default())
    }

    pub fn byte() -> NbtType {
        NbtType::Byte(Default::default())
    }

    pub fn short() -> NbtType {
        NbtType::Short(Default::default())
    }

    pub fn int() -> NbtType {
        NbtType::Int(Default::default())
    }

    pub fn long() -> NbtType {
        NbtType::Long(Default::default())
    }

    pub fn float() -> NbtType {
        NbtType::Float(Default::default())
    }

    pub fn double() -> NbtType {
        NbtType::Double(Default::default())
    }

    pub fn byte_array() -> NbtType {
        NbtType::ByteArray(Default::default())
    }

    pub fn string() -> NbtType {
        NbtType::String(Default::default())
    }

    pub fn list() -> NbtType {
        NbtType::List(Default::default())
    }

    pub fn compound() -> NbtType {
        NbtType::Compound(Default::default())
    }

    pub fn int_array() -> NbtType {
        NbtType::IntArray(Default::default())
    }

    pub fn long_array() -> NbtType {
        NbtType::LongArray(Default::default())
    }

    /// 从NBT标签读对应的数字序号
    pub fn get_num(&self) -> u8 {
        match self {
            NbtType::End(_) => NBT_END_ORDER,
            NbtType::Byte(_) => NBT_BYTE_ORDER,
            NbtType::Short(_) => NBT_SHORT_ORDER,
            NbtType::Int(_) => NBT_INT_ORDER,
            NbtType::Long(_) => NBT_LONG_ORDER,
            NbtType::Float(_) => NBT_FLOAT_ORDER,
            NbtType::Double(_) => NBT_DOUBLE_ORDER,
            NbtType::ByteArray(_) => NBT_BYTE_ARRAY_ORDER,
            NbtType::String(_) => NBT_STRING_ORDER,
            NbtType::List(_) => NBT_LIST_ORDER,
            NbtType::Compound(_) => NBT_COMPOUND_ORDER,
            NbtType::IntArray(_) => NBT_INT_ARRAY_ORDER,
            NbtType::LongArray(_) => NBT_LONG_ARRAY_ORDER,
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

    pub fn get_compound(self) -> Option<NbtCompound> {
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

    /// 判断两个NBT标签是否一致
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

    /// NBT标签读
    pub(crate) fn read<R: Read>(&mut self, stream: &mut R) -> CoreResult<()> {
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

    /// NBT标签写
    pub(crate) fn write<W: Write>(&self, stream: &mut W) -> CoreResult<()> {
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

impl fmt::Display for NbtType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NbtType::End(_) => write!(f, "END"),
            NbtType::Byte(nbt) => write!(f, "{}b", nbt.data),
            NbtType::Short(nbt) => write!(f, "{}s", nbt.data),
            NbtType::Int(nbt) => write!(f, "{}", nbt.data),
            NbtType::Long(nbt) => write!(f, "{}L", nbt.data),
            NbtType::Float(nbt) => write!(f, "{}f", nbt.data),
            NbtType::Double(nbt) => write!(f, "{}d", nbt.data),
            NbtType::ByteArray(nbt) => {
                write!(f, "[B;")?;
                for (i, b) in nbt.data.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}B", b)?;
                }
                write!(f, "]")
            }
            NbtType::String(nbt) => write!(f, "\"{}\"", nbt.data.escape_default()),
            NbtType::List(nbt) => {
                write!(f, "[")?;
                for (i, item) in nbt.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", item)?;
                }
                write!(f, "]")
            }
            NbtType::Compound(nbt) => {
                write!(f, "{{")?;
                let mut first = true;
                for (key, value) in &nbt.data {
                    if !first {
                        write!(f, ", ")?;
                    }
                    first = false;
                    write!(f, "{}: {}", key, value)?;
                }
                write!(f, "}}")
            }
            NbtType::IntArray(nbt) => {
                write!(f, "[I;")?;
                for (i, v) in nbt.data.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", v)?;
                }
                write!(f, "]")
            }
            NbtType::LongArray(nbt) => {
                write!(f, "[L;")?;
                for (i, v) in nbt.data.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}L", v)?;
                }
                write!(f, "]")
            }
        }
    }
}

/// 是否为NBT标签数字
pub fn is_nbt_num(nbt_type: u8) -> bool {
    nbt_type >= NBT_END_ORDER && nbt_type <= NBT_LONG_ARRAY_ORDER
}

pub(crate) fn io_error(e: std::io::Error) -> ErrorType {
    ErrorType::StreamError(ErrorData {
        error: e.to_string(),
    })
}
