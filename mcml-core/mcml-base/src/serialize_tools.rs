/// 序列化操作
use std::{
    cmp, collections::{HashMap, hash_map}, io::Read, path::Path,
};

use mcml_names::i18_items::error_type::{CoreResult, ErrorData, ErrorType};
use serde::{Serialize, de};
use serde_json::Number;

use crate::path_helper;

/// 简化json处理
pub struct MiniJsonObj {
    value: serde_json::Value,
}

/// 简化键值对
pub struct MiniJsonMap {
    map: HashMap<String, MiniJsonObj>,
}

impl MiniJsonObj {
    /// 从json内容中创建
    /// - `value`: json内容
    fn from_value(value: serde_json::Value) -> Self {
        Self { value }
    }

    /// 从字符串反序列化
    /// - `str`: 需要反序列化的内容
    pub fn from_str(str: &str) -> CoreResult<Self> {
        let value = serde_json::from_str::<serde_json::Value>(str).map_err(|err| {
            ErrorType::SerializerError(ErrorData {
                error: err.to_string(),
            })
        })?;

        Ok(Self { value })
    }

    /// 从流中 反序列化
    /// - `str`: 需要反序列化的内容
    pub fn from_stream<R: Read>(stream: R) -> CoreResult<Self> {
        let value = serde_json::from_reader::<_, serde_json::Value>(stream).map_err(|err| {
            ErrorType::SerializerError(ErrorData {
                error: err.to_string(),
            })
        })?;

        Ok(Self { value })
    }

    /// 是否为列表
    pub fn is_list(&self) -> bool {
        self.value.is_array()
    }

    /// 是否为键值对
    pub fn is_obj(&self) -> bool {
        self.value.is_object()
    }

    /// 是否为字符串
    pub fn is_str(&self) -> bool {
        self.value.is_string()
    }

    /// 转换成列表
    pub fn as_list(&self) -> Option<Vec<MiniJsonObj>> {
        match &self.value {
            serde_json::Value::Array(arr) => Some(
                arr.into_iter()
                    .map(|item| MiniJsonObj::from_value(item.clone()))
                    .collect(),
            ),
            _ => None,
        }
    }

    /// 转换为键值对
    pub fn as_object(&self) -> Option<MiniJsonMap> {
        match &self.value {
            serde_json::Value::Object(map) => Some(MiniJsonMap {
                map: map
                    .into_iter()
                    .map(|(key, value)| (key.clone(), MiniJsonObj::from_value(value.clone())))
                    .collect(),
            }),
            _ => None,
        }
    }

    /// 转换为字符串
    pub fn as_string(&self) -> Option<String> {
        self.value.as_str().map(|value| value.to_string())
    }

    /// 转换为数字
    pub fn as_i64(&self) -> Option<i64> {
        self.value.as_i64()
    }

    /// 从键中获取列表
    /// - `key`: 键名
    pub fn get_list(&self, key: &str) -> Option<Vec<MiniJsonObj>> {
        self.value
            .as_object()
            .and_then(|data| data.get(key))
            .and_then(|item| item.as_array())
            .map(|item| {
                item.iter()
                    .map(|item| MiniJsonObj::from_value(item.clone()))
                    .collect()
            })
    }
}

impl MiniJsonMap {
    /// 取出map中的value，组成一个列表
    /// - `str`: 需要取出的键
    pub fn extract_strings(&self, key: &str) -> Vec<String> {
        self.map
            .get(key)
            .and_then(|v| v.as_list())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_string().map(String::from))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// 获取可空字符串
    /// - `key`: 需要取出的键
    pub fn get_opt_string(&self, key: &str) -> Option<String> {
        self.map.get(key).and_then(|v| v.as_string())
    }

    /// 获取不可空字符串
    /// - `key`: 需要取出的键
    pub fn get_string(&self, key: &str) -> String {
        self.map
            .get(key)
            .and_then(|v| v.as_string())
            .unwrap_or_default()
    }

    /// 获取数字
    /// - `key`: 需要取出的键
    pub fn get_opt_i64(&self, key: &str) -> Option<i64> {
        self.map.get(key).and_then(|item| item.as_i64())
    }

    /// 获取键值对
    /// - `key`: 需要取出的键
    pub fn get_object(&self, key: &str) -> Option<MiniJsonMap> {
        self.map.get(key).and_then(|item| item.as_object())
    }

