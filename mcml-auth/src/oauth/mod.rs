use std::{sync::OnceLock, time::Duration};

use chrono::Local;
use mcml_names::i18_items::error_type::{CoreResult, ErrorData, ErrorType};
use mcml_net::urls;
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;

use crate::oauth::{
    oauth_obj::{OAuthGetCodeObj, OAuthGetCodeRes, OAuthObj},
    xbox_obj::{
        XBoxLiveRes, XBoxLoginObj, XBoxLoginPropertiesObj, XBoxLoginResObj, XSTSLoginObj,
        XSTSLoginPropertiesObj,
    },
};

pub mod oauth_obj;
pub mod xbox_obj;

/// OAuth客户端密钥
pub static KEY: OnceLock<String> = OnceLock::new();

/// 目前登录状态
pub enum AuthState {
    OAuth,
    XBox,
    XSTS,
    Token,
    Profile,
}

/// 设置OAuth客户端密钥
/// - `key`: 客户端密钥
pub fn set_key(key: String) {
    KEY.get_or_init(|| key);
}

/// 是否设置了密钥
fn have_key() -> CoreResult<String> {
    match KEY.get() {
        None => Err(ErrorType::OAuthKeyIsNull),
        Some(key) => Ok(key.clone()),
    }
}

/// 获取登录码
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

/// 获取token
/// - `res`: 上一阶段的登录码
/// - `token`: 是否取消获取
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
                continue;
            } else if error == "slow_down" {
                delay += 5;
            } else if error == "expired_token" {
                return Err(ErrorType::OAuthGetTokenError(ErrorData { error }));
            }
        } else {
            return Ok(data);
        }
    }
}

/// 刷新密匙
/// - `token`: 登录密钥
pub async fn refresh_oauth_token(token: String) -> CoreResult<OAuthGetCodeObj> {
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

/// Xbox登录
/// - `token`: Xbox的密钥
pub async fn get_xbox(token: String) -> CoreResult<XBoxLiveRes> {
    let obj = XBoxLoginObj {
        properties: XBoxLoginPropertiesObj {
            auth_method: String::from("RPS"),
            site_name: String::from("user.auth.xboxlive.com"),
            rps_ticket: format!("d={}", token),
        },
        relying_party: String::from("http://auth.xboxlive.com"),
        token_type: String::from("JWT"),
    };

    let data = mcml_net::get_login_client()
        .post_json_get_json::<_, XBoxLoginResObj>(urls::XBOX_LIVE, &obj)
        .await?;
    let item = data.display_claims.xui.first().unwrap();
    let xsts = data.token;
    let uhs = item.uhs.clone();

    if xsts.is_empty() || uhs.is_empty() {
        Err(ErrorType::OAuthGetTokenEmpty)
    } else {
        Ok(XBoxLiveRes::new(xsts, uhs))
    }
}

/// XSTS登陆
/// - `token`: XSTS的密钥
pub async fn get_xsts(token: String) -> CoreResult<XBoxLiveRes> {
    let obj = XSTSLoginObj {
        properties: XSTSLoginPropertiesObj {
            sandbox_id: String::from("RETAIL"),
            user_tokens: vec![token],
        },
        relying_party: String::from("rp://api.minecraftservices.com/"),
        token_type: String::from("JWT"),
    };

    let data = mcml_net::get_login_client()
        .post_json_get_json::<_, XBoxLoginResObj>(urls::XSTS, &obj)
        .await?;
    let item = data.display_claims.xui.first().unwrap();
    let xsts = data.token;
    let uhs = item.uhs.clone();

    if xsts.is_empty() || uhs.is_empty() {
        Err(ErrorType::OAuthGetTokenEmpty)
    } else {
        Ok(XBoxLiveRes::new(xsts, uhs))
    }
}
