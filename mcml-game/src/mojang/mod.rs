use serde_repr::{Deserialize_repr, Serialize_repr};

pub mod assets_obj;
pub mod game_arg_obj;
pub mod mojang_api;
pub mod version_checker;
pub mod version_obj;
pub mod version_parse;

/// 游戏版本类型
#[derive(Serialize_repr, Deserialize_repr, Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum GameType {
    /// 发布版
    Release,
    Snapshot,
    Other,
    All,
}

impl Default for GameType {
    fn default() -> Self {
        GameType::Release
    }
}