    /// 获取列表
    /// - `key`: 需要取出的键
    pub fn get_list(&self, key: &str) -> Option<Vec<MiniJsonObj>> {
        self.map.get(key).and_then(|item| item.as_list())
    }

    /// 遍历所有
    pub fn iter(&self) -> hash_map::Iter<'_, String, MiniJsonObj> {
        self.map.iter()
    }
}

/// 简化Toml的值
pub struct MiniTomlObj {
    value: toml::Value,
}

impl MiniTomlObj {
    pub fn from_value(value: toml::Value) -> Self {
        Self { value }
    }

    /// 获取可控字符串
    /// - `key`: 需要取出的键
    pub fn get_opt_string(&self, key: &str) -> Option<String> {
        self.value
            .as_table()
            .and_then(|item| item.get(key))
            .and_then(|item| item.as_str().map(|item| item.to_string()))
    }

    /// 转为列表
    pub fn as_list(&self) -> Option<Vec<MiniTomlObj>> {
        match &self.value {
            toml::Value::Array(list) => Some(
                list.into_iter()
                    .map(|item| MiniTomlObj::from_value(item.clone()))
                    .collect(),
            ),
            _ => None,
        }
    }

    /// 转为键值对
    pub fn as_object(&self) -> Option<MiniTomlMap> {
        match &self.value {
            toml::Value::Table(table) => Some(MiniTomlMap::from_table(table.clone())),
            _ => None,
        }
    }

    /// 转为布尔
    pub fn as_bool(&self) -> Option<bool> {
        match &self.value {
            toml::Value::Boolean(bool) => Some(*bool),
            _ => None,
        }
    }

    /// 转为字符串
    pub fn as_string(&self) -> Option<String> {
        match &self.value {
            toml::Value::String(str) => Some(str.to_string()),
            _ => None,
        }
    }
}

/// 简化Toml
pub struct MiniTomlMap {
    table: HashMap<String, MiniTomlObj>,
}

impl MiniTomlMap {
    /// 从键值对中创建
    pub fn from_table(table: toml::Table) -> Self {
        MiniTomlMap {
            table: table
                .into_iter()
                .map(|(key, value)| (key.clone(), MiniTomlObj::from_value(value.clone())))
                .collect(),
        }
    }

    /// 从流中读取
    pub fn from_stream<R: Read>(stream: &mut R) -> CoreResult<Self> {
        let mut toml = String::new();
        stream.read_to_string(&mut toml).map_err(|err| {
            ErrorType::ArchiveReadError(ErrorData {
                error: err.to_string(),
            })
        })?;

        let obj = toml::from_str::<toml::Table>(&toml).map_err(|err| {
            ErrorType::SerializerError(ErrorData {
                error: err.to_string(),
            })
        })?;

        Ok(Self::from_table(obj))
    }

    /// 获取队列
    /// - `key`: 需要取出的键
    pub fn get_list(&self, key: &str) -> Option<Vec<MiniTomlObj>> {
        let value = self.table.get(key)?;
        value.as_list().map(|item| item.into_iter().collect())
    }

    /// 获取键值对
    /// - `key`: 需要取出的键
    pub fn get_object(&self, key: &str) -> Option<MiniTomlMap> {
        self.table.get(key).and_then(|item| item.as_object())
    }

    /// 获取字符串
    /// - `key`: 需要取出的键
    pub fn get_opt_string(&self, key: &str) -> Option<String> {
        self.table
            .get(key)
            .and_then(|item| item.value.as_str().map(|item| item.to_string()))
    }

    pub fn get_bool(&self, key: &str) -> bool {
        self.table
            .get(key)
            .and_then(|v| v.as_bool())
            .or_else(|| {
                self.table
                    .get(key)
                    .and_then(|v| v.as_string())
                    .map(|s| s.eq_ignore_ascii_case("true"))
            })
            .unwrap_or(false)
    }

    /// 遍历
    pub fn iter(&self) -> hash_map::Iter<'_, String, MiniTomlObj> {
        self.table.iter()
    }
}

/// 从json文件序列化
/// - `file`: 需要读取的文件
pub fn json_from_file<T: de::DeserializeOwned>(file: impl AsRef<Path>) -> CoreResult<T> {
    let temp = path_helper::open_read(&file)?;
    Ok(serde_json::from_reader::<_, T>(temp).map_err(|err| {
        ErrorType::SerializerError(ErrorData {
            error: err.to_string(),
        })
    })?)
}

