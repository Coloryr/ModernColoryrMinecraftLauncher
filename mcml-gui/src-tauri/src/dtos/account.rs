//! 账户窗口 DTO：账户列表视图（IPC 返回，前端 wire camelCase）
//!
//! 账户在 Rust 侧以 `mcml_auth::LoginObj`（完整凭据）存储并持久化到 `accounts.json`，
//! 通过 [`AccountStoreDto::from_login`] 转成前端消费的 DTO：
//! 登录信息（userName / authType / loginTime）+ 前端显示（avatarColor / skin / tokenStatus / avatar）。

use mcml_auth::{AuthType, LoginObj};
use serde::{Deserialize, Serialize};

/// 账户列表视图（IPC 返回）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountStoreViewDto {
    pub accounts: Vec<AccountStoreDto>,
    pub current_uuid: Option<String>,
}

/// 单个账户（前端 wire：camelCase，字段即 `mcml-vue/src/lib/types.ts` 的 Account）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountStoreDto {
    /// 玩家用户名
    pub user_name: String,
    /// 账户 UUID
    pub uuid: String,
    /// 账户类型：offline / microsoft / littleskin / authlib / nide8
    pub auth_type: String,
    /// 最后登录时间（展示用）
    pub login_time: String,
    /// 头像渐变起点色（CSS，无皮肤时前端回退占位图）
    pub avatar_color: String,
    /// 皮肤主色（SVG 占位图生成用）
    pub skin: String,
    /// Token 状态：valid / expired
    pub token_status: String,
    /// 皮肤头像（mcml-skin-draw 渲染的 data URI PNG）；无皮肤为 None
    pub avatar: Option<String>,
}

impl AccountStoreDto {
    /// 从 LoginObj（账户完整凭据）构造前端 DTO
    ///
    /// 有皮肤时 `avatar` 应为 `mcml-skin-draw` 渲染的头像 data URI（渲染管线见
    /// `windows/account.rs`）；当前 LoginObj 尚未携带皮肤贴图，故为 None，
    /// 前端回退到原来的占位图显示。
    pub fn from_login(login: &LoginObj) -> Self {
        let (c1, c2) = palette(&login.uuid);
        Self {
            user_name: login.user_name.clone(),
            uuid: login.uuid.clone(),
            auth_type: auth_type_str(&login.auth_type).to_string(),
            login_time: login.last_login.format("%Y-%m-%d %H:%M").to_string(),
            avatar_color: format!("linear-gradient(135deg, {c1}, {c2})"),
            skin: c1,
            token_status: "valid".to_string(),
            avatar: None,
        }
    }
}

/// 前端账户类型字符串 → AuthType
pub fn auth_type_from_str(s: &str) -> AuthType {
    match s {
        "microsoft" => AuthType::OAuth,
        "littleskin" => AuthType::LittleSkin,
        "authlib" => AuthType::AuthlibInjector,
        "nide8" => AuthType::Nide8,
        _ => AuthType::Offline,
    }
}

/// AuthType → 前端账户类型字符串（自建皮肤站并入 littleskin 显示）
fn auth_type_str(t: &AuthType) -> &'static str {
    match t {
        AuthType::Offline => "offline",
        AuthType::OAuth => "microsoft",
        AuthType::LittleSkin | AuthType::SelfLittleSkin => "littleskin",
        AuthType::AuthlibInjector => "authlib",
        AuthType::Nide8 => "nide8",
    }
}

/// 按 uuid 哈希取配色（无皮肤时的占位色）
fn palette(uuid: &str) -> (String, String) {
    const P: [(&str, &str); 6] = [
        ("#3f8cff", "#5f6cff"),
        ("#34d399", "#22d3ee"),
        ("#a855f7", "#ec4899"),
        ("#f59e0b", "#ef4444"),
        ("#06b6d4", "#6366f1"),
        ("#f472b6", "#8b5cf6"),
    ];
    let h = uuid
        .bytes()
        .fold(0u32, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u32));
    let (c1, c2) = P[(h as usize) % P.len()];
    (c1.to_string(), c2.to_string())
}
