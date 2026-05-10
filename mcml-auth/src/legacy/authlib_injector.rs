/// 外置登录
use mcml_names::i18_items::error_type::ErrorType;

use crate::{
    AuthType, LoginObj,
    legacy::{self, gui_select_handel::SelectGuiHandel},
};

/// 外置登录
/// - `client_token`: 客户端代码
/// - `user`: 用户名
/// - `password`: 密码
/// - `server`: 服务器地址
/// - `gui`: 选择账户回调
pub async fn authenticate(
    client_token: String,
    user: String,
    password: String,
    server: String,
    gui: Option<Box<dyn SelectGuiHandel>>,
) -> Result<LoginObj, ErrorType> {
    let obj = legacy::authenticate(&server, client_token, user, password, true).await?;

    let mut auth = obj.auth;
    auth.auth_type = AuthType::AuthlibInjector;
    auth.text1 = Some(server.clone());

    let need_select = false;

    if let Some(list) = obj.logins {
        match gui {
            Some(gui) => {
                let auths: Vec<String> = list.iter().map(|x| x.user_name.clone()).collect();
                let index = gui.select_auth(auths);
                if let Some(item) = list.get(index as usize) {
                    auth.uuid = item.uuid.clone();
                    auth.user_name = item.user_name.clone();
                }
            }
            None => {
                let item = list.first().unwrap();
                auth.uuid = item.uuid.clone();
                auth.user_name = item.user_name.clone();
            }
        };
    }

    Ok(legacy::refresh(&server, &auth, need_select).await?)
}

/// 刷新登录
/// - `auth`: 保存的账户
pub async fn refresh(auth: &LoginObj) -> Result<LoginObj, ErrorType> {
    let server = auth.text1.clone().unwrap();
    if legacy::validate(&server, auth).await? {
        Ok(legacy::refresh(&server, auth, false).await?)
    } else {
        Err(ErrorType::AuthTokenTimeout)
    }
}
