//! 游戏版本 DTO
use serde::{Deserialize, Serialize};

/// 游戏版本信息
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionInfoDto {
    pub id: String,
    pub version_type: String,
}

impl VersionInfoDto {
    /// 是否为正式版（release）
    pub fn is_release(&self) -> bool {
        self.version_type == "release"
    }
}
