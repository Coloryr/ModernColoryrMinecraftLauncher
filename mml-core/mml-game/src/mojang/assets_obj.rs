//! Minecraft 资源索引 DTO

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// 单个资源条目
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct ObjectsObj {
    /// 资源哈希（sha1）
    pub hash: String,
    /// 资源大小（字节）
    pub size: i64,
}

impl Default for ObjectsObj {
    fn default() -> Self {
        Self {
            hash: Default::default(),
            size: Default::default(),
        }
    }
}

/// 资源索引文件
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct AssetsObj {
    /// 资源表（相对路径 → 资源条目）
    pub objects: HashMap<String, ObjectsObj>,
}

impl Default for AssetsObj {
    fn default() -> Self {
        Self {
            objects: Default::default(),
        }
    }
}
