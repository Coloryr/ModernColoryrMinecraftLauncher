//! Modrinth 整合包索引（.mrpack）DTO

use std::collections::HashMap;

use mml_net::modrinth_api::version_obj::HasheObj;
use serde::{Deserialize, Serialize};

use crate::launcher::project_save_obj::MmlProjectSaveObj;

/// Modrinth整合包数据
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct ModrinthPackObj {
    /// 索引格式版本
    #[serde(rename = "formatVersion")]
    pub format_version: i32,
    /// 整合包版本 ID
    #[serde(rename = "versionId")]
    pub version_id: String,
    /// 整合包名称
    pub name: String,
    /// 整合包简介
    pub summary: String,
    /// 需要下载的文件列表
    pub files: Vec<ModrinthPackFileObj>,
    /// 依赖（`minecraft` / `fabric-loader` 等 → 版本号）
    pub dependencies: HashMap<String, String>,
}

impl Default for ModrinthPackObj {
    fn default() -> Self {
        Self {
            format_version: Default::default(),
            version_id: Default::default(),
            name: Default::default(),
            summary: Default::default(),
            files: Default::default(),
            dependencies: Default::default(),
        }
    }
}

/// 整合包内的单个文件
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct ModrinthPackFileObj {
    /// 相对游戏目录的文件路径
    pub path: String,
    /// 文件哈希
    pub hashes: HasheObj,
    /// 下载地址列表
    pub downloads: Vec<String>,
    /// 文件大小（字节）
    #[serde(rename = "fileSize")]
    pub file_size: u64,
    /// 来源项目信息（私有扩展字段，仅本启动器写入）
    #[serde(rename = "_private_data", alias = "_colormc")]
    pub project: Option<MmlProjectSaveObj>,
}

impl Default for ModrinthPackFileObj {
    fn default() -> Self {
        Self {
            path: Default::default(),
            hashes: Default::default(),
            downloads: Default::default(),
            file_size: Default::default(),
            project: Default::default(),
        }
    }
}
