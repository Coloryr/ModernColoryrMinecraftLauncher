/// 游戏账户
use chrono::{DateTime, FixedOffset, Local};
use mcml_names::i18_items::error_type::CoreResult;
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use tokio_util::sync::CancellationToken;

pub mod auths;
pub mod legacy;
pub mod oauth;

/// 账户类型
#[derive(Serialize_repr, Deserialize_repr, Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
    SelfLittleSkin,
}

impl Default for AuthType {
    fn default() -> Self {
        AuthType::Offline
    }
}

/// 保存的账户
#[derive(Serialize, Deserialize, Debug, Clone)]
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
    pub text1: Option<String>,
    #[serde(rename = "Text2")]
    pub text2: Option<String>,
    #[serde(rename = "LastLogin")]
    pub last_login: DateTime<FixedOffset>,
}

impl LoginObj {
    /// 创建账户
    /// - `user_name`: 名字
    /// - `uuid`: 标识
    /// - `access_token`: 密钥
    /// - `client_token`: 客户端标识
    pub fn new(
        user_name: String,
        uuid: String,
        access_token: String,
        client_token: String,
    ) -> Self {
        let dt = Local::now();
        let dt_new: DateTime<FixedOffset> = dt.fixed_offset();

        Self {
            user_name,
            uuid,
            access_token,
            client_token,
            auth_type: Default::default(),
            text1: Default::default(),
            text2: Default::default(),
            last_login: dt_new,
        }
    }

    /// 创建空白账户
    /// - `user_name`: 名字
    /// - `uuid`: 标识
    pub fn new_empty(user_name: String, uuid: String) -> Self {
        Self {
            user_name,
            uuid,
            access_token: Default::default(),
            client_token: Default::default(),
            auth_type: Default::default(),
            text1: Default::default(),
            text2: Default::default(),
            last_login: Default::default(),
        }
    }

    /// 创建空白账户
    /// - `access_token`: 密钥
    /// - `client_token`: 客户端标识
    pub fn new_token(access_token: String, client_token: String) -> Self {
        Self {
            user_name: Default::default(),
            uuid: Default::default(),
            access_token,
            client_token,
            auth_type: Default::default(),
            text1: Default::default(),
            text2: Default::default(),
            last_login: Default::default(),
        }
    }

    /// 获取账户键
    pub fn get_key(&self) -> UserKeyObj {
        UserKeyObj {
            uuid: self.uuid.clone(),
            auth_type: self.auth_type.clone(),
        }
    }

    /// 更新登陆信息
    pub async fn refresh(&mut self, cancel: &CancellationToken) -> CoreResult<()> {
        match &self.auth_type {
            AuthType::OAuth => self.refresh_oauth(cancel).await,
            AuthType::Nide8 => self.refresh_nide8(cancel).await,
            AuthType::AuthlibInjector => self.refresh_authlib(cancel).await,
            AuthType::LittleSkin | AuthType::SelfLittleSkin => {
                self.refresh_littleskin(cancel).await
            }
            _ => Ok(()),
        }
    }
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

#[derive(Eq, Hash, PartialEq, Debug)]
pub struct UserKeyObj {
    /// 账户标识
    pub uuid: String,
    /// 账户类型
    pub auth_type: AuthType,
}
