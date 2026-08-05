use mcml_config::config_obj::{RunArgObj, WindowSettingObj};
use serde::{Deserialize, Serialize};

use crate::{launcher::instance_setting_obj::{AdvanceJvmObj, ServerObj}, loader::LoaderType};

/// 服务器实例
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct ServerPackObj {
    #[serde(rename = "Name")]
    pub name: String,
    /// 游戏版本
    #[serde(rename = "Version")]
    pub version: String,
    /// 加载器类型
    #[serde(rename = "Loader")]
    pub loader: LoaderType,
    /// 加载器版本
    #[serde(rename = "LoaderVersion")]
    pub loader_version: Option<String>,
    /// Jvm参数
    #[serde(rename = "JvmArg")]
    pub run_arg: Option<RunArgObj>,
    /// 窗口设置
    #[serde(rename = "Window")]
    pub window: Option<WindowSettingObj>,
    /// 加入服务器设置
    #[serde(rename = "StartServer")]
    pub start_server: Option<ServerObj>,
    /// 高级Jvm设置
    #[serde(rename = "AdvanceJvm")]
    pub advance_jvm: Option<AdvanceJvmObj>,

    /// 服务器包信息
    #[serde(rename = "Text")]
    pub text: String,
    /// 服务器包版本
    #[serde(rename = "PackVersion")]
    pub pack_version: String,

    pub mod_list: 
}
