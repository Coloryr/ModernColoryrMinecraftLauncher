//! 序列化工具模块
//!
//! 提供 JSON 和 TOML 格式的解析、序列化工具函数，
//! 以及简化的值访问封装（`MiniJsonObj`、`MiniTomlMap`）和
//! 自定义反序列化器（`deserialize_number_or_min`/`deserialize_number_or_max`）。
//!
//! # 简化访问封装
//!
//! - [`MiniJsonObj`] / [`MiniJsonMap`] — JSON 值的类型安全访问
//! - [`MiniTomlObj`] / [`MiniTomlMap`] — TOML 值的类型安全访问

/// 序列化操作
use std::{
    cmp,
    collections::{HashMap, hash_map},
    io::Read,
    path::Path,
};

use mcml_names::i18_items::error_type::{CoreResult, ErrorData, ErrorType};
use mcml_sys::path_helper;
use serde::{Serialize, de};
use serde_json::Number;

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
    ///
    /// - `value`: json内容
    fn from_value(value: serde_json::Value) -> Self {
        Self { value }
    }

    /// 从字符串反序列化
    ///
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
    ///
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
}

impl MiniJsonMap {
    /// 取出map中的value，组成一个列表
    ///
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
    ///
    /// - `key`: 需要取出的键
    pub fn get_opt_string(&self, key: &str) -> Option<String> {
        self.map.get(key).and_then(|v| v.as_string())
    }

    /// 获取不可空字符串
    ///
    /// - `key`: 需要取出的键
    pub fn get_string(&self, key: &str) -> String {
        self.map
            .get(key)
            .and_then(|v| v.as_string())
            .unwrap_or_default()
    }

    /// 获取数字
    ///
    /// - `key`: 需要取出的键
    pub fn get_opt_i64(&self, key: &str) -> Option<i64> {
        self.map.get(key).and_then(|item| item.as_i64())
    }

    /// 获取键值对
    ///
    /// - `key`: 需要取出的键
    pub fn get_object(&self, key: &str) -> Option<MiniJsonMap> {
        self.map.get(key).and_then(|item| item.as_object())
    }

    /// 获取列表
    ///
    /// - `key`: 需要取出的键
    pub fn get_list(&self, key: &str) -> Option<Vec<MiniJsonObj>> {
        self.map.get(key).and_then(|item| item.as_list())
    }

    /// 是否存在键
    /// 
    /// - `key`: 需要查找的键
    pub fn have_key(&self, key: &str) -> bool {
        self.map.contains_key(key)
    }

    /// 遍历所有
    pub fn iter(&self) -> hash_map::Iter<'_, String, MiniJsonObj> {
        self.map.iter()
    }
}

impl Default for MiniJsonMap {
    fn default() -> Self {
        Self {
            map: Default::default(),
        }
    }
}

/// 简化Toml的值
pub struct MiniTomlObj {
    value: toml::Value,
}

impl MiniTomlObj {
    /// 从 toml 值创建
    pub fn from_value(value: toml::Value) -> Self {
        Self { value }
    }

    /// 获取可控字符串
    ///
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
    ///
    /// - `key`: 需要取出的键
    pub fn get_list(&self, key: &str) -> Option<Vec<MiniTomlObj>> {
        let value = self.table.get(key)?;
        value.as_list().map(|item| item.into_iter().collect())
    }

    /// 获取键值对
    ///
    /// - `key`: 需要取出的键
    pub fn get_object(&self, key: &str) -> Option<MiniTomlMap> {
        self.table.get(key).and_then(|item| item.as_object())
    }

    /// 获取字符串
    ///
    /// - `key`: 需要取出的键
    pub fn get_opt_string(&self, key: &str) -> Option<String> {
        self.table
            .get(key)
            .and_then(|item| item.value.as_str().map(|item| item.to_string()))
    }

