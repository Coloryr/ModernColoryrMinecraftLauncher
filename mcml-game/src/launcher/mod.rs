use serde_repr::{Deserialize_repr, Serialize_repr};

pub mod instance_setting_obj;
pub mod game_time_obj;
pub mod file_online_info_obj;
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
