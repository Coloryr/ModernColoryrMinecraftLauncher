use serde::{Deserialize, Serialize};

use crate::legacy::selected_profile_obj::SelectedProfileObj;

/// 刷新账户
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct RefreshObj {
    /// 登陆密钥
    #[serde(rename = "accessToken")]
    pub access_token: String,
    /// 客户端标识
    #[serde(rename = "clientToken")]
    pub client_token: String,
    /// 选中的账户
    #[serde(rename = "selectedProfile")]
    pub selected_profile: Option<SelectedProfileObj>,
}

impl Default for RefreshObj {
    fn default() -> Self {
        Self {
            access_token: Default::default(),
            client_token: Default::default(),
            selected_profile: Default::default(),
        }
    }
}
