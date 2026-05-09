use mcml_http::NetError;
use mcml_names::i18_items::error_type::ErrorType;
use tokio_util::sync::CancellationToken;

use crate::legacy::{authenticate_obj::{AgentObj, AuthenticateObj, AuthenticateResObj}, login_res::LegacyLoginRes};

pub mod login_res;
pub mod authenticate_obj;

pub async fn authenticate(server: String, token: String, user: String, password: String, use_minecraft: bool, cancel: CancellationToken) -> Result<LegacyLoginRes, ErrorType> {
    let obj = AuthenticateObj::new(AgentObj::new(use_minecraft), user, password, token);

    let mut server = server;

    if !server.ends_with('/') {
        server.push('/');
    }

    let message = mcml_http::LOGIN_CLIENT.get().unwrap().post_json::<_, AuthenticateResObj>(&server, &obj).await;

    match message {
        Err(err) => {
            Err(ErrorType::from(err))
        },
        Ok(message) => {
            match message.error_message {
                Some(err)
            }
        }
    }
}