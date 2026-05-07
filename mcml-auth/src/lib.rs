use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

pub mod legacy;
pub mod oauth;

/// 账户类型
#[derive(Serialize_repr, Deserialize_repr, Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum AuthType {
    /// 离线账户
    Offline,
    /// 正版登录
    OAuth,
    /// 统一通行证
    Nide8,
    /// 外置登录
    AuthlibInjector,
    /// 皮肤站
    LittleSkin,
    /// 自建皮肤站
    SelfLittleSkin
}

impl Default for AuthType {
    fn default() -> Self {
        AuthType::Offline
    }
}

/// 保存的账户
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct LoginObj {
    #[serde(rename = "UserName")]
    pub user_name: String,
    #[serde(rename = "UUID")]
    pub uuid: String,
    #[serde(rename = "AccessToken")]
    pub access_token: String,
    #[serde(rename = "ClientToken")]
    pub client_token: String,
    #[serde(rename = "AuthType")]
    pub auth_type: AuthType,
    #[serde(rename = "Text1")]
    pub text1: String,
    #[serde(rename = "Text2")]
    pub text2: String,
    #[serde(rename = "LastLogin")]
    pub last_login: DateTime<FixedOffset>,
}

impl Default for LoginObj {
    fn default() -> Self {
        Self {
            user_name: Default::default(),
            uuid: Default::default(),
            access_token: Default::default(),
            client_token: Default::default(),
            auth_type: Default::default(),
            text1: Default::default(),
            text2: Default::default(),
            last_login: Default::default(),
        }
    }
}
