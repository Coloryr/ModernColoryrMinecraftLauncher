use serde_repr::{Deserialize_repr, Serialize_repr};

pub mod game_setting_obj;
pub mod game_time_obj;
pub mod mod_info_obj;
pub mod custom_game_arg_obj;
pub mod custom_loader_obj;

/// 资源来源
#[derive(Serialize_repr, Deserialize_repr, Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SourceType {
    CurseForge,
    Modrinth,
    McMod,
    ServerPack,
    None,
}

impl Default for SourceType {
    fn default() -> Self {
        SourceType::None
    }
}

/// 编码模式
#[derive(Serialize_repr, Deserialize_repr, Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum LogEncoding {
    UTF8,
    GBK,
}

impl Default for LogEncoding {
    fn default() -> Self {
        LogEncoding::UTF8
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
    /// 自定义
    Custom
}

impl Default for LoaderType {
    fn default() -> Self {
        LoaderType::Normal
    }
}