use serde::{Deserialize, Serialize};

use crate::legacy::selected_profile_obj::SelectedProfileObj;

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct AgentObj {
    pub name: String,
    pub version: i32,
}

impl AgentObj {
    pub fn new(use_minecraft: bool) -> Self {
        AgentObj {
            name: String::from(if use_minecraft { "Minecraft" } else { "Mcml" }),
            version: if use_minecraft {
                1
            } else {
                mcml_names::VERSION_NUM
            },
        }
    }
}

impl Default for AgentObj {
    fn default() -> Self {
        Self {
            name: Default::default(),
            version: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct AuthenticateObj {
    pub agent: AgentObj,
    pub username: String,
    pub password: String,
    #[serde(rename = "clientToken")]
    pub client_token: String,
}

impl AuthenticateObj {
    pub fn new(agent: AgentObj, username: String, password: String, client_token: String) -> Self {
        AuthenticateObj {
            agent,
            username,
            password,
            client_token,
        }
    }
}

impl Default for AuthenticateObj {
    fn default() -> Self {
        Self {
            agent: Default::default(),
            username: Default::default(),
            password: Default::default(),
            client_token: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct AuthenticateResObj {
    #[serde(rename = "accessToken")]
    pub access_token: String,
    #[serde(rename = "clientToken")]
    pub client_token: String,
    #[serde(rename = "selectedProfile")]
    pub selected_profile: Option<SelectedProfileObj>,
    #[serde(rename = "availableProfiles")]
    pub available_profiles: Option<Vec<SelectedProfileObj>>,
    #[serde(rename = "errorMessage")]
    pub error_message: Option<String>,
}

impl Default for AuthenticateResObj {
    fn default() -> Self {
        Self {
            access_token: Default::default(),
            client_token: Default::default(),
            selected_profile: Default::default(),
            available_profiles: Default::default(),
            error_message: Default::default(),
        }
    }
}
