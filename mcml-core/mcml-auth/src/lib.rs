//! Minecraft 启动器账户认证模块
//!
//! 本模块负责管理 Minecraft 启动器中的用户账户体系，支持多种认证方式：
//!
//! # 支持的认证类型
//!
//! | 认证类型 | 枚举变体 | 说明 |
//! |---------|---------|------|
//! | 离线账户 | `Offline` | 无需联网验证的离线模式 |
//! | 微软正版 | `OAuth` | 通过 Microsoft OAuth 2.0 流程认证 |
//! | 统一通行证 | `Nide8` | 第三方统一通行证认证 |
//! | 外置登录 | `AuthlibInjector` | Authlib-Injector 外置认证 |
//! | LittleSkin | `LittleSkin` | LittleSkin 皮肤站认证 |
//! | 自建皮肤站 | `SelfLittleSkin` | 自建 LittleSkin 皮肤站认证 |
//!
//! # 模块结构
//!
//! - [`auths`] — 账户持久化存储管理
//! - [`legacy`] — 旧版 Yggdrasil 认证协议（外置登录、皮肤站、统一通行证）
//! - [`oauth`] — Microsoft OAuth 2.0 现代认证协议（Xbox Live → XSTS → Minecraft）
//!
//! # 账户数据结构
//!
//! 核心类型 [`LoginObj`] 存储一个账户的完整凭据信息，
//! 包括用户名、UUID、access token、client token、认证类型等。
//! 账户通过 [`UserKeyObj`]（UUID + 认证类型）作为唯一键进行索引。

/// 游戏账户
use chrono::{DateTime, FixedOffset, Local};
use mcml_names::i18_items::error_type::CoreResult;
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use tokio_util::sync::CancellationToken;

/// 旧版 Yggdrasil 认证协议模块
pub mod auths;
/// 旧版 Yggdrasil 认证协议（外置登录、皮肤站、统一通行证）
pub mod legacy;
/// Microsoft OAuth 2.0 认证协议
pub mod oauth;

/// 账户认证类型
///
/// 定义了启动器支持的六种账户认证方式。
/// 使用 `#[repr(u8)]` 标记，可高效序列化为单字节存储。
#[derive(Serialize_repr, Deserialize_repr, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum AuthType {
    /// 离线账户，无需联网验证，用户名可自定义
    Offline,
    /// 微软正版登录（Microsoft OAuth 2.0 + Xbox Live 认证链）
    OAuth,
    /// 统一通行证（Nide8）第三方认证
    Nide8,
    /// Authlib-Injector 外置登录认证
    AuthlibInjector,
    /// LittleSkin 官方皮肤站认证
    LittleSkin,
    /// 自建 LittleSkin 皮肤站认证
    SelfLittleSkin,
}

/// 默认认证类型为离线账户
impl Default for AuthType {
    fn default() -> Self {
        AuthType::Offline
    }
}

impl AuthType {
    pub fn from_str(str: &str) -> AuthType {
        if str == "Offline" {
            AuthType::Offline
        } else if str == "OAuth" {
            AuthType::OAuth
        } else if str == "Nide8" {
            AuthType::Nide8
        } else if str == "AuthlibInjector" {
            AuthType::AuthlibInjector
        } else if str == "LittleSkin" {
            AuthType::LittleSkin
        } else if str == "SelfLittleSkin" {
            AuthType::SelfLittleSkin
        } else {
            AuthType::Offline
        }
    }
}

/// 保存的账户信息
///
/// 存储一个 Minecraft 账户的完整凭据，用于序列化持久化和登录验证。
/// 字段使用 PascalCase 命名以兼容 JSON 序列化格式。
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct LoginObj {
    /// 用户名
    #[serde(rename = "UserName")]
    pub user_name: String,
    /// 账户 UUID（Minecraft 格式，带连字符）
    #[serde(rename = "UUID")]
    pub uuid: String,
    /// 登录访问令牌（access token），用于验证身份
    #[serde(rename = "AccessToken")]
    pub access_token: String,
    /// 客户端标识令牌（client token），用于标识启动器实例
    #[serde(rename = "ClientToken")]
    pub client_token: String,
    /// 账户认证类型
    #[serde(rename = "AuthType")]
    pub auth_type: AuthType,
    /// 扩展字段 1：
    /// - OAuth: 存储 refresh_token
    /// - Nide8: 存储服务器 UUID
    /// - AuthlibInjector/LittleSkin: 存储服务器地址
    #[serde(rename = "Text1")]
    pub text1: Option<String>,
    /// 扩展字段 2（预留，当前未使用）
    #[serde(rename = "Text2")]
    pub text2: Option<String>,
    /// 最后登录时间（带时区的日期时间）
    #[serde(rename = "LastLogin")]
    pub last_login: DateTime<FixedOffset>,
}

