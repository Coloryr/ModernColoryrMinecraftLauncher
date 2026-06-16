use mcml_names::names;
use serde::{Deserialize, Serialize};

use crate::legacy::selected_profile_obj::SelectedProfileObj;

/// 启动器登陆信息
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct AgentObj {
    /// 启动器名字
    pub name: String,
    /// 启动器版本
    pub version: i32,
}

impl AgentObj {
    /// 创建启动器信息
    /// - `use_minecraft`: 是否为原版头
    pub fn new(use_minecraft: bool) -> Self {
        AgentObj {
            name: String::from(if use_minecraft {
                names::MINECRAFT
            } else {
                names::MCML
            }),
            version: if use_minecraft {
                1
            } else {
                mcml_names::VERSION_NUM
            },
        }
    }
}

impl Default for AgentObj {
    fn default() -> Self {
        Self {
            name: Default::default(),
            version: Default::default(),
        }
    }
}

/// 账户登陆信息
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct AuthenticateObj {
    /// 启动器登陆信息
    pub agent: AgentObj,
    /// 用户名
    pub username: String,
    /// 密码
    pub password: String,
    /// 客户端标识
    #[serde(rename = "clientToken")]
    pub client_token: String,
}

impl Default for AuthenticateObj {
    fn default() -> Self {
        Self {
            agent: Default::default(),
            username: Default::default(),
            password: Default::default(),
            client_token: Default::default(),
        }
    }
}

/// 登陆验证返回
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct AuthenticateResObj {
    /// 登陆密钥
    #[serde(rename = "accessToken")]
    pub access_token: String,
    /// 客户端标识
    #[serde(rename = "clientToken")]
    pub client_token: String,
    /// 选中的账户
    #[serde(rename = "selectedProfile")]
    pub selected_profile: Option<SelectedProfileObj>,
    #[serde(rename = "availableProfiles")]
    pub available_profiles: Option<Vec<SelectedProfileObj>>,
    #[serde(rename = "errorMessage")]
    pub error_message: Option<String>,
}

impl Default for AuthenticateResObj {
    fn default() -> Self {
        Self {
            access_token: Default::default(),
            client_token: Default::default(),
            selected_profile: Default::default(),
            available_profiles: Default::default(),
            error_message: Default::default(),
        }
    }
}