/// 从bytes中序列化
pub fn json_from_bytes<T: de::DeserializeOwned>(data: &[u8]) -> CoreResult<T> {
    Ok(serde_json::from_slice::<T>(&data).map_err(|err| {
        ErrorType::SerializerError(ErrorData {
            error: err.to_string(),
        })
    })?)
}

/// 从流中序列化
pub fn json_from_stream<T: de::DeserializeOwned>(stream: impl Read) -> CoreResult<T> {
    Ok(serde_json::from_reader::<_, T>(stream).map_err(|err| {
        ErrorType::SerializerError(ErrorData {
            error: err.to_string(),
        })
    })?)
}

/// 从字符串读取
pub fn json_from_str<T: de::DeserializeOwned>(str: &str) -> CoreResult<T> {
    Ok(serde_json::from_str::<T>(str).map_err(|err| {
        ErrorType::SerializerError(ErrorData {
            error: err.to_string(),
        })
    })?)
}

/// 写入json到文件中
pub fn json_to_file<T: Serialize>(obj: &T, file: impl AsRef<Path>) -> CoreResult<()> {
    let stream = path_helper::open_write(file)?;
    serde_json::to_writer(stream, obj).map_err(|err| {
        ErrorType::SerializerError(ErrorData {
            error: err.to_string(),
        })
    })?;

    Ok(())
}

/// 转换到json字符串
pub fn json_to_string<T: Serialize>(obj: &T) -> CoreResult<String> {
    serde_json::to_string_pretty(obj).map_err(|err| {
        ErrorType::SerializerError(ErrorData {
            error: err.to_string(),
        })
    })
}

/// 转换到bytes
pub fn json_to_bytes<T: Serialize>(obj: &T) -> CoreResult<Vec<u8>> {
    serde_json::to_vec(obj).map_err(|err| {
        ErrorType::SerializerError(ErrorData {
            error: err.to_string(),
        })
    })
}


/// Deserialize a JSON value that can be either a number or an array of numbers.
/// For a number: returns it directly.
/// For an array: returns the minimum value (0 for empty array).
pub fn deserialize_number_or_min<'de, D>(deserializer: D) -> Result<i64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::{self, SeqAccess, Visitor};
    use std::fmt;

    struct V;
    impl<'de> Visitor<'de> for V {
        type Value = i64;
        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_str("a number or array of numbers")
        }
        fn visit_i64<E: de::Error>(self, v: i64) -> Result<i64, E> {
            Ok(v)
        }
        fn visit_u64<E: de::Error>(self, v: u64) -> Result<i64, E> {
            Ok(v as i64)
        }
        fn visit_f64<E: de::Error>(self, v: f64) -> Result<i64, E> {
            Ok(v as i64)
        }
        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<i64, A::Error> {
            let mut min = i64::MAX;
            let mut found = false;
            while let Some(v) = seq.next_element::<Number>()? {
                if let Some(n) = v.as_i64() {
                    min = cmp::min(min, n);
                    found = true;
                }
            }
            Ok(if found { min } else { 0 })
        }
    }
    deserializer.deserialize_any(V)
}

/// Deserialize a JSON value that can be either a number or an array of numbers.
/// For a number: returns it directly.
/// For an array: returns the maximum value (0 for empty array).
pub fn deserialize_number_or_max<'de, D>(deserializer: D) -> Result<i64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::{self, SeqAccess, Visitor};
    use std::fmt;

    struct V;
    impl<'de> Visitor<'de> for V {
        type Value = i64;
        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_str("a number or array of numbers")
        }
        fn visit_i64<E: de::Error>(self, v: i64) -> Result<i64, E> {
            Ok(v)
        }
        fn visit_u64<E: de::Error>(self, v: u64) -> Result<i64, E> {
            Ok(v as i64)
        }
        fn visit_f64<E: de::Error>(self, v: f64) -> Result<i64, E> {
            Ok(v as i64)
        }
        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<i64, A::Error> {
            let mut max = i64::MIN;
            let mut found = false;
            while let Some(v) = seq.next_element::<Number>()? {
                if let Some(n) = v.as_i64() {
                    max = cmp::max(max, n);
                    found = true;
                }
            }
            Ok(if found { max } else { 0 })
        }
    }
    deserializer.deserialize_any(V)
}
