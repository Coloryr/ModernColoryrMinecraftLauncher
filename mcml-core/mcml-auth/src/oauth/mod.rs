//! Microsoft OAuth 2.0 认证模块
//!
//! 本模块实现了 Microsoft 正版 Minecraft 的完整 OAuth 2.0 认证流程。
//! 由于 Minecraft Java 版已迁移至微软账户体系，登录需要经过以下认证链：
//!
//! # 认证流程
//!
//! ```text
//! Microsoft OAuth 设备码授权
//!     │
//!     ▼
//! 获取 Microsoft Token ───► refresh_token 可用于续期
//!     │
//!     ▼
//! Xbox Live 认证（获取 XBL Token）
//!     │
//!     ▼
//! XSTS 认证（获取 XSTS Token）
//!     │
//!     ▼
//! Minecraft 服务认证（获取 Minecraft Token）
//!     │
//!     ▼
//! 获取 Minecraft 玩家 Profile（用户名 + UUID）
//! ```
//!
//! # 子模块
//!
//! - [`oauth_res`] — OAuth 请求/响应的数据结构
//! - [`xbox_obj`] — Xbox Live/XSTS 认证的数据结构
//!
//! # 认证状态
//!
//! [`AuthState`] 枚举表示了认证流程中的各个阶段：
//! `OAuth` → `XBox` → `XSTS` → `Token` → `Profile`

use std::{sync::OnceLock, time::Duration};

use chrono::Local;
use mcml_names::i18_items::error_type::{CoreResult, ErrorData, ErrorType};
use mcml_net::{mojang_api, urls};
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;

use crate::{
    LoginObj,
    oauth::{
        oauth_res::{OAuthGetCodeObj, OAuthGetCodeRes, OAuthObj},
        xbox_obj::{
            XBoxLiveRes, XBoxLoginObj, XBoxLoginPropertiesObj, XBoxLoginResObj, XSTSLoginObj,
            XSTSLoginPropertiesObj,
        },
    },
};

/// OAuth 请求/响应数据结构
pub mod oauth_res;
/// Xbox Live/XSTS 认证数据结构
pub mod xbox_obj;

/// OAuth 客户端密钥（Azure 应用程序 ID）
///
/// 全局单例，通过 `set_key()` 在启动时设置。
pub static KEY: OnceLock<String> = OnceLock::new();

/// 微软认证流程中的当前阶段
///
/// 用于在 UI 中展示认证进度。
pub enum AuthState {
    /// 正在进行 Microsoft OAuth 设备码授权
    OAuth,
    /// 正在进行 Xbox Live 认证
    XBox,
    /// 正在进行 XSTS（Xbox Secure Token Service）认证
    XSTS,
    /// 正在获取 Minecraft 服务令牌
    Token,
    /// 正在获取 Minecraft 玩家档案
    Profile,
}

/// 设置 OAuth 客户端密钥
///
/// 应在程序启动时调用，设置 Azure 应用程序的客户端 ID。
///
/// # 参数
///
/// - `key`: Azure 应用程序注册 ID
pub fn set_key(key: &str) {
    KEY.get_or_init(|| key.to_string());
}

/// 获取已设置的 OAuth 客户端密钥
///
/// # 返回值
///
/// 成功时返回密钥字符串，未设置时返回 `ErrorType::KeyIsNull`
fn have_key() -> CoreResult<String> {
    match KEY.get() {
        None => Err(ErrorType::KeyIsNull),
        Some(key) => Ok(key.clone()),
    }
}

