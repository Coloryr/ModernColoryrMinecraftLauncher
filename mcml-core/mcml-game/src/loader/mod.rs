use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

pub mod custom;
pub mod fabric;
pub mod fabric_loader_obj;
pub mod fabric_meta_obj;
pub mod forge;
pub mod forge_install_obj;
pub mod forge_launch_obj;
pub mod liteloader;
pub mod loader_versions;
pub mod liteloader_meta_obj;
pub mod optifine;
pub mod optifine_obj;
pub mod quilt;
pub mod quilt_loader_obj;
pub mod quilt_meta_obj;

#[derive(Hash, Eq, PartialEq, Debug, Clone)]
pub struct LoaderKey {
    pub mc: String,
    pub version: String,
}

impl LoaderKey {
    pub fn new(mc: &str, version: &str) -> Self {
        LoaderKey {
            mc: String::from(mc),
            version: String::from(version),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct LibrariesObj {
    pub name: String,
    pub url: String,
}

impl Default for LibrariesObj {
    fn default() -> Self {
        Self {
            name: Default::default(),
            url: Default::default(),
        }
    }
}

/// 模组加载器类型
#[derive(Serialize_repr, Deserialize_repr, Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum LoaderType {
    /// 无模组加载器
    Normal,
    /// Forge加载器
    Forge,
    /// Fabric加载器
    Fabric,
    /// Quilt加载器
    Quilt,
    /// NeoForge加载器
    NeoForge,
    /// 高清修复
    OptiFine,
    /// LiteLoader
    LiteLoader,
    /// 自定义
    Custom,
}

impl Default for LoaderType {
    fn default() -> Self {
        LoaderType::Normal
    }
}

impl LoaderType {
    /// 获取加载器版本名前缀
    pub fn prefix(&self) -> &'static str {
        self.id()
    }

    /// 加载器独立 ID（与前端 i18n 键对应，不随语言变化）
    pub fn id(&self) -> &'static str {
        match self {
            LoaderType::Normal => "normal",
            LoaderType::Forge => "forge",
            LoaderType::Fabric => "fabric",
            LoaderType::Quilt => "quilt",
            LoaderType::NeoForge => "neoforge",
            LoaderType::OptiFine => "optifine",
            LoaderType::LiteLoader => "liteloader",
            LoaderType::Custom => "custom",
        }
    }

    /// 按 ID 解析加载器类型
    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "normal" => Some(LoaderType::Normal),
            "forge" => Some(LoaderType::Forge),
            "fabric" => Some(LoaderType::Fabric),
            "quilt" => Some(LoaderType::Quilt),
            "neoforge" => Some(LoaderType::NeoForge),
            "optifine" => Some(LoaderType::OptiFine),
            "liteloader" => Some(LoaderType::LiteLoader),
            "custom" => Some(LoaderType::Custom),
            _ => None,
        }
    }

    /// 全部加载器 ID（下拉列表数据源）
    pub fn ids() -> Vec<&'static str> {
        vec![
            "normal",
            "forge",
            "fabric",
            "quilt",
            "neoforge",
            "optifine",
            "liteloader",
            "custom",
        ]
    }
}
