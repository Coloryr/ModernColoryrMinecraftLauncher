//! HMCL 配置（hmclversion.cfg / server.json）DTO

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// HMCL 实例信息
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct HMCLObj {
    /// 实例名
    pub name: String,
    /// 组件列表（游戏版本 / 加载器版本）
    pub addons: Vec<AddonsObj>,
    /// 启动设置
    #[serde(rename = "launchInfo")]
    pub launch_info: Option<LaunchInfoObj>,
}

impl Default for HMCLObj {
    fn default() -> Self {
        Self {
            name: Default::default(),
            addons: Default::default(),
            launch_info: Default::default(),
        }
    }
}

/// 组件信息
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct AddonsObj {
    /// 组件 ID
    pub id: String,
    /// 组件版本
    pub version: String,
}

impl Default for AddonsObj {
    fn default() -> Self {
        Self {
            id: Default::default(),
            version: Default::default(),
        }
    }
}

/// HMCL 启动设置
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct LaunchInfoObj {
    /// 最小内存（MB）
    #[serde(rename = "minMemory")]
    pub min_memory: Option<u32>,
    /// 最大内存（MB）
    #[serde(rename = "maxMemory")]
    pub max_memory: Option<u32>,
    /// 窗口宽度
    #[serde(rename = "width")]
    pub width: Option<u16>,
    /// 窗口高度
    #[serde(rename = "height")]
    pub height: Option<u16>,
    /// 是否全屏
    #[serde(rename = "fullscreen")]
    pub fullscreen: Option<bool>,
    /// 环境变量
    #[serde(rename = "environmentVariables")]
    pub environment_variables: Option<HashMap<String, String>>,
    /// 游戏参数
    #[serde(rename = "launchArgument")]
    pub launch_argument: Option<Vec<String>>,
    /// JVM 参数
    #[serde(rename = "javaArgument")]
    pub java_argument: Option<Vec<String>>,
    /// Quick Play 设置
    #[serde(rename = "quickPlayOption")]
    pub quick_play_option: Option<QuickPlayOptionObj>,
    /// 启动前执行的命令
    #[serde(rename = "preLaunchCommand")]
    pub pre_launch_command: Option<String>,
    /// 退出后执行的命令
    #[serde(rename = "postExitCommand")]
    pub post_exit_command: Option<String>,
}

impl Default for LaunchInfoObj {
    fn default() -> Self {
        Self {
            min_memory: Default::default(),
            max_memory: Default::default(),
            width: Default::default(),
            height: Default::default(),
            fullscreen: Default::default(),
            launch_argument: Default::default(),
            java_argument: Default::default(),
            quick_play_option: Default::default(),
            pre_launch_command: Default::default(),
            post_exit_command: Default::default(),
            environment_variables: Default::default(),
        }
    }
}

/// Quick Play 设置（暂未使用）
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct QuickPlayOptionObj {}

impl Default for QuickPlayOptionObj {
    fn default() -> Self {
        Self {}
    }
}

/// HMCL 服务器整合包信息
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct HMCLServerObj {
    /// 服务器名
    pub name: String,
    /// 作者
    pub author: String,
    /// 版本
    pub version: String,
    /// 描述
    pub description: String,
    /// 文件下载源地址
    pub file_api: String,
    /// 需要同步的文件列表
    pub files: Vec<HMCLServerFileObj>,
    /// 组件列表
    pub addons: Vec<AddonsObj>,
}

impl Default for HMCLServerObj {
    fn default() -> Self {
        Self {
            name: Default::default(),
            author: Default::default(),
            version: Default::default(),
            description: Default::default(),
            file_api: Default::default(),
            files: Default::default(),
            addons: Default::default(),
        }
    }
}

/// 服务器整合包内的单个文件
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct HMCLServerFileObj {
    /// 相对游戏目录的路径
    pub path: String,
    /// 文件哈希
    pub hash: String,
}

impl Default for HMCLServerFileObj {
    fn default() -> Self {
        Self {
            path: Default::default(),
            hash: Default::default(),
        }
    }
}
