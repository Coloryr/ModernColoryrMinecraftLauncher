/// LittleSkin登录
use mcml_names::i18_items::error_type::{CoreResult, ErrorType};
use mcml_net::urls;
use tokio_util::sync::CancellationToken;

use crate::{
    AuthType, LoginObj,
    legacy::{self, GuiSelectHandel},
};

/// 皮肤站登录
/// - `client_token`: 客户端标识
/// - `user`: 用户名
/// - `password`: 密码
/// - `server`: 服务器地址
/// - `gui`: 选择账户回调
pub async fn authenticate(
    client_token: String,
    user: String,
    password: String,
    server: Option<String>,
    gui: Option<Box<dyn GuiSelectHandel>>,
) -> CoreResult<LoginObj> {
    let mut auth_type = AuthType::LittleSkin;
    let server = match server {
        None => String::from(urls::LITTLE_SKIN_URL),
        Some(server) => {
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
        }
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

    legacy::refresh(&server1, &mut auth, true).await?;
    Ok(auth)
}

impl LoginObj {
    /// 刷新登录
    /// - `auth`: 保存的账户
    pub async fn refresh_littleskin(&mut self, cancel: &CancellationToken) -> CoreResult<()> {
        let mut server = if self.auth_type == AuthType::LittleSkin {
            String::from(urls::LITTLE_SKIN_URL)
        } else {
            self.text1.clone().unwrap()
        };

        server.push_str("api/yggdrasil");

        if legacy::validate(&server, self).await? {
            if cancel.is_cancelled() {
                return Err(ErrorType::TaskCancel);
            }
            Ok(legacy::refresh(&server, self, false).await?)
        } else {
            Err(ErrorType::AuthTokenTimeout)
        }
    }

    /// 获取启动时所需的密钥
    pub async fn get_littleskin_key(&self) -> CoreResult<String> {
        let mut server = if self.auth_type == AuthType::LittleSkin {
            String::from(urls::LITTLE_SKIN_URL)
        } else {
            self.text1.clone().unwrap()
        };

        server.push_str("api/yggdrasil");

        Ok(mcml_net::get_login_client().get_text(&server).await?)
    }
}
