/// 统一通行证
use mcml_names::{i18_items::error_type::ErrorType, urls::NIDE8_URL};

use crate::{
    AuthType, LoginObj,
    legacy::{self},
};

/// 统一通行证登录
/// - `client_token`: 客户端代码
/// - `user`: 用户名
/// - `password`: 密码
/// - `server`: 服务器UUID
pub async fn authenticate(
    client_token: String,
    user: String,
    password: String,
    server: &String,
) -> Result<LoginObj, ErrorType> {
    let url = String::from(NIDE8_URL) + server;

    let obj = legacy::authenticate(&url, client_token, user, password, false).await?;

    let mut auth = obj.auth;
    auth.auth_type = AuthType::Nide8;
    auth.text1 = Some(server.clone());

    Ok(auth)
}

/// 刷新登录
/// - `auth`: 保存的账户
pub async fn refresh(auth: &LoginObj) -> Result<LoginObj, ErrorType> {
    let server = String::from(NIDE8_URL) + &auth.text1.clone().unwrap();

    if legacy::validate(&server, auth).await? {
        Ok(legacy::refresh(&server, auth, false).await?)
    } else {
        Err(ErrorType::AuthTokenTimeout)
    }
}
