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
    pub async fn refresh(&mut self, cancel: &CancellationToken) -> CoreResult<()> {
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
#[derive(Eq, Hash, PartialEq, Debug)]
pub struct UserKeyObj {
    /// 账户标识（Minecraft UUID）
    pub uuid: String,
    /// 账户认证类型
    pub auth_type: AuthType,
}