/// 发起 OAuth 设备码授权——第一步：获取登录码
///
/// 向 Microsoft 设备授权端点请求设备码和验证 URL。
/// 用户需要在浏览器中打开返回的 URL 并输入设备码来完成授权。
///
/// # 返回值
///
/// 返回 `OAuthGetCodeRes`，包含：
/// - `code`: 用户需要输入的设备码
/// - `url`: 用户需要访问的验证网址
/// - `device_code`: 后续轮询用的设备码
/// - `expires_in`: 设备码的有效期（秒）
pub async fn get_code() -> CoreResult<OAuthGetCodeRes> {
    let key = have_key()?;

    let obj: &[(&str, &str)] = &[
        ("client_id", &key),
        ("scope", "XboxLive.signin offline_access"),
    ];

    let data = mcml_net::get_login_client()
        .post_form_get_json::<OAuthObj>(urls::OAUTH_CODE, obj)
        .await?;

    match data.error {
        Some(err) => Err(ErrorType::OAuthGetTokenError(ErrorData { error: err })),
        None => Ok(OAuthGetCodeRes {
            code: data.user_code,
            url: data.verification_uri,
            device_code: data.device_code,
            expires_in: data.expires_in,
        }),
    }
}

/// 轮询等待用户完成设备码授权——第二步：获取 Microsoft Token
///
/// 此函数会循环轮询 Microsoft 令牌端点，直到用户完成授权、超时或被取消。
///
/// # 参数
///
/// - `res`: 第一步返回的设备码信息
/// - `cancel`: 取消令牌，用于用户主动终止等待
///
/// # 轮询策略
///
/// - 初始间隔 2 秒
/// - 收到 `slow_down` 错误时递增 5 秒
/// - 超过 `expires_in` 后返回超时错误
///
/// # 返回值
///
/// 成功时返回包含 `access_token` 和 `refresh_token` 的 `OAuthGetCodeObj`
pub async fn run_get_code(
    res: &OAuthGetCodeRes,
    cancel: &CancellationToken,
) -> CoreResult<OAuthGetCodeObj> {
    let key = have_key()?;

    let obj: &[(&str, &str)] = &[
        ("client_id", &key),
        ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
        ("device_code", &res.device_code.clone()),
    ];

    let start_time = Local::now().timestamp();
    let mut delay = 2;

    loop {
        sleep(Duration::from_secs(delay)).await;
        if cancel.is_cancelled() {
            return Err(ErrorType::TaskCancel);
        }

        let estimated_time = Local::now().timestamp() - start_time;
        if estimated_time > res.expires_in {
            return Err(ErrorType::TaskTimeout);
        }

        let data = mcml_net::get_login_client()
            .post_form_get_json::<OAuthGetCodeObj>(urls::OAUTH_TOKEN, obj)
            .await?;

        if let Some(error) = data.error {
            if error == "authorization_pending" {
                // 用户尚未完成授权，继续等待
                continue;
            } else if error == "slow_down" {
                // 服务器要求降低轮询频率
                delay += 5;
            } else if error == "expired_token" {
                return Err(ErrorType::OAuthGetTokenError(ErrorData { error }));
            }
        } else {
            return Ok(data);
        }
    }
}

/// 使用 refresh_token 刷新 Microsoft 令牌
///
/// 当 access_token 过期后，用上次保存的 refresh_token 获取新令牌，
/// 无需用户重新授权。
///
/// # 参数
///
/// - `token`: 之前保存的 refresh_token
///
/// # 返回值
///
/// 成功时返回新的 `OAuthGetCodeObj`（包含新的 access_token 和 refresh_token）
pub async fn refresh_oauth_token(token: &str) -> CoreResult<OAuthGetCodeObj> {
    let key = have_key()?;

    let obj: &[(&str, &str)] = &[
        ("client_id", &key),
        ("grant_type", "refresh_token"),
        ("refresh_token", &token),
    ];

    let data = mcml_net::get_login_client()
        .post_form_get_json::<OAuthGetCodeObj>(urls::OAUTH_TOKEN, obj)
        .await?;

    match data.error {
        Some(err) => Err(ErrorType::AuthRefreshFail(err)),
        None => Ok(data),
    }
}

