/// Xbox网络模型
use serde::{Deserialize, Serialize};

/// Xbox 登录属性
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct XBoxLoginPropertiesObj {
    /// 认证方式
    #[serde(rename = "AuthMethod")]
    pub auth_method: String,
    /// 站点名称
    #[serde(rename = "SiteName")]
    pub site_name: String,
    /// RPS 票据
    #[serde(rename = "RpsTicket")]
    pub rps_ticket: String,
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

/// Xbox 登录请求
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct XBoxLoginObj {
    /// 登录属性
    #[serde(rename = "Properties")]
    pub properties: XBoxLoginPropertiesObj,
    /// 依赖方
    #[serde(rename = "RelyingParty")]
    pub relying_party: String,
    /// 令牌类型
    #[serde(rename = "TokenType")]
    pub token_type: String,
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

/// XSTS 登录属性
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct XSTSLoginPropertiesObj {
    /// 沙盒标识
    #[serde(rename = "SandboxId")]
    pub sandbox_id: String,
    /// 用户令牌列表
    #[serde(rename = "UserTokens")]
    pub user_tokens: Vec<String>,
}

impl Default for XSTSLoginPropertiesObj {
    fn default() -> Self {
        Self {
            sandbox_id: Default::default(),
            user_tokens: Default::default(),
        }
    }
}

/// XSTS 登录请求
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct XSTSLoginObj {
    /// 登录属性
    #[serde(rename = "Properties")]
    pub properties: XSTSLoginPropertiesObj,
    /// 依赖方
    #[serde(rename = "RelyingParty")]
    pub relying_party: String,
    /// 令牌类型
    #[serde(rename = "TokenType")]
    pub token_type: String,
}

impl Default for XSTSLoginObj {
    fn default() -> Self {
        Self {
            properties: Default::default(),
            relying_party: Default::default(),
            token_type: Default::default(),
        }
    }
}

/// Xbox 登录显示声明 XUI
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct XBoxLoginDisplayClaimsXuiObj {
    /// 用户哈希
    pub uhs: String,
}

impl Default for XBoxLoginDisplayClaimsXuiObj {
    fn default() -> Self {
        Self {
            uhs: Default::default(),
        }
    }
}

/// Xbox 登录显示声明
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct XBoxLoginDisplayClaimsObj {
    /// XUI 声明列表
    pub xui: Vec<XBoxLoginDisplayClaimsXuiObj>,
}

impl Default for XBoxLoginDisplayClaimsObj {
    fn default() -> Self {
        Self {
            xui: Default::default(),
        }
    }
}

/// Xbox 登录响应
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct XBoxLoginResObj {
    /// 认证令牌
    #[serde(rename = "Token")]
    pub token: String,
    /// 显示声明
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

/// Xbox Live 登录结果
pub struct XBoxLiveRes {
    /// XBL 令牌
    pub xbl_token: String,
    /// XBL 用户哈希
    pub xbl_uhs: String,
}
