/// 旧版账户验证
use chrono::Local;
use mcml_names::i18_items::error_type::{CoreResult, ErrorType};
use reqwest::StatusCode;

use crate::{
    LoginObj,
    legacy::authenticate_obj::{
        AgentObj, AuthenticateObj, AuthenticateResObj, RefreshObj, SelectedProfileObj,
    },
};

pub mod authenticate_obj;
pub mod authlib_injector;
pub mod little_skin;
pub mod nide8;

pub trait GuiSelectHandel {
    /// 选择登陆的账户
    /// - `auths`: 账户列表
    fn select_auth(&self, auths: Vec<String>) -> i32;
}

/// 旧版方式登录结果
pub struct LegacyLoginRes {
    /// 选中的账户
    pub auth: LoginObj,
    /// 可选的账户列表
    pub logins: Option<Vec<LoginObj>>,
}

/// 登录
/// - `server`: 服务器地址
/// - `client_token`: 客户端标识
/// - `user`: 用户名
/// - `password`: 密码
/// - `use_minecraft`: 使用minecraft头
pub async fn authenticate(
    server: &String,
    client_token: String,
    user: String,
    password: String,
    use_minecraft: bool,
) -> CoreResult<LegacyLoginRes> {
    let obj = AuthenticateObj {
        agent: AgentObj::new(use_minecraft),
        username: user.clone(),
        password,
        client_token,
    };

    let mut server = server.clone();

    if !server.ends_with('/') {
        server.push('/');
    }

    server.push_str("authserver/authenticate");

    let obj = mcml_net::get_login_client()
        .post_json_get_json::<_, AuthenticateResObj>(&server, &obj)
        .await?;

    if let Some(data) = obj.error_message {
        Err(ErrorType::AuthLoginFail(data))
    } else if obj.selected_profile.is_none() && obj.available_profiles.is_none() {
        Err(ErrorType::AuthLoginNoProfile)
    } else if let Some(data) = obj.selected_profile {
        Ok(LegacyLoginRes {
            auth: LoginObj::new(data.name, data.id, obj.access_token, obj.client_token),
            logins: None,
        })
    } else if let Some(list) = obj.available_profiles {
        if list.len() == 0 {
            Err(ErrorType::AuthLoginNoProfile)
        } else if list.len() == 1 {
            let temp = list.first().unwrap();

            Ok(LegacyLoginRes {
                auth: LoginObj::new(
                    temp.name.clone(),
                    temp.id.clone(),
                    obj.access_token,
                    obj.client_token,
                ),
                logins: None,
            })
        } else {
            if let Some(item) = list
                .iter()
                .find(|item| item.name.eq_ignore_ascii_case(&user))
            {
                Ok(LegacyLoginRes {
                    auth: LoginObj::new(
                        item.name.clone(),
                        item.id.clone(),
                        obj.access_token,
                        obj.client_token,
                    ),
                    logins: None,
                })
            } else {
                let mut logins: Vec<LoginObj> = Vec::new();
                for item in list.iter() {
                    logins.push(LoginObj::new_empty(item.name.clone(), item.id.clone()));
                }

                Ok(LegacyLoginRes {
                    auth: LoginObj::new_token(obj.access_token, obj.client_token),
                    logins: Some(logins),
                })
            }
        }
    } else {
        Err(ErrorType::AuthLoginNoProfile)
    }
}

/// 刷新登录
/// - `server`: 服务器地址
/// - `login`: 保存的账户
/// - `select`: 是否为选择模式
pub async fn refresh(server: &String, login: &mut LoginObj, select: bool) -> CoreResult<()> {
    let obj = if select {
        RefreshObj {
            access_token: login.access_token.clone(),
            client_token: login.client_token.clone(),
            selected_profile: Some(SelectedProfileObj {
                name: login.user_name.clone(),
                id: login.uuid.clone(),
            }),
        }
    } else {
        RefreshObj {
            access_token: login.access_token.clone(),
            client_token: login.client_token.clone(),
            selected_profile: None,
        }
    };

    let mut server = server.clone();

    if !server.ends_with('/') {
        server.push('/');
    }

    server.push_str("authserver/refresh");

    let obj = mcml_net::get_login_client()
        .post_json_get_json::<_, AuthenticateResObj>(&server, &obj)
        .await?;

    if let Some(data) = obj.error_message {
        Err(ErrorType::AuthLoginFail(data))
    } else if obj.selected_profile.is_none() && !select {
        Err(ErrorType::AuthRefreshNoProfile)
    } else if obj.selected_profile.is_some() {
        let select = obj.selected_profile.unwrap();
        login.user_name = select.name;
        login.uuid = select.id;
        login.access_token = obj.access_token;
        login.client_token = obj.client_token;
        login.last_login = Local::now().fixed_offset();

        Ok(())
    } else {
        login.access_token = obj.access_token;
        login.client_token = obj.client_token;
        login.last_login = Local::now().fixed_offset();

        Ok(())
    }
}

/// 检测密钥可用性
/// - `server`: 检测地址
/// - `login`: 保存的账户
pub async fn validate(server: &String, login: &LoginObj) -> CoreResult<bool> {
    let obj = RefreshObj {
        access_token: login.access_token.clone(),
        client_token: login.client_token.clone(),
        selected_profile: None,
    };

    let mut server = server.clone();

    if !server.ends_with('/') {
        server.push('/');
    }

    server.push_str("authserver/validate");

    let obj = mcml_net::get_login_client()
        .post_json_get_req(&server, &obj)
        .await?;

    Ok(obj.status() == StatusCode::NO_CONTENT)
}