/// Xbox Live 认证——第三步：用 Microsoft Token 换取 XBL Token
///
/// # 参数
///
/// - `token`: Microsoft OAuth access_token
///
/// # 返回值
///
/// 返回 `XBoxLiveRes`，包含 XBL token 和用户哈希（UHS）
pub async fn get_xbox(token: &str) -> CoreResult<XBoxLiveRes> {
    let obj = XBoxLoginObj {
        properties: XBoxLoginPropertiesObj {
            auth_method: "RPS".to_string(),
            site_name: "user.auth.xboxlive.com".to_string(),
            rps_ticket: format!("d={}", token),
        },
        relying_party: "http://auth.xboxlive.com".to_string(),
        token_type: "JWT".to_string(),
    };

    let data = mcml_net::get_login_client()
        .post_json_get_json::<_, XBoxLoginResObj>(urls::XBOX_LIVE, &obj)
        .await?;
    let item = &data.display_claims.xui[0];
    let token = data.token;
    let uhs = item.uhs.clone();

    if token.is_empty() || uhs.is_empty() {
        Err(ErrorType::OAuthGetTokenEmpty)
    } else {
        Ok(XBoxLiveRes {
            xbl_token: token,
            xbl_uhs: uhs,
        })
    }
}

/// XSTS 认证——第四步：用 XBL Token 换取 XSTS Token
///
/// XSTS（Xbox Secure Token Service）是访问 Minecraft 服务所需的
/// 安全令牌服务。
///
/// # 参数
///
/// - `token`: XBL token
///
/// # 返回值
///
/// 返回 `XBoxLiveRes`，包含 XSTS token 和用户哈希
pub async fn get_xsts(token: &str) -> CoreResult<XBoxLiveRes> {
    let obj = XSTSLoginObj {
        properties: XSTSLoginPropertiesObj {
            sandbox_id: "RETAIL".to_string(),
            user_tokens: vec![token.to_string()],
        },
        relying_party: "rp://api.minecraftservices.com/".to_string(),
        token_type: "JWT".to_string(),
    };

    let data = mcml_net::get_login_client()
        .post_json_get_json::<_, XBoxLoginResObj>(urls::XSTS, &obj)
        .await?;
    let item = &data.display_claims.xui[0];
    let token = data.token;
    let uhs = item.uhs.clone();

    if token.is_empty() || uhs.is_empty() {
        Err(ErrorType::OAuthGetTokenEmpty)
    } else {
        Ok(XBoxLiveRes {
            xbl_token: token,
            xbl_uhs: uhs,
        })
    }
}

impl LoginObj {
    /// 微软正版账户的刷新流程
    ///
    /// 执行完整的认证链刷新：
    /// 1. 先尝试用现有 Minecraft Token 获取 Profile（快速验证）
    /// 2. 失败则执行完整刷新链：refresh_token → Xbox → XSTS → Minecraft Token → Profile
    ///
    /// # 参数
    ///
    /// - `cancel`: 取消令牌，用于中断异步操作
    pub async fn refresh_oauth(&mut self, cancel: CancellationToken) -> CoreResult<()> {
        // 快速路径：尝试用现有 token 获取 profile
        let profile = mojang_api::get_minecraft_profile(&self.access_token).await;
        if profile.is_ok() {
            return Ok(());
        }

        // 完整刷新链
        let oauth = refresh_oauth_token(&self.text1.clone().unwrap()).await?;
        if cancel.is_cancelled() {
            return Err(ErrorType::TaskCancel);
        }
        let xbox = get_xbox(&oauth.access_token).await?;
        if cancel.is_cancelled() {
            return Err(ErrorType::TaskCancel);
        }
        let xsts = get_xsts(&xbox.xbl_token).await?;
        if cancel.is_cancelled() {
            return Err(ErrorType::TaskCancel);
        }
        let token = mojang_api::get_minecraft_token(&xsts.xbl_uhs, &xsts.xbl_token).await?;
        if cancel.is_cancelled() {
            return Err(ErrorType::TaskCancel);
        }
        let profile = mojang_api::get_minecraft_profile(&token).await?;

        // 更新本地账户信息
        self.user_name = profile.name;
        self.uuid = profile.id;
        self.text1 = Some(oauth.refresh_token);
        self.access_token = token;
        self.last_login = Local::now().fixed_offset();

        Ok(())
    }
}
