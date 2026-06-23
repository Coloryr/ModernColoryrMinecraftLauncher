/// 统一通行证
use mcml_names::i18_items::error_type::{CoreResult, ErrorType};
use mcml_net::urls;

use crate::{
    AuthType, LoginObj,
    legacy::{self},
};

/// 统一通行证登录
/// - `client_token`: 客户端标识
/// - `user`: 用户名
/// - `password`: 密码
/// - `server`: 服务器UUID
pub async fn authenticate(
    client_token: String,
    user: String,
    password: String,
    server: String,
) -> CoreResult<LoginObj> {
    let url = String::from(urls::NIDE8_URL) + &server;

    let obj = legacy::authenticate(&url, client_token, user, password, false).await?;

    let mut auth = obj.auth;
    auth.auth_type = AuthType::Nide8;
    auth.text1 = Some(server.clone());

    Ok(auth)
}

/// 刷新登录
/// - `auth`: 保存的账户
pub async fn refresh(auth: &LoginObj) -> CoreResult<LoginObj> {
    let server = String::from(urls::NIDE8_URL) + &auth.text1.clone().unwrap();

    if legacy::validate(&server, auth).await? {
        Ok(legacy::refresh(&server, auth, false).await?)
    } else {
        Err(ErrorType::AuthTokenTimeout)
    }
}
