//! NBT 标签类型定义模块
//!
//! 本模块定义了 Minecraft NBT（Named Binary Tag）规范中的全部 13 种标签类型。
//! 每种类型都是一个独立的 Rust 结构体，封装了具体的数据和二进制序列化/反序列化逻辑。
//!
//! # 类型列表
//!
//! | 类型 | Rust 结构体 | 对应数据 | 二进制大小 |
//! |------|-----------|---------|-----------|
//! | TAG_End | [`NbtEnd`] | 无数据 | 0 字节 |
//! | TAG_Byte | [`NbtByte`] | `u8` | 1 字节 |
//! | TAG_Short | [`NbtShort`] | `i16` | 2 字节（大端序） |
//! | TAG_Int | [`NbtInt`] | `i32` | 4 字节（大端序） |
//! | TAG_Long | [`NbtLong`] | `i64` | 8 字节（大端序） |
//! | TAG_Float | [`NbtFloat`] | `f32` | 4 字节（IEEE 754 大端序） |
//! | TAG_Double | [`NbtDouble`] | `f64` | 8 字节（IEEE 754 大端序） |
//! | TAG_Byte_Array | [`NbtByteArray`] | `Vec<u8>` | 4 字节长度 + 数据 |
//! | TAG_String | [`NbtString`] | `String` | 2 字节长度 + UTF-8 数据 |
//! | TAG_List | [`NbtList`] | `Vec<NbtType>` | 1 字节元素类型 + 4 字节长度 + 数据 |
//! | TAG_Compound | [`NbtCompound`] | `HashMap<String, NbtType>` | 键值对循环，以 End 结束 |
//! | TAG_Int_Array | [`NbtIntArray`] | `Vec<i32>` | 4 字节长度 + 数据（4 字节/元素） |
//! | TAG_Long_Array | [`NbtLongArray`] | `Vec<i64>` | 4 字节长度 + 数据（8 字节/元素） |
//!
//! # 工厂函数
//!
//! 模块底部提供了 `end()`、`byte(data)`、`short(data)` 等便捷构造器函数，
//! 用于快速创建各类型实例。

use std::{
    collections::HashMap,
    io::{Read, Write},
};

use mcml_names::i18_items::error_type::{CoreResult, ErrorData, ErrorType};

use crate::{NbtStream, NbtType, io_error, is_nbt_num};

/// NBT 结束标记（TAG_End）
///
/// 无数据负载。用于标识 Compound 标签中键值对列表的结束。
/// 在 NBT 文件流中不占用任何字节（仅类型序号占用 1 字节）。
pub struct NbtEnd {}

impl Default for NbtEnd {
    fn default() -> Self {
        Self::new()
    }
}

impl NbtEnd {
    /// 创建空的 End 标签
    pub fn new() -> Self {
        Self {}
    }

    /// 判断另一个 NBT 标签是否也是 End 类型
    pub fn eq(&self, nbt: &NbtType) -> bool {
        matches!(nbt, NbtType::End(_))
    }

    /// 将自身转换为 `NbtType` 枚举变体
    pub fn to_nbt(self) -> NbtType {
        NbtType::End(self)
    }
}

/// End 标签无任何数据需要读写
impl NbtStream for NbtEnd {
    fn read<R: Read>(&mut self, _stream: &mut R) -> CoreResult<()> {
        Ok(())
    }

    fn write<W: Write>(&self, _stream: &mut W) -> CoreResult<()> {
        Ok(())
    }
}

/// NBT 字节类型（TAG_Byte）
///
/// 存储一个有符号 8 位整数（`u8`），在二进制流中占用 1 字节。
/// 这是 Minecraft NBT 中最小的数值类型。
pub struct NbtByte {
    /// 字节数据
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
    /// 创建 Byte 标签
    pub fn new(data: u8) -> Self {
        Self { data }
    }

    /// 判断另一个 NBT 标签是否为 Byte 类型且值相等
    pub fn eq(&self, nbt: &NbtType) -> bool {
        match nbt {
            NbtType::Byte(nbt) => nbt.data == self.data,
            _ => false,
        }
    }

