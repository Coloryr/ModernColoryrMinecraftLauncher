//! Java 运行时模型
use serde::{Deserialize, Serialize};

/// Java 运行时信息
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JavaInfo {
    pub name: String,
    pub path: String,
    pub version: String,
    pub major: u32,
    pub java_type: String,
    pub arch: String,
}

impl JavaInfo {
    /// 主版本号字符串（如 "21"）
    pub fn major_label(&self) -> String {
        self.major.to_string()
    }
}
