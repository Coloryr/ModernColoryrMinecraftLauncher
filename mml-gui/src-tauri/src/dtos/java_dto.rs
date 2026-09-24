//! Java 运行时 DTO
use serde::{Deserialize, Serialize};

/// Java 运行时信息
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JavaInfoDto {
    /// 显示名
    pub name: String,
    /// 可执行文件路径
    pub path: String,
    /// 完整版本号
    pub version: String,
    /// 主版本号
    pub major: u32,
    /// 发行类型（JDK / JRE）
    pub java_type: String,
    /// 位数（x86 / x64）
    pub arch: String,
}

impl JavaInfoDto {
    /// 主版本号字符串（如 "21"）
    pub fn major_label(&self) -> String {
        self.major.to_string()
    }
}
