//! 游戏版本模型
use serde::{Deserialize, Serialize};

/// 游戏版本信息
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionInfo {
    pub id: String,
    pub version_type: String,
}

impl VersionInfo {
    /// 是否为正式版（release）
    pub fn is_release(&self) -> bool {
        self.version_type == "release"
    }
}
