use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct XBoxLoginPropertiesObj {
    #[serde(rename = "AuthMethod")]
    pub auth_method: String,
    #[serde(rename = "SiteName")]
    pub site_name: String,
    #[serde(rename = "RpsTicket")]
    pub rps_ticket: String,
}

impl XBoxLoginPropertiesObj {
    pub fn new(auth_method: String, site_name: String, rps_ticket: String) -> Self {
        XBoxLoginPropertiesObj {
            auth_method,
            site_name,
            rps_ticket,
        }
    }
}

impl Default for XBoxLoginPropertiesObj {
    fn default() -> Self {
        Self {
            auth_method: Default::default(),
            site_name: Default::default(),
            rps_ticket: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct XBoxLoginObj {
    #[serde(rename = "Properties")]
    pub properties: XBoxLoginPropertiesObj,
    #[serde(rename = "RelyingParty")]
    pub relying_party: String,
    #[serde(rename = "TokenType")]
    pub token_type: String,
}

impl XBoxLoginObj {
    pub fn new(
        properties: XBoxLoginPropertiesObj,
        relying_party: String,
        token_type: String,
    ) -> Self {
        XBoxLoginObj {
            properties,
            relying_party,
            token_type,
        }
    }
}

impl Default for XBoxLoginObj {
    fn default() -> Self {
        Self {
            properties: Default::default(),
            relying_party: Default::default(),
            token_type: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct XBoxLoginDisplayClaimsXuiObj {
    pub uhs: String,
}

impl Default for XBoxLoginDisplayClaimsXuiObj {
    fn default() -> Self {
        Self {
            uhs: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct XBoxLoginDisplayClaimsObj {
    pub xui: Vec<XBoxLoginDisplayClaimsXuiObj>,
}

impl Default for XBoxLoginDisplayClaimsObj {
    fn default() -> Self {
        Self {
            xui: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct XBoxLoginResObj {
    #[serde(rename = "Token")]
    pub token: String,
    #[serde(rename = "DisplayClaims")]
    pub display_claims: XBoxLoginDisplayClaimsObj,
}

impl Default for XBoxLoginResObj {
    fn default() -> Self {
        Self {
            token: Default::default(),
            display_claims: Default::default(),
        }
    }
}

pub struct OAuthXBoxLiveRes {
    pub xbl_token: String,
    pub xbl_uhs: String,
}

impl OAuthXBoxLiveRes {
    pub fn new(xbl_token: String, xbl_uhs: String) -> Self {
        OAuthXBoxLiveRes { xbl_token, xbl_uhs }
    }
}