    /// 将自身转换为 `NbtType` 枚举变体
    pub fn to_nbt(self) -> NbtType {
        NbtType::Byte(self)
    }
}

impl NbtStream for NbtByte {
    /// 从流中读取 1 字节
    fn read<R: Read>(&mut self, stream: &mut R) -> CoreResult<()> {
        let mut temp = [0u8; 1];
        stream.read_exact(&mut temp).map_err(|err| io_error(err))?;

        self.data = temp[0];

        Ok(())
    }

    /// 将 1 字节写入流
    fn write<W: Write>(&self, stream: &mut W) -> CoreResult<()> {
        let temp = [self.data];
        stream.write_all(&temp).map_err(|err| io_error(err))?;

        Ok(())
    }
}

/// NBT 短整型（TAG_Short）
///
/// 存储一个有符号 16 位整数（`i16`），在二进制流中占用 2 字节，使用大端序。
pub struct NbtShort {
    /// 短整型数据
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
    /// 创建 Short 标签
    pub fn new(data: i16) -> Self {
        Self { data }
    }

    /// 判断另一个 NBT 标签是否为 Short 类型且值相等
    pub fn eq(&self, nbt: &NbtType) -> bool {
        match nbt {
            NbtType::Short(nbt) => nbt.data == self.data,
            _ => false,
        }
    }

    /// 将自身转换为 `NbtType` 枚举变体
    pub fn to_nbt(self) -> NbtType {
        NbtType::Short(self)
    }
}

impl NbtStream for NbtShort {
    /// 从流中读取 2 字节（大端序）
    fn read<R: Read>(&mut self, stream: &mut R) -> CoreResult<()> {
        let mut temp = [0u8; 2];
        stream.read_exact(&mut temp).map_err(|err| io_error(err))?;

        self.data = i16::from_be_bytes(temp);

        Ok(())
    }

    /// 将 2 字节写入流（大端序）
    fn write<W: Write>(&self, stream: &mut W) -> CoreResult<()> {
        let temp = i16::to_be_bytes(self.data);
        stream.write_all(&temp).map_err(|err| io_error(err))?;

        Ok(())
    }
}

/// NBT 整型（TAG_Int）
///
/// 存储一个有符号 32 位整数（`i32`），在二进制流中占用 4 字节，使用大端序。
pub struct NbtInt {
    /// 整型数据
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
    /// 创建 Int 标签
    pub fn new(data: i32) -> Self {
        Self { data }
    }

    /// 判断另一个 NBT 标签是否为 Int 类型且值相等
    pub fn eq(&self, nbt: &NbtType) -> bool {
        match nbt {
            NbtType::Int(nbt) => nbt.data == self.data,
            _ => false,
        }
    }

    /// 将自身转换为 `NbtType` 枚举变体
    pub fn to_nbt(self) -> NbtType {
        NbtType::Int(self)
    }
}

impl NbtStream for NbtInt {
    /// 从流中读取 4 字节（大端序）
    fn read<R: Read>(&mut self, stream: &mut R) -> CoreResult<()> {
        let mut temp = [0u8; 4];
        stream.read_exact(&mut temp).map_err(|err| io_error(err))?;

        self.data = i32::from_be_bytes(temp);

        Ok(())
    }

    /// 将 4 字节写入流（大端序）
    fn write<W: Write>(&self, stream: &mut W) -> CoreResult<()> {
        let temp = i32::to_be_bytes(self.data);
        stream.write_all(&temp).map_err(|err| io_error(err))?;

        Ok(())
    }
}

/// NBT 长整型（TAG_Long）
///
/// 存储一个有符号 64 位整数（`i64`），在二进制流中占用 8 字节，使用大端序。
pub struct NbtLong {
    /// 长整型数据
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
    /// 创建 Long 标签
    pub fn new(data: i64) -> Self {
        Self { data }
    }

    /// 判断另一个 NBT 标签是否为 Long 类型且值相等
    pub fn eq(&self, nbt: &NbtType) -> bool {
        match nbt {
            NbtType::Long(nbt) => nbt.data == self.data,
            _ => false,
        }
    }

