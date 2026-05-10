use serde::{Deserialize, Serialize};

use crate::legacy::selected_profile_obj::SelectedProfileObj;

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct RefreshObj {
    #[serde(rename = "accessToken")]
    pub access_token: String,
    #[serde(rename = "clientToken")]
    pub client_token: String,
    #[serde(rename = "selectedProfile")]
    pub selected_profile: SelectedProfileObj,
}

impl RefreshObj {
    pub fn new(access_token: String, client_token: String, selected_profile: SelectedProfileObj) -> Self {

    }
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
