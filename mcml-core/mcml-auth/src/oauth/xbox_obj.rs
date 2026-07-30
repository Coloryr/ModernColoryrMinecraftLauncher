//! Xbox Live / XSTS 认证数据模型
//!
//! 本模块定义了与 Xbox Live 认证服务 (XBL) 和 Xbox 安全令牌服务 (XSTS)
//! 通信所需的 JSON 数据结构。
//!
//! # 认证流程中的位置
//!
//! Microsoft Token → **Xbox Live** → **XSTS** → Minecraft Token → Profile
//!
//! # 核心类型
//!
//! - [`XBoxLoginObj`] — XBL 认证请求
//! - [`XSTSLoginObj`] — XSTS 认证请求
//! - [`XBoxLoginResObj`] — 统一的认证响应结构
//! - [`XBoxLiveRes`] — 提取后的认证结果（token + UHS）

use serde::{Deserialize, Serialize};

/// Xbox Live 认证请求的属性部分
///
/// 包含认证方法、站点名称和从 Microsoft Token 生成的 RPS 票据。
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct XBoxLoginPropertiesObj {
    /// 认证方法，固定为 "RPS"
    #[serde(rename = "AuthMethod")]
    pub auth_method: String,
    /// 站点名称，固定为 "user.auth.xboxlive.com"
    #[serde(rename = "SiteName")]
    pub site_name: String,
    /// RPS 票据，格式为 "d={Microsoft Access Token}"
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

/// Xbox Live 认证请求
///
/// 发送至 `https://user.auth.xboxlive.com/user/authenticate`。
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct XBoxLoginObj {
    /// 认证属性
    #[serde(rename = "Properties")]
    pub properties: XBoxLoginPropertiesObj,
    /// 依赖方标识，固定为 "http://auth.xboxlive.com"
    #[serde(rename = "RelyingParty")]
    pub relying_party: String,
    /// 令牌类型，固定为 "JWT"
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

/// XSTS 认证请求的属性部分
///
/// 包含沙盒标识和从 XBL 获取的令牌。
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct XSTSLoginPropertiesObj {
    /// 沙盒标识，固定为 "RETAIL"（零售版 Minecraft）
    #[serde(rename = "SandboxId")]
    pub sandbox_id: String,
    /// 上游令牌列表（XBL Token）
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

/// XSTS 认证请求
///
/// 发送至 `https://xsts.auth.xboxlive.com/xsts/authorize`。
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct XSTSLoginObj {
    /// 认证属性
    #[serde(rename = "Properties")]
    pub properties: XSTSLoginPropertiesObj,
    /// 依赖方标识，固定为 "rp://api.minecraftservices.com/"
    #[serde(rename = "RelyingParty")]
    pub relying_party: String,
    /// 令牌类型，固定为 "JWT"
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

/// Xbox Live 显示声明中的 XUI 条目
///
/// 包含认证返回的用户哈希（User Hash），在后续获取
/// Minecraft Token 时需要用到。
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct XBoxLoginDisplayClaimsXuiObj {
    /// 用户哈希（User Hash），用于 Minecraft 认证
    pub uhs: String,
}

impl Default for XBoxLoginDisplayClaimsXuiObj {
    fn default() -> Self {
        Self {
            uhs: Default::default(),
        }
    }
}

/// Xbox Live 认证响应中的显示声明部分
///
/// 包含 XUI 条目列表，其中第一个条目包含用户哈希。
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

/// Xbox Live 和 XSTS 认证的统一响应结构
///
/// 两个端点返回格式相同，因此共用此结构体。
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct XBoxLoginResObj {
    /// 认证令牌（JWT 格式）
    #[serde(rename = "Token")]
    pub token: String,
    /// 显示声明，包含用户哈希信息
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

/// Xbox Live 认证成功后提取的结果
///
/// 包含后续步骤所需的令牌和用户哈希。
pub struct XBoxLiveRes {
    /// XBL 或 XSTS 令牌
    pub xbl_token: String,
    /// 用户哈希（UHS），用于 Minecraft 服务认证
    pub xbl_uhs: String,
}