    /// 将自身转换为 `NbtType` 枚举变体
    pub fn to_nbt(self) -> NbtType {
        NbtType::Long(self)
    }
}

impl NbtStream for NbtLong {
    /// 从流中读取 8 字节（大端序）
    fn read<R: Read>(&mut self, stream: &mut R) -> CoreResult<()> {
        let mut temp = [0u8; 8];
        stream.read_exact(&mut temp).map_err(|err| io_error(err))?;

        self.data = i64::from_be_bytes(temp);

        Ok(())
    }

    /// 将 8 字节写入流（大端序）
    fn write<W: Write>(&self, stream: &mut W) -> CoreResult<()> {
        let temp = i64::to_be_bytes(self.data);
        stream.write_all(&temp).map_err(|err| io_error(err))?;

        Ok(())
    }
}

/// NBT 浮点型（TAG_Float）
///
/// 存储一个 32 位 IEEE 754 浮点数（`f32`），在二进制流中占用 4 字节，使用大端序。
pub struct NbtFloat {
    /// 浮点数据
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
    /// 创建 Float 标签
    pub fn new(data: f32) -> Self {
        Self { data }
    }

    /// 判断另一个 NBT 标签是否为 Float 类型且值相等
    pub fn eq(&self, nbt: &NbtType) -> bool {
        match nbt {
            NbtType::Float(nbt) => nbt.data == self.data,
            _ => false,
        }
    }

    /// 将自身转换为 `NbtType` 枚举变体
    pub fn to_nbt(self) -> NbtType {
        NbtType::Float(self)
    }
}

impl NbtStream for NbtFloat {
    /// 从流中读取 4 字节（大端序）
    fn read<R: Read>(&mut self, stream: &mut R) -> CoreResult<()> {
        let mut temp = [0u8; 4];
        stream.read_exact(&mut temp).map_err(|err| io_error(err))?;

        self.data = f32::from_be_bytes(temp);

        Ok(())
    }

    /// 将 4 字节写入流（大端序）
    fn write<W: Write>(&self, stream: &mut W) -> CoreResult<()> {
        let temp = f32::to_be_bytes(self.data);
        stream.write_all(&temp).map_err(|err| io_error(err))?;

        Ok(())
    }
}

/// NBT 双精度浮点型（TAG_Double）
///
/// 存储一个 64 位 IEEE 754 浮点数（`f64`），在二进制流中占用 8 字节，使用大端序。
pub struct NbtDouble {
    /// 双精度浮点数据
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
    /// 创建 Double 标签
    pub fn new(data: f64) -> Self {
        Self { data }
    }

    /// 判断另一个 NBT 标签是否为 Double 类型且值相等
    pub fn eq(&self, nbt: &NbtType) -> bool {
        match nbt {
            NbtType::Double(nbt) => nbt.data == self.data,
            _ => false,
        }
    }

    /// 将自身转换为 `NbtType` 枚举变体
    pub fn to_nbt(self) -> NbtType {
        NbtType::Double(self)
    }
}

impl NbtStream for NbtDouble {
    /// 从流中读取 8 字节（大端序）
    fn read<R: Read>(&mut self, stream: &mut R) -> CoreResult<()> {
        let mut temp = [0u8; 8];
        stream.read_exact(&mut temp).map_err(|err| io_error(err))?;

        self.data = f64::from_be_bytes(temp);

        Ok(())
    }

    /// 将 8 字节写入流（大端序）
    fn write<W: Write>(&self, stream: &mut W) -> CoreResult<()> {
        let temp = f64::to_be_bytes(self.data);
        stream.write_all(&temp).map_err(|err| io_error(err))?;

        Ok(())
    }
}

/// NBT 字节数组类型（TAG_Byte_Array）
///
/// 存储一个字节向量（`Vec<u8>`），在二进制流中格式为：
/// 4 字节数组长度（大端序 i32）+ 实际的字节数据。
pub struct NbtByteArray {
    /// 字节数组数据
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
    /// 创建 ByteArray 标签
    pub fn new(data: Vec<u8>) -> Self {
        Self { data }
    }