    /// 获取布尔值
    ///
    /// - `key`: 需要取出的键
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
///
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
///
/// - `data`: 输入数据
pub fn json_from_bytes<T: de::DeserializeOwned>(data: &[u8]) -> CoreResult<T> {
    Ok(serde_json::from_slice::<T>(&data).map_err(|err| {
        ErrorType::SerializerError(ErrorData {
            error: err.to_string(),
        })
    })?)
}

/// 从流中序列化
///
/// - `stream`: 输入数据流
pub fn json_from_stream<T: de::DeserializeOwned>(stream: impl Read) -> CoreResult<T> {
    Ok(serde_json::from_reader::<_, T>(stream).map_err(|err| {
        ErrorType::SerializerError(ErrorData {
            error: err.to_string(),
        })
    })?)
}

/// 从字符串读取
///
/// - `str`: 输入字符串
pub fn json_from_str<T: de::DeserializeOwned>(str: &str) -> CoreResult<T> {
    Ok(serde_json::from_str::<T>(str).map_err(|err| {
        ErrorType::SerializerError(ErrorData {
            error: err.to_string(),
        })
    })?)
}

/// 写入json到文件中
///
/// - `obj`: 需要序列化的内容
/// - `file`: 需要写入的文件
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
///
/// - `obj`: 需要序列化的内容
pub fn json_to_string<T: Serialize>(obj: &T) -> CoreResult<String> {
    serde_json::to_string_pretty(obj).map_err(|err| {
        ErrorType::SerializerError(ErrorData {
            error: err.to_string(),
        })
    })
}

/// 转换到bytes
///
/// - `obj`: 需要序列化的内容
pub fn json_to_bytes<T: Serialize>(obj: &T) -> CoreResult<Vec<u8>> {
    serde_json::to_vec(obj).map_err(|err| {
        ErrorType::SerializerError(ErrorData {
            error: err.to_string(),
        })
    })
}

/// 反序列化一个可以是数字或数字数组的 JSON 值。
/// 若是数字：直接返回。
/// 若是数组：返回其中的最小值（空数组返回 0）。
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

/// 反序列化一个可以是数字或数字数组的 JSON 值。
/// 若是数字：直接返回。
/// 若是数组：返回其中的最大值（空数组返回 0）。
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use serde::{Deserialize, Serialize};

