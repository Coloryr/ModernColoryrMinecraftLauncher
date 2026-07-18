/// 序列化操作
use std::{collections::{HashMap, hash_map}, io::Read, path::Path};

use mcml_names::i18_items::error_type::{CoreResult, ErrorData, ErrorType};
use serde::{Serialize, de};

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
    pub fn as_str(&self) -> Option<String> {
        self.value.as_str().map(|value| value.to_string())
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
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// 获取可空字符串
    /// - `key`: 需要取出的键
    pub fn get_opt_string(&self, key: &str) -> Option<String> {
        self.map.get(key).and_then(|v| v.as_str())
    }

    /// 获取不可空字符串
    /// - `key`: 需要取出的键
    pub fn get_string(&self, key: &str) -> String {
        self.map
            .get(key)
            .and_then(|v| v.as_str())
            .unwrap_or_default()
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

pub struct MiniTomlObj {
    
}

/// 从json文件序列化
/// - `file`: 需要读取的文件
pub fn json_file<T, P: AsRef<Path>>(file: P) -> CoreResult<T>
where
    T: de::DeserializeOwned,
{
    let temp = path_helper::open_read(&file)?;
    Ok(serde_json::from_reader::<_, T>(temp).map_err(|err| {
        ErrorType::SerializerError(ErrorData {
            error: err.to_string(),
        })
    })?)
}

/// 从bytes中序列化
pub fn json_bytes<T>(data: &[u8]) -> CoreResult<T>
where
    T: de::DeserializeOwned,
{
    Ok(serde_json::from_slice::<T>(&data).map_err(|err| {
        ErrorType::SerializerError(ErrorData {
            error: err.to_string(),
        })
    })?)
}

/// 从流中序列化
pub fn json_stream<T, R: Read>(stream: R) -> CoreResult<T>
where
    T: de::DeserializeOwned,
{
    Ok(serde_json::from_reader::<_, T>(stream).map_err(|err| {
        ErrorType::SerializerError(ErrorData {
            error: err.to_string(),
        })
    })?)
}

/// 写入json到文件中
pub fn json_to_file<T: Serialize, P: AsRef<Path>>(obj: &T, file: P) -> CoreResult<()> {
    let stream = path_helper::open_write(file)?;
    serde_json::to_writer(stream, obj).map_err(|err| {
        ErrorType::SerializerError(ErrorData {
            error: err.to_string(),
        })
    })?;

    Ok(())
}

/// 转换到json字符串
pub fn to_json<T: Serialize>(obj: &T) -> CoreResult<String> {
    serde_json::to_string_pretty(obj).map_err(|err| {
        ErrorType::SerializerError(ErrorData {
            error: err.to_string(),
        })
    })
}