    /// 判断另一个 NBT 标签是否为 ByteArray 类型且数据相等
    pub fn eq(&self, nbt: &NbtType) -> bool {
        match nbt {
            NbtType::ByteArray(nbt) => nbt.data == self.data,
            _ => false,
        }
    }

    /// 将自身转换为 `NbtType` 枚举变体
    pub fn to_nbt(self) -> NbtType {
        NbtType::ByteArray(self)
    }
}

impl NbtStream for NbtByteArray {
    /// 从流中读取：先读 4 字节长度，再读对应数量的字节数据
    fn read<R: Read>(&mut self, stream: &mut R) -> CoreResult<()> {
        let mut temp = [0u8; 4];
        stream.read_exact(&mut temp).map_err(|err| io_error(err))?;

        let len = i32::from_be_bytes(temp);

        let mut temp = vec![0; len as usize];
        stream.read_exact(&mut temp).map_err(|err| io_error(err))?;

        self.data = temp;

        Ok(())
    }

    /// 写入流：先写 4 字节长度，再写字节数据
    fn write<W: Write>(&self, stream: &mut W) -> CoreResult<()> {
        let temp = i32::to_be_bytes(self.data.len() as i32);
        stream.write_all(&temp).map_err(|err| io_error(err))?;
        stream.write_all(&self.data).map_err(|err| io_error(err))?;

        Ok(())
    }
}

/// NBT 字符串类型（TAG_String）
///
/// 存储一个 UTF-8 编码的字符串（`String`），在二进制流中格式为：
/// 2 字节字符串长度（大端序 i16，字节数）+ UTF-8 字节序列。
/// 注意：长度字段记录的是**字节数**，而非字符数。
pub struct NbtString {
    /// 字符串数据
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
    /// 创建 String 标签
    pub fn new(data: String) -> Self {
        Self { data }
    }

    /// 判断另一个 NBT 标签是否为 String 类型且值相等
    pub fn eq(&self, nbt: &NbtType) -> bool {
        match nbt {
            NbtType::String(nbt) => nbt.data == self.data,
            _ => false,
        }
    }

    /// 将自身转换为 `NbtType` 枚举变体
    pub fn to_nbt(self) -> NbtType {
        NbtType::String(self)
    }
}

impl NbtStream for NbtString {
    /// 从流中读取：先读 2 字节长度，再读对应数量的 UTF-8 字节，解码为字符串
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

    /// 写入流：先写 2 字节长度，再写 UTF-8 字节
    fn write<W: Write>(&self, stream: &mut W) -> CoreResult<()> {
        let temp = i16::to_be_bytes(self.data.len() as i16);
        stream.write_all(&temp).map_err(|err| io_error(err))?;
        stream
            .write_all(&self.data.as_bytes())
            .map_err(|err| io_error(err))?;

        Ok(())
    }
}

/// NBT 列表类型（TAG_List）
///
/// 存储一组相同 NBT 类型的元素列表。所有元素必须是同一 NBT 标签类型。
/// 在二进制流中格式为：1 字节元素类型序号 + 4 字节元素个数（大端序 i32）+
/// 每个元素的 NBT 数据（不含元素自身的类型序号）。
///
/// # 注意
///
/// 不允许混合不同类型的元素。修改列表类型时，会清空已有数据。
pub struct NbtList {
    /// 数据列表
    data: Vec<NbtType>,
    /// 列表元素所允许的 NBT 类型序号
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
    /// 创建指定元素类型的空 List 标签
    ///
    /// # 参数
    ///
    /// - `nbt_num`: 列表元素的 NBT 类型序号（0–12）
    pub fn new(nbt_num: u8) -> Self {
        Self {
            nbt_num,
            data: Vec::new(),
        }
    }

    /// 设置列表的元素类型（通过 NbtType 实例），同时清空已有数据
    pub fn set_type(&mut self, nbt_type: NbtType) {
        self.nbt_num = nbt_type.get_num();
        self.data.clear();
    }

