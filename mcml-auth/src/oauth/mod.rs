use std::{sync::OnceLock, time::Duration};

use chrono::Local;
use mcml_names::{
    i18_items::error_type::{ErrorType, OAuthErrorData},
    urls::{OAUTH_CODE, OAUTH_TOKEN, XBOX_LIVE, XSTS},
};
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;

use crate::oauth::{
    oauth_get_code::OAuthGetCodeRes,
    oauth_obj::{OAuthGetCodeObj, OAuthObj},
    xbox_obj::{
        XBoxLiveRes, XBoxLoginObj, XBoxLoginPropertiesObj, XBoxLoginResObj, XSTSLoginObj,
        XSTSLoginPropertiesObj,
    },
};

pub mod oauth_get_code;
pub mod oauth_obj;
pub mod xbox_obj;

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

fn test_key() -> Result<String, ErrorType> {
    match KEY.get() {
        None => Err(ErrorType::OAuthKeyIsNull),
        Some(key) => Ok(key.clone()),
    }
}

/// 获取登录码
pub async fn get_code() -> Result<OAuthGetCodeRes, ErrorType> {
    let key = test_key()?;

    let obj: &[(&str, &str)] = &[
        ("client_id", &key),
        ("scope", "XboxLive.signin offline_access"),
    ];

    let data = mcml_http::LOGIN_CLIENT
        .get()
        .unwrap()
        .post_form_get_json::<OAuthObj>(OAUTH_CODE, obj)
        .await?;

    match data.error {
        Some(err) => Err(ErrorType::OAuthGetTokenError(OAuthErrorData { error: err })),
        None => Ok(OAuthGetCodeRes::new(
            data.user_code,
            data.verification_uri,
            data.device_code,
            data.expires_in,
        )),
    }
}

/// 获取token
/// - `res`: 上一阶段的登录码
/// - `token`: 是否取消获取
pub async fn run_get_code(
    res: &OAuthGetCodeRes,
    token: &CancellationToken,
) -> Result<OAuthGetCodeObj, ErrorType> {
    let key = test_key()?;

    let obj: &[(&str, &str)] = &[
        ("client_id", &key),
        ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
        ("device_code", &res.device_code.clone()),
    ];

    let start_time = Local::now().timestamp();
    let mut delay = 2;

    loop {
        sleep(Duration::from_secs(delay)).await;
        if token.is_cancelled() {
            return Err(ErrorType::TaskCancel);
        }

        let estimated_time = Local::now().timestamp() - start_time;
        if estimated_time > res.expires_in {
            return Err(ErrorType::TaskTimeout);
        }

        let data = mcml_http::LOGIN_CLIENT
            .get()
            .unwrap()
            .post_form_get_json::<OAuthGetCodeObj>(OAUTH_TOKEN, obj)
            .await?;

        if data.error.is_some() {
            let error = data.error.unwrap();
            if error == "authorization_pending" {
                continue;
            } else if error == "slow_down" {
                delay += 5;
            } else if error == "expired_token" {
                return Err(ErrorType::OAuthGetTokenError(OAuthErrorData { error }));
            }
        } else {
            return Ok(data);
        }
    }
}

/// 刷新密匙
/// - `token`: 登录密钥
pub async fn refresh_oauth_token(token: String) -> Result<OAuthGetCodeObj, ErrorType> {
    let key = test_key()?;

    let obj: &[(&str, &str)] = &[
        ("client_id", &key),
        ("grant_type", "refresh_token"),
        ("refresh_token", &token),
    ];

    let data = mcml_http::LOGIN_CLIENT
        .get()
        .unwrap()
        .post_form_get_json::<OAuthGetCodeObj>(OAUTH_TOKEN, obj)
        .await?;

    match data.error {
        Some(err) => Err(ErrorType::AuthRefreshFail(err)),
        None => Ok(data),
    }
}

/// Xbox登录
/// - `token`: OAuth的密钥
pub async fn get_xbox(token: String) -> Result<XBoxLiveRes, ErrorType> {
    let obj = XBoxLoginObj::new(
        XBoxLoginPropertiesObj::new(
            String::from("RPS"),
            String::from("user.auth.xboxlive.com"),
            format!("d={}", token),
        ),
        String::from("http://auth.xboxlive.com"),
        String::from("JWT"),
    );

    let data = mcml_http::LOGIN_CLIENT
        .get()
        .unwrap()
        .post_json_get_json::<_, XBoxLoginResObj>(XBOX_LIVE, &obj)
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

pub async fn get_xsts(token: String) -> Result<XBoxLiveRes, ErrorType> {
    let obj = XSTSLoginObj::new(
        XSTSLoginPropertiesObj::new(String::from("RETAIL"), vec![token]),
        String::from("rp://api.minecraftservices.com/"),
        String::from("JWT"),
    );

    let data = mcml_http::LOGIN_CLIENT
        .get()
        .unwrap()
        .post_json_get_json::<_, XBoxLoginResObj>(XSTS, &obj)
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
