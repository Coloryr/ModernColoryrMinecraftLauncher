//! 微软设备码登录（OAuth）：发起 / 轮询 / 取消，以及登录过程的事件
//!
//! 从 `account/mod.rs` 拆出来的。设备码流程是"前端拿到码 → 用户去浏览器输码 → 后端轮询
//! 拿 token"，所以状态全靠 `account-oauth` / `account-oauth-state` 事件推给前端。
//! 命令带 `#[gui_macros::ipc_group("account")]` 把组键钉回 `account`（AGENTS.md §4）。

use std::fmt::Display;
use std::sync::RwLock;

use mml_auth::{AuthType, LoginObj, auths, oauth};
use mml_names::i18_items::error_type::{CoreResult, ErrorType};
use mml_net::mojang_api::{self, MinecraftProfileObj};
use tauri::{AppHandle, Emitter};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::dtos::account_dto::{AccountOAuthDto, AccountOAuthStateDto, AccountStoreDto};
use crate::listens;

use super::emit_account_change;

/// 进行中的微软登录轮询取消句柄（None = 无登录进行中）
pub(super) static OAUTH_NOW: RwLock<Option<CancellationToken>> = RwLock::new(None);

/// 微软登录设备码事件（前端弹出设备码弹窗）
#[gui_macros::emit]
pub(super) fn emit_account_oauth(app: &AppHandle, dto: AccountOAuthDto) {
    let _ = app.emit(listens::ACCOUNT_OAUTH, dto);
}

/// 微软登录阶段事件（state：waiting / xbox / xsts / token / profile / ok / fail）
#[gui_macros::emit]
pub(super) fn emit_account_oauth_state(app: &AppHandle, dto: AccountOAuthStateDto) {
    let _ = app.emit(listens::ACCOUNT_OAUTH_STATE, dto);
}

/// 发送微软登录阶段事件（包装：按 state + message 组装 DTO）
pub(super) fn emit_oauth_state(app: &AppHandle, state: &str, message: Option<String>) {
    emit_account_oauth_state(
        app,
        AccountOAuthStateDto {
            state: state.to_string(),
            message,
        },
    );
}

/// OAuth 步骤失败：结束本次登录并向前端发送失败状态
pub(super) fn oauth_fail(app: &AppHandle, err: impl Display) -> String {
    *OAUTH_NOW.write().unwrap() = None;
    let msg = err.to_string();
    emit_oauth_state(app, "fail", Some(msg.clone()));
    msg
}

/// 微软账户登录：设备码授权 → Xbox → XSTS → Minecraft Token → Profile → 保存账户
///
/// 各阶段通过 [`emit_oauth_state`] 通知前端展示进度，
/// 用户可经 `account_cancel_oauth` 中断等待。
pub(super) async fn microsoft_login(app: &AppHandle) -> Result<AccountStoreDto, String> {
    // 取设备码失败（网络不通等）也要走 oauth_fail：让前端弹出失败提示而不是无响应
    let code = match oauth::get_code().await {
        Ok(code) => code,
        Err(err) => return Err(oauth_fail(app, err)),
    };

    let cancel = CancellationToken::new();
    *OAUTH_NOW.write().unwrap() = Some(cancel.clone());

    let dto = AccountOAuthDto {
        code: code.code.clone(),
        url: format!("{}?otc={}", code.url, code.code),
    };
    emit_oauth_state(app, "waiting", None);
    emit_account_oauth(app, dto);

    let oauth_res = match oauth::run_get_code(&code, &cancel).await {
        Ok(ok) => ok,
        Err(err) => return Err(oauth_fail(app, err)),
    };

    emit_oauth_state(app, "xbox", None);
    let xbox = match oauth::get_xbox(&oauth_res.access_token).await {
        Ok(ok) => ok,
        Err(err) => return Err(oauth_fail(app, err)),
    };

    emit_oauth_state(app, "xsts", None);
    let xsts = match oauth::get_xsts(&xbox.xbl_token).await {
        Ok(ok) => ok,
        Err(err) => return Err(oauth_fail(app, err)),
    };

    emit_oauth_state(app, "token", None);
    let (token, expires_in) =
        match mojang_api::get_minecraft_token(&xsts.xbl_uhs, &xsts.xbl_token).await {
            Ok(ok) => ok,
            Err(err) => return Err(oauth_fail(app, err)),
        };

    emit_oauth_state(app, "profile", None);
    let profile = match mojang_api::get_minecraft_profile(&token).await {
        Ok(ok) => ok,
        Err(err) => return Err(oauth_fail(app, err)),
    };

    *OAUTH_NOW.write().unwrap() = None;

    // Profile 返回的 UUID 无连字符，转为 Minecraft 常规带连字符格式
    let uuid = Uuid::parse_str(&profile.id)
        .map(|u| u.to_string())
        .unwrap_or(profile.id);

    let mut login = LoginObj::new(profile.name, uuid, token, String::new());
    login.auth_type = AuthType::OAuth;
    login.text1 = Some(oauth_res.refresh_token);
    login.set_expire_in(expires_in);
    login.save();

    if auths::get_current().is_none() {
        auths::set_current(Some(login.get_key()));
    }

    emit_account_change(app);
    emit_oauth_state(app, "ok", None);
    Ok(AccountStoreDto::from_login(&login))
}

/// 查询正版档案，令牌失效（401 / 403）时自动刷新令牌重试一次
///
/// 微软访问令牌约 24 小时过期，长期挂机后查纹理 / 装备会整体 401。
/// 刷新走完整 OAuth 链并把新令牌写回账户存储（内存 + accounts.json），
/// 后续请求直接用新令牌；令牌不展示给前端、账户可见信息不变，无需广播 account-change。
pub(super) async fn get_oauth_profile(auth: &mut LoginObj) -> CoreResult<MinecraftProfileObj> {
    let first = mojang_api::get_minecraft_profile(&auth.access_token).await;
    let expired = matches!(
        &first,
        Err(ErrorType::HttpError(data)) if matches!(data.status, Some(401) | Some(403))
    );
    if !expired {
        return first;
    }
    auth.refresh(CancellationToken::new()).await?;
    auth.save();
    mojang_api::get_minecraft_profile(&auth.access_token).await
}

/// 取消进行中的微软登录轮询
#[gui_macros::ipc_group("account")]
#[tauri::command]
pub fn account_cancel_oauth() {
    if let Some(cancel) = OAUTH_NOW.write().unwrap().take() {
        cancel.cancel();
    }
}