    /// 设置列表的元素类型（通过类型序号），同时清空已有数据
    ///
    /// 如果序号不在合法范围（0–12）内，则忽略此操作。
    pub fn set_num(&mut self, nbt_num: u8) {
        if is_nbt_num(nbt_num) {
            self.nbt_num = nbt_num;
            self.data.clear();
        }
    }

    /// 向列表中添加一个元素
    ///
    /// # 返回值
    ///
    /// 如果元素类型与列表允许的类型一致，则添加成功并返回 `true`；
    /// 否则返回 `false`。
    pub fn add_item(&mut self, nbt: NbtType) -> bool {
        if nbt.get_num() != self.nbt_num {
            false
        } else {
            self.data.push(nbt);

            true
        }
    }

    /// 移除并返回指定索引的元素
    pub fn remove(&mut self, index: usize) -> NbtType {
        self.data.remove(index)
    }

    /// 获取指定索引元素的不可变引用
    pub fn get_item(&self, index: usize) -> Option<&NbtType> {
        self.data.get(index)
    }

    /// 返回列表中的元素数量
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// 返回列表元素的迭代器
    pub fn iter(&self) -> impl Iterator<Item = &NbtType> {
        self.data.iter()
    }

    /// 判断另一个 NBT 标签是否为 List 类型且元素类型、数量和值均相等
    pub fn eq(&self, nbt: &NbtType) -> bool {
        match nbt {
            NbtType::List(nbt) => {
                // 先比较元素类型序号
                if self.nbt_num != nbt.nbt_num {
                    return false;
                }
                // 再比较元素数量
                if self.data.len() != nbt.data.len() {
                    return false;
                }

                // 逐个比较元素值
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

    /// 将自身转换为 `NbtType` 枚举变体
    pub fn to_nbt(self) -> NbtType {
        NbtType::List(self)
    }
}

impl NbtStream for NbtList {
    /// 从流中读取：元素类型序号 → 元素个数 → 每个元素的 NBT 数据
    fn read<R: Read>(&mut self, stream: &mut R) -> CoreResult<()> {
        // 读取元素类型序号
        let mut temp = [0u8; 1];
        stream.read_exact(&mut temp).map_err(|err| io_error(err))?;

        self.nbt_num = temp[0];
        if !is_nbt_num(self.nbt_num) {
            return Err(ErrorType::NbtTypeError);
        }

        // 读取元素个数
        let mut temp = [0u8; 4];
        stream.read_exact(&mut temp).map_err(|err| io_error(err))?;

        let len = i32::from_be_bytes(temp);

        // 逐个读取列表元素
        for _i in 0..len {
            let mut nbt = NbtType::get_nbt(self.nbt_num).unwrap();
            nbt.read(stream)?;
            self.data.push(nbt);
        }

        Ok(())
    }

    /// 写入流：元素类型序号 → 元素个数 → 每个元素的 NBT 数据
    fn write<W: Write>(&self, stream: &mut W) -> CoreResult<()> {
        // 空列表时元素类型序号写 0（TAG_End）
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

/// NBT 复合标签类型（TAG_Compound）
///
/// 存储一组键值对，键为字符串，值为任意类型的 NBT 标签。
/// 在二进制流中格式为：循环读取"类型序号 + 键名 + 数据"，直到遇到
/// TAG_End（类型序号 0）标记结束。这是 Minecraft NBT 中最核心的结构类型，
/// 用于表示区块数据、物品 NBT 标签等复杂嵌套结构。
///
/// # 示例
///
/// ```ignore
/// let mut compound = NbtCompound::new();
/// // 通过调用方手动插入键值对
/// compound.data.insert("Level".to_string(), NbtType::int());
/// ```
pub struct NbtCompound {
    /// 键值对数据，键为字符串，值为 NBT 标签
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
    /// 创建空的 Compound 标签
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    /// 判断另一个 NBT 标签是否为 Compound 类型且所有键值对一致
    ///
    /// 比较逻辑：先检查键的数量是否相同，再逐个键检查值是否调用各自的 `eq` 方法。
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

    /// 获取指定键对应的 NBT 标签的不可变引用
    pub fn get(&self, key: &str) -> Option<&NbtType> {
        self.data.get(key)
    }

    /// 获取指定键对应的 NBT 标签的可变引用
    pub fn get_mut(&mut self, key: &str) -> Option<&mut NbtType> {
        self.data.get_mut(key)
    }

    /// 从 Compound 中提取 `&NbtByteArray`，自动进行类型匹配
    pub fn get_byte_array(&self, key: &str) -> Option<&NbtByteArray> {
        match self.get(key) {
            Some(NbtType::ByteArray(v)) => Some(v),
            _ => None,
        }
    }

    /// 从 Compound 中提取 `&mut NbtByteArray`，自动进行类型匹配
    pub fn get_byte_array_mut(&mut self, key: &str) -> Option<&mut NbtByteArray> {
        match self.get_mut(key) {
            Some(NbtType::ByteArray(v)) => Some(v),
            _ => None,
        }
    }

    /// 从 Compound 中提取 `&NbtLongArray`，自动进行类型匹配
    pub fn get_long_array(&self, key: &str) -> Option<&NbtLongArray> {
        match self.get(key) {
            Some(NbtType::LongArray(v)) => Some(v),
            _ => None,
        }
    }

    /// 从 Compound 中提取 `&mut NbtLongArray`，自动进行类型匹配
    pub fn get_long_array_mut(&mut self, key: &str) -> Option<&mut NbtLongArray> {
        match self.get_mut(key) {
            Some(NbtType::LongArray(v)) => Some(v),
            _ => None,
        }
    }

    /// 从 Compound 中提取 `&NbtCompound`（嵌套 Compound），自动进行类型匹配
    pub fn get_compound(&self, key: &str) -> Option<&NbtCompound> {
        match self.get(key) {
            Some(NbtType::Compound(v)) => Some(v),
            _ => None,
        }
    }

    /// 从 Compound 中提取 `&mut NbtCompound`（嵌套 Compound），自动进行类型匹配
    pub fn get_compound_mut(&mut self, key: &str) -> Option<&mut NbtCompound> {
        match self.get_mut(key) {
            Some(NbtType::Compound(v)) => Some(v),
            _ => None,
        }
    }

    /// 从 Compound 中提取 `i64` 值（TAG_Long 的数据部分）
    pub fn get_long(&self, key: &str) -> Option<i64> {
        match self.get(key) {
            Some(NbtType::Long(v)) => Some(v.data),
            _ => None,
        }
    }

    /// 从 Compound 中提取 `i16` 值（TAG_Short 的数据部分）
    pub fn get_short(&self, key: &str) -> Option<i16> {
        match self.get(key) {
            Some(NbtType::Short(v)) => Some(v.data),
            _ => None,
        }
    }

    /// 从 Compound 中提取 `i32` 值（TAG_Int 的数据部分）
    pub fn get_int(&self, key: &str) -> Option<i32> {
        match self.get(key) {
            Some(NbtType::Int(v)) => Some(v.data),
            _ => None,
        }
    }

    /// 从 Compound 中提取 `u8` 值（TAG_Byte 的数据部分）
    pub fn get_byte(&self, key: &str) -> Option<u8> {
        match self.get(key) {
            Some(NbtType::Byte(v)) => Some(v.data),
            _ => None,
        }
    }

    /// 从 Compound 中提取 `String` 值（TAG_String 的数据部分，克隆返回）
    pub fn get_string(&self, key: &str) -> Option<String> {
        match self.get(key) {
            Some(NbtType::String(v)) => Some(v.data.clone()),
            _ => None,
        }
    }

    /// 从 Compound 中提取 `&NbtList`，自动进行类型匹配
    pub fn get_list(&self, key: &str) -> Option<&NbtList> {
        match self.get(key) {
            Some(NbtType::List(v)) => Some(v),
            _ => None,
        }
    }

    /// 从 Compound 中提取 `&mut NbtList`，自动进行类型匹配
    pub fn get_list_mut(&mut self, key: &str) -> Option<&mut NbtList> {
        match self.get_mut(key) {
            Some(NbtType::List(v)) => Some(v),
            _ => None,
        }
    }

    /// 将自身转换为 `NbtType` 枚举变体
    pub fn to_nbt(self) -> NbtType {
        NbtType::Compound(self)
    }
}

impl NbtStream for NbtCompound {
    /// 从流中逐条读取键值对，直到遇到 TAG_End（类型序号 0）为止
    ///
    /// 每条键值对的格式：1 字节类型序号 → 2 字节键名长度 → 键名字符串 → 值数据
    fn read<R: Read>(&mut self, stream: &mut R) -> CoreResult<()> {
        loop {
            // 读取下一条目的类型序号
            let mut temp = [0u8; 1];
            stream.read_exact(&mut temp).map_err(|err| io_error(err))?;

            // 遇到 TAG_End 则停止读取
            if temp[0] == 0 {
                return Ok(());
            }

            let nbt = NbtType::get_nbt(temp[0]);
            if nbt.is_none() {
                return Err(ErrorType::NbtTypeError);
            }

            // 读取键名长度（大端序 i16）
            let mut temp = [0u8; 2];
            stream.read_exact(&mut temp).map_err(|err| io_error(err))?;

            let len = i16::from_be_bytes(temp);

            // 读取键名字符串（UTF-8 编码）
            let mut temp = vec![0; len as usize];
            stream.read_exact(&mut temp).map_err(|err| io_error(err))?;

            let key = String::from_utf8(temp).map_err(|err| {
                ErrorType::StreamError(ErrorData {
                    error: err.to_string(),
                })
            })?;

            // 读取值数据
            let mut nbt = nbt.unwrap();
            nbt.read(stream)?;

            self.data.insert(key, nbt);
        }
    }

    /// 将全部键值对写入流，末尾附加 TAG_End（0x00）作为终止标记
    fn write<W: Write>(&self, stream: &mut W) -> CoreResult<()> {
        for (key, nbt) in &self.data {
            // 写入类型序号
            let temp = [nbt.get_num()];
            stream.write_all(&temp).map_err(|err| io_error(err))?;

            if !matches!(nbt, NbtType::End(_)) {
                // 写入键名长度和键名
                let temp = i16::to_be_bytes(key.len() as i16);
                stream.write_all(&temp).map_err(|err| io_error(err))?;
                stream
                    .write_all(key.as_bytes())
                    .map_err(|err| io_error(err))?;

                // 写入值数据
                nbt.write(stream)?;
            }
        }

        // 写入 TAG_End 终止标记
        let temp = [0];
        stream.write_all(&temp).map_err(|err| io_error(err))?;

        Ok(())
    }
}

/// NBT 整型数组类型（TAG_Int_Array）
///
/// 存储一个 32 位整数数组（`Vec<i32>`），在二进制流中格式为：
/// 4 字节数组长度（大端序 i32，元素个数）+ 每个元素 4 字节（大端序 i32）。
pub struct NbtIntArray {
    /// 32 位整数数组数据
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
    /// 创建 IntArray 标签
    pub fn new(data: Vec<i32>) -> Self {
        Self { data }
    }

    /// 判断另一个 NBT 标签是否为 IntArray 类型且数据相等
    pub fn eq(&self, nbt: &NbtType) -> bool {
        match nbt {
            NbtType::IntArray(nbt) => nbt.data == self.data,
            _ => false,
        }
    }

    /// 将自身转换为 `NbtType` 枚举变体
    pub fn to_nbt(self) -> NbtType {
        NbtType::IntArray(self)
    }
}

impl NbtStream for NbtIntArray {
    /// 从流中读取：4 字节长度 → 每个元素 4 字节（大端序 i32）
    fn read<R: Read>(&mut self, stream: &mut R) -> CoreResult<()> {
        let mut temp = [0u8; 4];
        stream.read_exact(&mut temp).map_err(|err| io_error(err))?;

        // 长度字段表示元素个数，总字节数 = 元素个数 × 4
        let len = i32::from_be_bytes(temp) * 4;

        let mut temp = vec![0; len as usize];
        stream.read_exact(&mut temp).map_err(|err| io_error(err))?;

        // 每 4 字节解析为一个大端序 i32
        self.data = temp
            .chunks_exact(4)
            .map(|chunk| i32::from_be_bytes(chunk.try_into().unwrap()))
            .collect();

        Ok(())
    }

    /// 写入流：4 字节长度 → 每个元素 4 字节（大端序 i32）
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

/// NBT 长整型数组类型（TAG_Long_Array）
///
/// 存储一个 64 位整数数组（`Vec<i64>`），在二进制流中格式为：
/// 4 字节数组长度（大端序 i32，元素个数）+ 每个元素 8 字节（大端序 i64）。
/// 在 Minecraft 1.12+ 中引入，常用于存储区块的高度图等大数据结构。
pub struct NbtLongArray {
    /// 64 位整数数组数据
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
    /// 创建 LongArray 标签
    pub fn new(data: Vec<i64>) -> Self {
        Self { data }
    }

    /// 判断另一个 NBT 标签是否为 LongArray 类型且数据相等
    pub fn eq(&self, nbt: &NbtType) -> bool {
        match nbt {
            NbtType::LongArray(nbt) => nbt.data == self.data,
            _ => false,
        }
    }

    /// 将自身转换为 `NbtType` 枚举变体
    pub fn to_nbt(self) -> NbtType {
        NbtType::LongArray(self)
    }
}

impl NbtStream for NbtLongArray {
    /// 从流中读取：4 字节长度 → 每个元素 8 字节（大端序 i64）
    fn read<R: Read>(&mut self, stream: &mut R) -> CoreResult<()> {
        let mut temp = [0u8; 4];
        stream.read_exact(&mut temp).map_err(|err| io_error(err))?;

        // 长度字段表示元素个数，总字节数 = 元素个数 × 8
        let len = i32::from_be_bytes(temp) * 8;

        let mut temp = vec![0; len as usize];
        stream.read_exact(&mut temp).map_err(|err| io_error(err))?;

        // 每 8 字节解析为一个大端序 i64
        self.data = temp
            .chunks_exact(8)
            .map(|chunk| i64::from_be_bytes(chunk.try_into().unwrap()))
            .collect();

        Ok(())
    }

    /// 写入流：4 字节长度 → 每个元素 8 字节（大端序 i64）
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

/// 创建空的 NbtEnd 实例
pub fn end() -> NbtEnd {
    NbtEnd::new()
}

/// 创建 NbtByte 实例
pub fn byte(data: u8) -> NbtByte {
    NbtByte::new(data)
}

/// 创建 NbtShort 实例
pub fn short(data: i16) -> NbtShort {
    NbtShort::new(data)
}

/// 创建 NbtInt 实例
pub fn int(data: i32) -> NbtInt {
    NbtInt::new(data)
}

/// 创建 NbtLong 实例
pub fn long(data: i64) -> NbtLong {
    NbtLong::new(data)
}

/// 创建 NbtFloat 实例
pub fn float(data: f32) -> NbtFloat {
    NbtFloat::new(data)
}

/// 创建 NbtDouble 实例
pub fn double(data: f64) -> NbtDouble {
    NbtDouble::new(data)
}

/// 创建 NbtByteArray 实例
pub fn byte_array(data: Vec<u8>) -> NbtByteArray {
    NbtByteArray::new(data)
}

/// 创建 NbtString 实例
///
/// 注意：传入的 `&str` 会被转换为 `String` 后再存储。
pub fn string(data: &str) -> NbtString {
    NbtString::new(String::from(data))
}

/// 创建指定元素类型的空 NbtList 实例
pub fn list(nbt_num: u8) -> NbtList {
    NbtList::new(nbt_num)
}

/// 创建空的 NbtCompound 实例
pub fn compound() -> NbtCompound {
    NbtCompound::new()
}

/// 创建 NbtIntArray 实例
pub fn int_array(data: Vec<i32>) -> NbtIntArray {
    NbtIntArray::new(data)
}

/// 创建 NbtLongArray 实例
pub fn long_array(data: Vec<i64>) -> NbtLongArray {
    NbtLongArray::new(data)
}
