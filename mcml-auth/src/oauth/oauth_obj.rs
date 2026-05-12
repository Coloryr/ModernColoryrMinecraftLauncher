use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct OAuthObj {
    pub user_code: String,
    pub error: Option<String>,
    pub device_code: String,
    pub verification_uri: String,
    pub expires_in: i64,
}

impl Default for OAuthObj {
    fn default() -> Self {
        Self {
            user_code: Default::default(),
            error: Default::default(),
            device_code: Default::default(),
            verification_uri: Default::default(),
            expires_in: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct OAuthGetCodeObj {
    pub error: Option<String>,
    pub access_token: String,
    pub refresh_token: String,
}

impl Default for OAuthGetCodeObj {
    fn default() -> Self {
        Self {
            error: Default::default(),
            access_token: Default::default(),
            refresh_token: Default::default(),
        }
    }
}