    /// MiniJsonObj 类型判断与转换
    #[test]
    fn test_mini_json_obj() {
        let obj = MiniJsonObj::from_str(r#"{"a":1,"b":"x"}"#).unwrap();
        assert!(obj.is_obj());
        assert!(!obj.is_list());
        assert!(!obj.is_str());

        let list = MiniJsonObj::from_str(r#"[1,2,3]"#).unwrap();
        assert!(list.is_list());
        assert!(list.as_list().unwrap().len() == 3);

        let s = MiniJsonObj::from_str(r#""text""#).unwrap();
        assert!(s.is_str());
        assert_eq!(s.as_string().unwrap(), "text");
        assert_eq!(s.as_i64(), None);
    }

    /// MiniJsonObj 解析失败
    #[test]
    fn test_mini_json_obj_invalid() {
        assert!(MiniJsonObj::from_str("{invalid").is_err());
    }

    /// MiniJsonMap 各类取值
    #[test]
    fn test_mini_json_map() {
        let obj = MiniJsonObj::from_str(
            r#"{
                "name": "mcml",
                "count": 5,
                "missing_opt": null,
                "tags": ["a", "b", 3],
                "nested": {"key": "value"},
                "list_of_obj": [{"x": 1}, {"x": 2}]
            }"#,
        )
        .unwrap();
        let map = obj.as_object().unwrap();

        assert_eq!(map.get_string("name"), "mcml");
        // 不存在的键返回默认空串
        assert_eq!(map.get_string("no_such_key"), "");
        assert_eq!(map.get_opt_string("name").unwrap(), "mcml");
        assert_eq!(map.get_opt_string("no_such_key"), None);
        assert_eq!(map.get_opt_i64("count").unwrap(), 5);
        assert_eq!(map.get_opt_i64("no_such_key"), None);
        assert!(map.have_key("name"));
        assert!(!map.have_key("no_such_key"));

        // extract_strings 只保留字符串项
        assert_eq!(map.extract_strings("tags"), vec!["a", "b"]);
        assert_eq!(map.extract_strings("no_such_key"), Vec::<String>::new());

        // 嵌套对象与列表
        let nested = map.get_object("nested").unwrap();
        assert_eq!(nested.get_string("key"), "value");
        let list = map.get_list("list_of_obj").unwrap();
        assert_eq!(list.len(), 2);

        // iter 遍历
        assert_eq!(map.iter().count(), 6);
    }

    /// MiniJsonObj::from_stream
    #[test]
    fn test_mini_json_from_stream() {
        let obj = MiniJsonObj::from_stream(Cursor::new(r#"{"ok":true}"#.as_bytes())).unwrap();
        assert_eq!(
            obj.as_object().unwrap().get_opt_string("ok"),
            None // 是布尔不是字符串
        );
        assert!(MiniJsonObj::from_stream(Cursor::new("bad".as_bytes())).is_err());
    }

    /// MiniTomlObj / MiniTomlMap
    #[test]
    fn test_mini_toml() {
        let toml_str = r#"
title = "demo"
enabled = true
flag_str = "true"
score = 10

[server]
host = "localhost"
port = 25565
"#;
        let map = MiniTomlMap::from_stream(&mut Cursor::new(toml_str.as_bytes())).unwrap();
        assert_eq!(map.get_opt_string("title").unwrap(), "demo");
        // 布尔值与字符串 "true" 都返回真
        assert!(map.get_bool("enabled"));
        assert!(map.get_bool("flag_str"));
        assert!(!map.get_bool("score"));
        assert!(!map.get_bool("no_such_key"));

        let server = map.get_object("server").unwrap();
        assert_eq!(server.get_opt_string("host").unwrap(), "localhost");

        // MiniTomlObj 直接访问
        let obj = MiniTomlObj::from_value(toml::Value::Boolean(true));
        assert!(obj.as_bool().unwrap());
        let obj = MiniTomlObj::from_value(toml::Value::from("str"));
        assert_eq!(obj.as_string().unwrap(), "str");
        assert!(obj.as_bool().is_none());
    }

    /// json 文件读写往返
    #[test]
    fn test_json_file_roundtrip() {
        #[derive(Deserialize, Serialize, PartialEq, Debug)]
        struct Data {
            name: String,
            value: i64,
        }

        let dir = std::env::temp_dir().join(format!(
            "mcml_base_ser_test_{}_{}",
            std::process::id(),
            uuid::Uuid::new_v4().simple()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("data.json");

        let data = Data {
            name: "test".to_string(),
            value: -42,
        };
        json_to_file(&data, &file).unwrap();
        assert_eq!(json_from_file::<Data>(&file).unwrap(), data);

        // bytes / stream / str
        let bytes = json_to_bytes(&data).unwrap();
        assert_eq!(json_from_bytes::<Data>(&bytes).unwrap(), data);
        assert_eq!(
            json_from_stream::<Data>(Cursor::new(bytes.clone())).unwrap(),
            data
        );
        assert_eq!(
            json_from_str::<Data>(&String::from_utf8(bytes).unwrap()).unwrap(),
            data
        );

        // to_string 输出可再次解析
        let s = json_to_string(&data).unwrap();
        assert!(s.contains("\"name\""));

        // 非法 JSON 报错
        assert!(json_from_str::<Data>("{").is_err());

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// deserialize_number_or_min / deserialize_number_or_max
    #[test]
    fn test_deserialize_number_or_min_max() {
        #[derive(Deserialize)]
        struct MinStruct {
            #[serde(deserialize_with = "deserialize_number_or_min")]
            v: i64,
        }
        #[derive(Deserialize)]
        struct MaxStruct {
            #[serde(deserialize_with = "deserialize_number_or_max")]
            v: i64,
        }

        // 数字直接返回
        assert_eq!(serde_json::from_str::<MinStruct>(r#"{"v":5}"#).unwrap().v, 5);
        assert_eq!(serde_json::from_str::<MaxStruct>(r#"{"v":5}"#).unwrap().v, 5);
        // 数组取最小 / 最大
        assert_eq!(serde_json::from_str::<MinStruct>(r#"{"v":[9,3,7]}"#).unwrap().v, 3);
        assert_eq!(serde_json::from_str::<MaxStruct>(r#"{"v":[9,3,7]}"#).unwrap().v, 9);
        // 空数组返回 0
        assert_eq!(serde_json::from_str::<MinStruct>(r#"{"v":[]}"#).unwrap().v, 0);
        assert_eq!(serde_json::from_str::<MaxStruct>(r#"{"v":[]}"#).unwrap().v, 0);
        // 负数
        assert_eq!(serde_json::from_str::<MinStruct>(r#"{"v":[-1,-9]}"#).unwrap().v, -9);
        assert_eq!(serde_json::from_str::<MaxStruct>(r#"{"v":[-1,-9]}"#).unwrap().v, -1);
    }

}
