/// 旧版账户验证
use mcml_names::i18_items::error_type::ErrorType;

use crate::{
    LoginObj,
    legacy::{
        authenticate_obj::{AgentObj, AuthenticateObj, AuthenticateResObj},
    },
};

pub mod authenticate_obj;
pub mod refresh_obj;
pub mod selected_profile_obj;

/// 旧版方式登录结果
pub struct LegacyLoginRes {
    /// 选中的账户
    pub auth: LoginObj,
    /// 可选的账户列表
    pub logins: Option<Vec<LoginObj>>,
}

impl LegacyLoginRes {
    pub fn new(auth: LoginObj) -> Self {
        LegacyLoginRes { auth, logins: None }
    }
}

/// 登录
/// - `server`: 服务器地址
/// - `client_token`: 客户端标识
/// - `user`: 用户名
/// - `password`: 密码
/// - `use_minecraft`: 使用minecraft头
pub async fn authenticate(
    server: String,
    client_token: String,
    user: String,
    password: String,
    use_minecraft: bool,
) -> Result<LegacyLoginRes, ErrorType> {
    let obj = AuthenticateObj::new(
        AgentObj::new(use_minecraft),
        user.clone(),
        password,
        client_token,
    );

    let mut server = server;

    if !server.ends_with('/') {
        server.push('/');
    }

    server.push_str("authserver/authenticate");

    let obj = mcml_http::LOGIN_CLIENT
        .get()
        .unwrap()
        .post_json::<_, AuthenticateResObj>(&server, &obj)
        .await?;

    match obj.error_message {
        Some(err) => Err(ErrorType::LoginFail(err)),
        None => {
            if obj.selected_profile.is_none() && obj.available_profiles.is_none() {
                Err(ErrorType::LoginNoProfile)
            } else if obj.selected_profile.is_some() {
                let temp = obj.selected_profile.unwrap();

                Ok(LegacyLoginRes::new(LoginObj::new(
                    temp.name,
                    temp.id,
                    obj.access_token,
                    obj.client_token,
                )))
            } else {
                match obj.available_profiles {
                    Some(list) => {
                        if list.len() == 1 {
                            let temp = list.first().unwrap();

                            Ok(LegacyLoginRes::new(LoginObj::new(
                                temp.name.clone(),
                                temp.id.clone(),
                                obj.access_token,
                                obj.client_token,
                            )))
                        } else {
                            let temp = list
                                .iter()
                                .find(|item| item.name.eq_ignore_ascii_case(&user));
                            match temp {
                                Some(item) => Ok(LegacyLoginRes::new(LoginObj::new(
                                    item.name.clone(),
                                    item.id.clone(),
                                    obj.access_token,
                                    obj.client_token,
                                ))),
                                None => {
                                    let mut logins: Vec<LoginObj> = Vec::new();
                                    for item in list.iter() {
                                        logins.push(LoginObj::new_empty(
                                            item.name.clone(),
                                            item.id.clone(),
                                        ));
                                    }

                                    Ok(LegacyLoginRes {
                                        auth: LoginObj::new_token(
                                            obj.access_token,
                                            obj.client_token,
                                        ),
                                        logins: Some(logins),
                                    })
                                }
                            }
                        }
                    }
                    None => Err(ErrorType::LoginNoProfile),
                }
            }
        }
    }
}

pub async fn refresh(server: String, obj: LoginObj, select: bool) -> Result<LoginObj, ErrorType> {
    let obj = AuthenticateObj::new(
        AgentObj::new(use_minecraft),
        user.clone(),
        password,
        client_token,
    );

    let mut server = server;

    if !server.ends_with('/') {
        server.push('/');
    }

    let obj = mcml_http::LOGIN_CLIENT
        .get()
        .unwrap()
        .post_json::<_, AuthenticateResObj>(&server, &obj)
        .await?;
}
