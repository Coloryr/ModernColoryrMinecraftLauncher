/// OAuth网络模型
use serde::{Deserialize, Serialize};

/// OAuth获取登陆码结果
pub struct OAuthGetCodeRes {
    /// 登录码
    pub code: String,
    /// 登录网址
    pub url: String,
    /// 设备码
    pub device_code: String,
    /// 请求间隔
    pub expires_in: i64,
}

/// OAuch请求结果
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct OAuthObj {
    /// 登陆码
    pub user_code: String,
    /// 错误信息
    pub error: Option<String>,
    /// 设备码
    pub device_code: String,
    /// 验证网址
    pub verification_uri: String,
    /// 可用时间
    pub expires_in: i64,
}

impl Default for OAuthObj {
    fn default() -> Self {
        Self {
            user_code: Default::default(),
            error: Default::default(),
            device_code: Default::default(),
            verification_uri: Default::default(),
            expires_in: Default::default(),
        }
    }
}

/// OAuth请求登陆返回
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct OAuthGetCodeObj {
    /// 错误码
    pub error: Option<String>,
    /// 登陆密钥
    pub access_token: String,
    /// 刷新密钥
    pub refresh_token: String,
}

impl Default for OAuthGetCodeObj {
    fn default() -> Self {
        Self {
            error: Default::default(),
            access_token: Default::default(),
            refresh_token: Default::default(),
        }
    }
}
