//! Microsoft OAuth 2.0 数据模型
//!
//! 本模块定义了与 Microsoft 设备授权端点通信时
//! 所需的序列化数据结构。

use serde::{Deserialize, Serialize};

/// OAuth 设备码授权——第一步返回结果
///
/// 包含用户完成浏览器授权所需的信息。
pub struct OAuthGetCodeRes {
    /// 用户需要在浏览器中输入的设备码
    pub code: String,
    /// 用户需要访问的验证网址（如 `https://microsoft.com/link`）
    pub url: String,
    /// 设备码，用于后续轮询令牌
    pub device_code: String,
    /// 设备码的有效期（秒），超时后需重新获取
    pub expires_in: i64,
}

/// OAuth 设备码获取请求的响应
///
/// 来自 Microsoft 设备授权端点的原始 JSON 响应。
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct OAuthObj {
    /// 用户码（显示给用户，用于手动输入）
    pub user_code: String,
    /// 错误信息（授权失败时）
    pub error: Option<String>,
    /// 设备码（用于后续令牌轮询）
    pub device_code: String,
    /// 验证网址（用户需访问的 URL）
    pub verification_uri: String,
    /// 有效时间（秒）
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

/// OAuth 令牌获取请求的响应
///
/// 轮询 Microsoft 令牌端点后返回的结果。
/// 成功时 `access_token` 和 `refresh_token` 非空，`error` 为 `None`。
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct OAuthGetCodeObj {
    /// 错误码：
    /// - `authorization_pending` — 用户尚未完成授权
    /// - `slow_down` — 轮询频率过高
    /// - `expired_token` — 设备码已过期
    pub error: Option<String>,
    /// Microsoft OAuth 访问令牌
    pub access_token: String,
    /// Microsoft OAuth 刷新令牌（用于长期保持登录状态）
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
