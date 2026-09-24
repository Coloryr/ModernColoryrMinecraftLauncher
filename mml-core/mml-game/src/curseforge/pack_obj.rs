//! CurseForge 整合包清单（manifest.json）DTO

use serde::{Deserialize, Serialize};

/// CurseForge整合包数据
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct CurseForgePackObj {
    /// Minecraft 配置
    pub minecraft: MinecraftObj,
    /// 清单类型（minecraftModpack）
    #[serde(rename = "manifestType")]
    pub manifest_type: String,
    /// 清单版本号
    #[serde(rename = "manifestVersion")]
    pub manifest_version: i32,
    /// 整合包名称
    pub name: String,
    /// 整合包版本
    pub version: String,
    /// 作者
    pub author: String,
    /// 文件列表
    pub files: Vec<FilesObj>,
    /// 覆盖包目录名
    pub overrides: String,
}

impl Default for CurseForgePackObj {
    fn default() -> Self {
        Self {
            minecraft: Default::default(),
            manifest_type: Default::default(),
            manifest_version: Default::default(),
            name: Default::default(),
            version: Default::default(),
            author: Default::default(),
            files: Default::default(),
            overrides: Default::default(),
        }
    }
}

/// 整合包的 Minecraft 配置
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct MinecraftObj {
    /// 游戏版本
    pub version: String,
    /// 加载器列表
    #[serde(rename = "modLoaders")]
    pub mod_loaders: Vec<ModLoadersObj>,
}

impl Default for MinecraftObj {
    fn default() -> Self {
        Self {
            version: Default::default(),
            mod_loaders: Default::default(),
        }
    }
}

/// 单个模组加载器
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct ModLoadersObj {
    /// 加载器 ID（如 `forge-14.23.5.2859`）
    pub id: String,
    /// 是否为主加载器
    pub primary: bool,
}

impl Default for ModLoadersObj {
    fn default() -> Self {
        Self {
            id: Default::default(),
            primary: Default::default(),
        }
    }
}

/// 整合包内的单个文件
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct FilesObj {
    /// 项目 ID
    #[serde(rename = "projectID")]
    pub project_id: u64,
    /// 文件 ID
    #[serde(rename = "fileID")]
    pub file_id: u64,
    /// 是否必选
    pub required: bool,
}

impl Default for FilesObj {
    fn default() -> Self {
        Self {
            project_id: Default::default(),
            file_id: Default::default(),
            required: Default::default(),
        }
    }
}
