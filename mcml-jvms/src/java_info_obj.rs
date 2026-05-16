use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

/// 二进制格式
#[derive(Serialize_repr, Deserialize_repr, Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ArchEnum {
    X86,
    X86_64,
    Aarch64,
    Arm,
    Unknow,
}

impl Default for ArchEnum {
    fn default() -> Self {
        ArchEnum::Unknow
    }
}

/// Java信息
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct JavaInfoObj {
    /// 名字
    #[serde(rename = "Name")]
    pub name: String,
    /// 名字
    #[serde(rename = "Path")]
    pub path: String,
    /// 版本号
    #[serde(rename = "Version")]
    pub version: String,
    /// 主版本号
    #[serde(rename = "MajorVersion")]
    pub major_version: i32,
    /// Java类型
    #[serde(rename = "Type")]
    pub java_type: String,
    /// 进制
    #[serde(rename = "Arch")]
    pub arch: ArchEnum,
}

impl Default for JavaInfoObj {
    fn default() -> Self {
        Self {
            name: Default::default(),
            path: Default::default(),
            version: Default::default(),
            major_version: Default::default(),
            java_type: Default::default(),
            arch: Default::default(),
        }
    }
}
