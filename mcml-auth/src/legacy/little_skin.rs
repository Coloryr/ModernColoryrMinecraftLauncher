/// LittleSkin登录
use mcml_names::{i18_items::error_type::ErrorType, urls::LITTLE_SKIN_URL};

use crate::{
    AuthType, LoginObj,
    legacy::{self, gui_select_handel::GuiSelectHandel},
};

/// 皮肤站登录
/// - `client_token`: 客户端代码
/// - `user`: 用户名
/// - `password`: 密码
/// - `server`: 服务器地址
/// - `gui`: 选择账户回调
pub async fn authenticate(
    client_token: String,
    user: String,
    password: String,
    server: &String,
    gui: Option<Box<dyn GuiSelectHandel>>,
) -> Result<LoginObj, ErrorType> {
    let mut auth_type = AuthType::LittleSkin;
    let server = if server.is_empty() {
        String::from(LITTLE_SKIN_URL)
    } else {
        auth_type = AuthType::SelfLittleSkin;
        let mut server = server.clone();
        if server.ends_with("/api/yggdrasil") {
            server = server.replace("/api/yggdrasil", "/");
        }
        if server.ends_with("/user") {
            server = server.replace("/user", "/");
        }
        if !server.ends_with('/') {
            server.push('/');
        }

        server
    };

    let server1 = server.clone() + "api/yggdrasil";

    let obj = legacy::authenticate(&server1, client_token, user, password, true).await?;

    let mut auth = obj.auth;

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

    auth.auth_type = auth_type;
    if auth_type == AuthType::SelfLittleSkin {
        auth.text1 = Some(server.clone());
    }

    Ok(legacy::refresh(&server1, &auth, true).await?)
}

/// 刷新登录
/// - `auth`: 保存的账户
pub async fn refresh(auth: &LoginObj) -> Result<LoginObj, ErrorType> {
    let mut server = if auth.auth_type == AuthType::LittleSkin {
        String::from(LITTLE_SKIN_URL)
    } else {
        auth.text1.clone().unwrap()
    };

    server.push_str("api/yggdrasil");

    if legacy::validate(&server, auth).await? {
        Ok(legacy::refresh(&server, auth, false).await?)
    } else {
        Err(ErrorType::AuthTokenTimeout)
    }
}
