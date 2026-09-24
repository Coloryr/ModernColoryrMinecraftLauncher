//! Modrinth 版本 DTO

use serde::{Deserialize, Serialize};

/// Modrinth 项目版本
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct ModrinthVersionObj {
    /// 版本 ID
    pub id: String,
    /// 所属项目 ID
    pub project_id: String,
    /// 版本名称
    pub name: String,
    /// 版本号
    pub version_number: String,
    /// 发布时间
    pub date_published: String,
    /// 下载量
    pub downloads: u64,
    /// 该版本包含的文件列表
    pub files: Vec<ModrinthVersionFileObj>,
    /// 依赖列表
    pub dependencies: Vec<DependencieObj>,
}

impl Default for ModrinthVersionObj {
    fn default() -> Self {
        Self {
            id: Default::default(),
            project_id: Default::default(),
            name: Default::default(),
            version_number: Default::default(),
            date_published: Default::default(),
            downloads: Default::default(),
            files: Default::default(),
            dependencies: Default::default(),
        }
    }
}

/// 版本内的单个文件
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct ModrinthVersionFileObj {
    /// 文件哈希（sha1 / sha512）
    pub hashes: HasheObj,
    /// 下载 URL
    pub url: String,
    /// 文件名
    pub filename: String,
    /// 是否为主要文件
    pub primary: bool,
    /// 文件大小（字节）
    pub size: u64,
}

impl Default for ModrinthVersionFileObj {
    fn default() -> Self {
        Self {
            hashes: Default::default(),
            url: Default::default(),
            filename: Default::default(),
            primary: Default::default(),
            size: Default::default(),
        }
    }
}

/// 文件哈希集合
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct HasheObj {
    /// SHA-1 哈希
    pub sha1: String,
    /// SHA-512 哈希
    pub sha512: String,
}

impl Default for HasheObj {
    fn default() -> Self {
        Self {
            sha1: Default::default(),
            sha512: Default::default(),
        }
    }
}

/// 版本依赖
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct DependencieObj {
    /// 依赖的版本 ID
    pub version_id: Option<String>,
    /// 依赖的项目 ID（内置打包文件（如 Mod Menu Helper.zip）无 project_id，为 null）
    pub project_id: Option<String>,
    /// 依赖类型（required 必选 / optional 可选）
    pub dependency_type: String,
}

impl Default for DependencieObj {
    fn default() -> Self {
        Self {
            version_id: Default::default(),
            project_id: Default::default(),
            dependency_type: Default::default(),
        }
    }
}
