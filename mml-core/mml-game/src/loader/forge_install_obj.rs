//! Forge 安装器 install_profile.json DTO

use serde::{Deserialize, Serialize};

use crate::loader::{LibrariesObj, forge_launch_obj::ForgeLibrariesObj};

/// 新版 Forge 安装配置
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct ForgeInstallObj {
    /// 配置类型（forgeclient / forge server 等）
    pub profile: String,
    /// Forge 版本
    pub version: String,
    /// 对应的 Minecraft 版本
    pub minecraft: String,
    /// 运行库列表
    pub libraries: Vec<ForgeLibrariesObj>,
}

impl Default for ForgeInstallObj {
    fn default() -> Self {
        Self {
            profile: Default::default(),
            version: Default::default(),
            minecraft: Default::default(),
            libraries: Default::default(),
        }
    }
}

/// 旧版 Forge 版本信息
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct VersionInfoObj {
    /// 主类
    #[serde(rename = "mainClass")]
    pub main_class: String,
    /// 游戏启动参数
    #[serde(rename = "minecraftArguments")]
    pub minecraft_arguments: String,
    /// 运行库列表
    pub libraries: Vec<LibrariesObj>,
}

impl Default for VersionInfoObj {
    fn default() -> Self {
        Self {
            main_class: Default::default(),
            minecraft_arguments: Default::default(),
            libraries: Default::default(),
        }
    }
}

/// 旧版 Forge 安装配置
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct ForgeInstallOldObj {
    /// 版本信息
    #[serde(rename = "versionInfo")]
    pub version_info: VersionInfoObj,
}

impl Default for ForgeInstallOldObj {
    fn default() -> Self {
        Self {
            version_info: Default::default(),
        }
    }
}
