/// 序列化操作
use std::{io::Read, path::Path};

use mcml_names::i18_items::error_type::{CoreResult, ErrorData, ErrorType};
use serde::{Serialize, de};

use crate::path_helper;

pub struct MiniJsonObj {
    value: serde_json::Value
}

impl MiniJsonObj {
    pub fn from_str(str) -> CoreResult<Self> {
        let value = serde_json::from_str(s)
    }
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

/// 取出map中的value，组成一个列表
pub fn extract_strings_from_json(
    map: &serde_json::Map<String, serde_json::Value>,
    key: &str,
) -> Vec<String> {
    map.get(key)
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

/// 获取json的字符串
pub fn get_opt_string_from_json(
    map: &serde_json::Map<String, serde_json::Value>,
    key: &str,
) -> Option<String> {
    map.get(key)
        .and_then(|v| Some(v.as_str().unwrap_or("").to_string()))
}

/// 获取json的字符串
pub fn get_string_from_json(map: &serde_json::Map<String, serde_json::Value>, key: &str) -> String {
    map.get(key)
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string()
}