impl LoginObj {
    /// 创建完整的账户信息
    ///
    /// 使用当前时间作为最后登录时间，其他扩展字段初始化为空。
    ///
    /// # 参数
    ///
    /// - `user_name`: 玩家用户名
    /// - `uuid`: 账户 UUID
    /// - `access_token`: 登录访问令牌
    /// - `client_token`: 客户端标识令牌
    pub fn new(
        user_name: String,
        uuid: String,
        access_token: String,
        client_token: String,
    ) -> Self {
        let dt = Local::now();
        let dt_new: DateTime<FixedOffset> = dt.fixed_offset();

        Self {
            user_name,
            uuid,
            access_token,
            client_token,
            auth_type: Default::default(),
            text1: Default::default(),
            text2: Default::default(),
            last_login: dt_new,
        }
    }

    /// 创建空白账户（仅有用户名和 UUID，无令牌）
    ///
    /// 用于存储从认证服务器返回的可选角色列表中尚未选中的账户。
    ///
    /// # 参数
    ///
    /// - `user_name`: 玩家用户名
    /// - `uuid`: 账户 UUID
    pub fn new_empty(user_name: String, uuid: String) -> Self {
        Self {
            user_name,
            uuid,
            access_token: Default::default(),
            client_token: Default::default(),
            auth_type: Default::default(),
            text1: Default::default(),
            text2: Default::default(),
            last_login: Default::default(),
        }
    }

    /// 创建仅有令牌的账户（无用户名和 UUID）
    ///
    /// 用于多角色选择场景：已获取令牌但尚未确定具体角色。
    ///
    /// # 参数
    ///
    /// - `access_token`: 登录访问令牌
    /// - `client_token`: 客户端标识令牌
    pub fn new_token(access_token: String, client_token: String) -> Self {
        Self {
            user_name: Default::default(),
            uuid: Default::default(),
            access_token,
            client_token,
            auth_type: Default::default(),
            text1: Default::default(),
            text2: Default::default(),
            last_login: Default::default(),
        }
    }

    /// 获取账户的唯一键（UUID + 认证类型）
    ///
    /// 用于在账户存储中索引和去重。
    pub fn get_key(&self) -> UserKeyObj {
        UserKeyObj {
            uuid: self.uuid.clone(),
            auth_type: self.auth_type.clone(),
        }
    }

    /// 根据认证类型刷新登录凭据
    ///
    /// 此方法会根据 `auth_type` 字段分派到对应的刷新逻辑：
    /// - `OAuth` → 微软 OAuth 刷新链
    /// - `Nide8` → 统一通行证刷新
    /// - `AuthlibInjector` → 外置登录刷新
    /// - `LittleSkin` / `SelfLittleSkin` → 皮肤站刷新
    /// - 离线账户 → 直接返回成功（无需刷新）
    ///
    /// # 参数
    ///
    /// - `cancel`: 取消令牌，用于中断异步操作
    pub async fn refresh(&mut self, cancel: CancellationToken) -> CoreResult<()> {
        match &self.auth_type {
            AuthType::OAuth => self.refresh_oauth(cancel).await,
            AuthType::Nide8 => self.refresh_nide8(cancel).await,
            AuthType::AuthlibInjector => self.refresh_authlib(cancel).await,
            AuthType::LittleSkin | AuthType::SelfLittleSkin => {
                self.refresh_littleskin(cancel).await
            }
            _ => Ok(()),
        }
    }
}

/// LoginObj 的默认值：空账户
impl Default for LoginObj {
    fn default() -> Self {
        Self {
            user_name: Default::default(),
            uuid: Default::default(),
            access_token: Default::default(),
            client_token: Default::default(),
            auth_type: Default::default(),
            text1: Default::default(),
            text2: Default::default(),
            last_login: Default::default(),
        }
    }
}

/// 账户唯一键
///
/// 由 UUID 和认证类型组成，用于在账户存储中唯一标识一个账户。
/// 同一 UUID 的不同认证类型视为不同账户。
#[derive(Eq, Hash, PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct UserKeyObj {
    /// 账户标识（Minecraft UUID）
    pub uuid: String,
    /// 账户认证类型
    pub auth_type: AuthType,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// 测试用的假凭据（与真实账户无关）
    const FAKE_UUID: &str = "00000000-0000-0000-0000-00000000aaaa";
    const FAKE_TOKEN: &str = "fake-access-token";
    const FAKE_CLIENT: &str = "fake-client-token";

    /// AuthType 按 `#[repr(u8)]` 顺位序列化为 0..=5
    #[test]
    fn test_auth_type_repr_values() {
        let cases = [
            (AuthType::Offline, 0u8),
            (AuthType::OAuth, 1),
            (AuthType::Nide8, 2),
            (AuthType::AuthlibInjector, 3),
            (AuthType::LittleSkin, 4),
            (AuthType::SelfLittleSkin, 5),
        ];
        for (ty, num) in cases {
            let json = serde_json::to_string(&ty).unwrap();
            assert_eq!(json, num.to_string(), "{:?} 应序列化为 {}", ty, num);
            let back: AuthType = serde_json::from_str(&json).unwrap();
            assert_eq!(back, ty);
        }
    }

    /// AuthType 反序列化非法数字应失败（无 `#[serde(other)]` 兜底）
    #[test]
    fn test_auth_type_deserialize_invalid() {
        assert!(serde_json::from_str::<AuthType>("6").is_err());
        assert!(serde_json::from_str::<AuthType>("-1").is_err());
        assert!(serde_json::from_str::<AuthType>("\"Offline\"").is_err());
    }

    /// from_str 应识别全部认证类型名称
    #[test]
    fn test_auth_type_from_str() {
        assert_eq!(AuthType::from_str("Offline"), AuthType::Offline);
        assert_eq!(AuthType::from_str("OAuth"), AuthType::OAuth);
        assert_eq!(AuthType::from_str("Nide8"), AuthType::Nide8);
        assert_eq!(AuthType::from_str("AuthlibInjector"), AuthType::AuthlibInjector);
        assert_eq!(AuthType::from_str("LittleSkin"), AuthType::LittleSkin);
        assert_eq!(AuthType::from_str("SelfLittleSkin"), AuthType::SelfLittleSkin);
    }

    /// from_str 对未知名称回退为 Offline（文档化现有行为）
    #[test]
    fn test_auth_type_from_str_fallback() {
        assert_eq!(AuthType::from_str("unknown"), AuthType::Offline);
        assert_eq!(AuthType::from_str("offline"), AuthType::Offline);
        assert_eq!(AuthType::from_str(""), AuthType::Offline);
    }

    /// LoginObj 序列化应使用 PascalCase 字段名，且可完整往返
    #[test]
    fn test_login_obj_json_round_trip() {
        let mut obj = LoginObj::new(
            "Steve".to_string(),
            FAKE_UUID.to_string(),
            FAKE_TOKEN.to_string(),
            FAKE_CLIENT.to_string(),
        );
        obj.auth_type = AuthType::OAuth;
        obj.text1 = Some("fake-refresh-token".to_string());
        obj.text2 = Some("extra".to_string());

        let json = serde_json::to_value(&obj).unwrap();
        let map = json.as_object().unwrap();
        // 字段名与旧版持久化格式保持兼容
        for key in [
            "UserName",
            "UUID",
            "AccessToken",
            "ClientToken",
            "AuthType",
            "Text1",
            "Text2",
            "LastLogin",
        ] {
            assert!(map.contains_key(key), "缺少字段 {}", key);
        }
        assert_eq!(map["UserName"], "Steve");
        assert_eq!(map["AuthType"], 1);

        let back: LoginObj = serde_json::from_value(json).unwrap();
        assert_eq!(back.user_name, obj.user_name);
        assert_eq!(back.uuid, obj.uuid);
        assert_eq!(back.access_token, obj.access_token);
        assert_eq!(back.client_token, obj.client_token);
        assert_eq!(back.auth_type, obj.auth_type);
        assert_eq!(back.text1, obj.text1);
        assert_eq!(back.text2, obj.text2);
        assert_eq!(back.last_login, obj.last_login);
    }

    /// 缺失字段的旧 JSON 应通过 `#[serde(default)]` 填充默认值
    #[test]
    fn test_login_obj_partial_json_defaults() {
        let json = r#"{"UserName":"Alex"}"#;
        let obj: LoginObj = serde_json::from_str(json).unwrap();
        assert_eq!(obj.user_name, "Alex");
        assert_eq!(obj.uuid, "");
        assert_eq!(obj.access_token, "");
        assert_eq!(obj.client_token, "");
        assert_eq!(obj.auth_type, AuthType::Offline);
        assert_eq!(obj.text1, None);
        assert_eq!(obj.text2, None);
    }

    /// new() 应使用当前时间，new_empty()/new_token() 应只填指定字段
    #[test]
    fn test_login_obj_constructors() {
        let full = LoginObj::new(
            "Steve".to_string(),
            FAKE_UUID.to_string(),
            FAKE_TOKEN.to_string(),
            FAKE_CLIENT.to_string(),
        );
        assert_eq!(full.user_name, "Steve");
        assert_eq!(full.uuid, FAKE_UUID);
        assert_eq!(full.auth_type, AuthType::Offline);
        assert_eq!(full.text1, None);
        // 最后登录时间应为当前时间（与默认值 0 明显不同）
        assert!(full.last_login.timestamp() > 0);

        let empty = LoginObj::new_empty("Alex".to_string(), FAKE_UUID.to_string());
        assert_eq!(empty.user_name, "Alex");
        assert_eq!(empty.access_token, "");
        assert_eq!(empty.client_token, "");
        // 空账户的最后登录时间为默认值（Unix 纪元）
        assert_eq!(empty.last_login.timestamp(), 0);

        let token = LoginObj::new_token(FAKE_TOKEN.to_string(), FAKE_CLIENT.to_string());
        assert_eq!(token.access_token, FAKE_TOKEN);
        assert_eq!(token.user_name, "");
        assert_eq!(token.uuid, "");
    }

    /// get_key 应由 UUID + 认证类型组成；HashMap 中同 UUID 不同类型视为不同账户
    #[test]
    fn test_user_key_obj_distinct() {
        let mut a = LoginObj::new(
            "Steve".to_string(),
            FAKE_UUID.to_string(),
            FAKE_TOKEN.to_string(),
            FAKE_CLIENT.to_string(),
        );
        let key = a.get_key();
        assert_eq!(key.uuid, FAKE_UUID);
        assert_eq!(key.auth_type, AuthType::Offline);

        let mut map: HashMap<UserKeyObj, LoginObj> = HashMap::new();
        map.insert(a.get_key(), a.clone());

        // 同 UUID 不同认证类型应视为不同账户
        let mut b = a.clone();
        b.auth_type = AuthType::OAuth;
        map.insert(b.get_key(), b);
        assert_eq!(map.len(), 2);

        // 相同键应覆盖
        a.access_token = "new-token".to_string();
        map.insert(a.get_key(), a.clone());
        assert_eq!(map.len(), 2);
        assert_eq!(map[&a.get_key()].access_token, "new-token");
    }

    /// 离线账户 refresh 无需网络，应直接返回成功
    #[tokio::test]
    async fn test_refresh_offline_noop() {
        let mut obj = LoginObj::new(
            "Steve".to_string(),
            FAKE_UUID.to_string(),
            FAKE_TOKEN.to_string(),
            FAKE_CLIENT.to_string(),
        );
        obj.refresh(CancellationToken::new()).await.unwrap();
    }
}
