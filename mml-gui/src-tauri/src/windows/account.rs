//! 账户窗口：账户列表 + 微软设备码登录 + Token 刷新
//!
//! 数据由 `mml_auth::auths` 持有（`accounts.json`），这里只做命令与事件转发。

use std::fmt::Display;
use std::sync::RwLock;

use mml_auth::{AuthType, LoginObj, auths, legacy::{authlib_injector, little_skin, nide8}, oauth};
use mml_net::mojang_api;
use mml_sys::open_helper;
use tauri::{AppHandle, Emitter};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::dtos::account_dto::{
    AccountOAuthDto, AccountOAuthStateDto, AccountStoreDto, AccountStoreViewDto, auth_type_from_str,
};
use crate::listens;

/// 进行中的微软登录轮询取消句柄（None = 无登录进行中）
static OAUTH_NOW: RwLock<Option<CancellationToken>> = RwLock::new(None);

/// 账户变更事件（跨窗口同步刷新列表）
#[gui_macros::emit]
fn emit_account_change(app: &AppHandle) {
    let _ = app.emit(listens::ACCOUNT_CHANGE, ());
}

/// 微软登录设备码事件（前端弹出设备码弹窗）
#[gui_macros::emit]
fn emit_account_oauth(app: &AppHandle, dto: AccountOAuthDto) {
    let _ = app.emit(listens::ACCOUNT_OAUTH, dto);
}

/// 微软登录阶段事件（state：waiting / xbox / xsts / token / profile / ok / fail）
#[gui_macros::emit]
fn emit_account_oauth_state(app: &AppHandle, dto: AccountOAuthStateDto) {
    let _ = app.emit(listens::ACCOUNT_OAUTH_STATE, dto);
}

/// 发送微软登录阶段事件（包装：按 state + message 组装 DTO）
fn emit_oauth_state(app: &AppHandle, state: &str, message: Option<String>) {
    emit_account_oauth_state(
        app,
        AccountOAuthStateDto {
            state: state.to_string(),
            message,
        },
    );
}

/// OAuth 步骤失败：结束本次登录并向前端发送失败状态
fn oauth_fail(app: &AppHandle, err: impl Display) -> String {
    *OAUTH_NOW.write().unwrap() = None;
    let msg = err.to_string();
    emit_oauth_state(app, "fail", Some(msg.clone()));
    msg
}

/// 按 UUID 在账户列表中查找账户（auths 以 UUID + 类型为键，此处 UUID 即可唯一定位当前场景）
fn find_by_uuid(uuid: &str) -> Option<LoginObj> {
    auths::get_all().into_iter().find(|a| a.uuid == uuid)
}

/// 获取账户列表 + 当前账户
#[tauri::command]
pub fn account_get_accounts() -> AccountStoreViewDto {
    AccountStoreViewDto {
        accounts: auths::get_all()
            .iter()
            .map(AccountStoreDto::from_login)
            .collect(),
        current_uuid: auths::get_current().map(|k| k.uuid),
    }
}

/// 添加账户（离线 / 皮肤站等由前端传账户类型与凭据；微软走设备码登录流程）
///
/// 皮肤站类（LittleSkin / 外置登录 / 统一通行证）会向对应认证服务器发起真实
/// Yggdrasil 登录，服务器地址由认证层存入 `LoginObj.text1` 供后续刷新与皮肤拉取；
/// 离线账户本地直接创建。
#[tauri::command]
pub async fn account_add_account(
    app: AppHandle,
    name: String,
    account_type: String,
    server: Option<String>,
    password: Option<String>,
) -> Result<AccountStoreDto, String> {
    let auth_type = auth_type_from_str(&account_type);
    if auth_type == AuthType::OAuth {
        return microsoft_login(&app).await;
    }

    let server = server
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let user = name.trim().to_string();
    let password = password.unwrap_or_default();

    let mut login = match auth_type {
        AuthType::LittleSkin | AuthType::SelfLittleSkin => {
            // server 为 None 时使用官方 LittleSkin
            little_skin::authenticate(Uuid::new_v4().to_string(), user, password, server, None).await
        }
        AuthType::AuthlibInjector => {
            let Some(server) = server else {
                return Err("err.serverEmpty".to_string());
            };
            authlib_injector::authenticate(Uuid::new_v4().to_string(), user, password, server, None)
                .await
        }
        AuthType::Nide8 => {
            let Some(server) = server else {
                return Err("err.serverEmpty".to_string());
            };
            nide8::authenticate(Uuid::new_v4().to_string(), user, password, server).await
        }
        _ => {
            // 离线账户：本地生成，无凭据
            if user.is_empty() {
                return Err("err.nameEmpty".to_string());
            }
            Ok(LoginObj::new(
                user,
                Uuid::new_v4().to_string(),
                String::new(),
                String::new(),
            ))
        }
    }
    .map_err(|e| e.to_string())?;

    // 皮肤站类的 auth_type 与 text1（服务器地址）由认证层设置，此处不覆盖
    if auth_type == AuthType::Offline {
        login.auth_type = AuthType::Offline;
    }
    if auths::get_current().is_none() {
        auths::set_current(Some(login.get_key()));
    }
    login.save();
    emit_account_change(&app);
    Ok(AccountStoreDto::from_login(&login))
}

/// 微软账户登录：设备码授权 → Xbox → XSTS → Minecraft Token → Profile → 保存账户
///
/// 各阶段通过 [`emit_oauth_state`] 通知前端展示进度，
/// 用户可经 `account_cancel_oauth` 中断等待。
async fn microsoft_login(app: &AppHandle) -> Result<AccountStoreDto, String> {
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
    let token = match mojang_api::get_minecraft_token(&xsts.xbl_uhs, &xsts.xbl_token).await {
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
    login.save();

    if auths::get_current().is_none() {
        auths::set_current(Some(login.get_key()));
    }

    emit_account_change(app);
    emit_oauth_state(app, "ok", None);
    Ok(AccountStoreDto::from_login(&login))
}

/// 取消进行中的微软登录轮询
#[tauri::command]
pub fn account_cancel_oauth() {
    if let Some(cancel) = OAUTH_NOW.write().unwrap().take() {
        cancel.cancel();
    }
}

/// 用系统浏览器打开微软授权页面
#[tauri::command]
pub fn account_open_browser(url: String) {
    open_helper::open_url(&url);
}

/// 删除账户
#[tauri::command]
pub fn account_remove_account(app: AppHandle, uuid: String) -> Result<bool, String> {
    let Some(login) = find_by_uuid(&uuid) else {
        return Ok(false);
    };
    login.delete();
    if auths::get_current().map(|k| k.uuid) == Some(uuid) {
        auths::set_current(auths::get_all().first().map(|a| a.get_key()));
    }
    emit_account_change(&app);
    Ok(true)
}

/// 刷新账户 Token（按认证类型走 LoginObj::refresh，微软账户走完整 OAuth 刷新链）
#[tauri::command]
pub async fn account_refresh_account_token(app: AppHandle, uuid: String) -> Result<bool, String> {
    let Some(mut login) = find_by_uuid(&uuid) else {
        return Ok(false);
    };
    login
        .refresh(CancellationToken::new())
        .await
        .map_err(|e| e.to_string())?;
    login.save();
    emit_account_change(&app);
    Ok(true)
}

/// 设置当前使用账户
#[tauri::command]
pub fn account_set_current_account(app: AppHandle, uuid: String) -> Result<bool, String> {
    let Some(login) = find_by_uuid(&uuid) else {
        return Ok(false);
    };
    auths::set_current(Some(login.get_key()));
    emit_account_change(&app);
    Ok(true)
}

/// 编辑离线账户的名字与 UUID
///
/// 离线账户没有服务器凭据，改名/改 UUID 就是改账户本身（存储键随之更新）。
#[tauri::command]
pub fn account_edit_offline(
    app: AppHandle,
    uuid: String,
    new_name: String,
    new_uuid: String,
) -> Result<(), String> {
    let mut login = auths::get(&uuid, AuthType::Offline).ok_or("err.accountMissing")?;

    let new_name = new_name.trim().to_string();
    let new_uuid = new_uuid.trim().to_string();
    if new_name.is_empty() {
        return Err("err.nameEmpty".to_string());
    }
    if new_uuid.is_empty() {
        return Err("err.uuidEmpty".to_string());
    }
    // 新 UUID 不能与其它离线账户冲突（同键会静默覆盖）
    if new_uuid != uuid && auths::get(&new_uuid, AuthType::Offline).is_some() {
        return Err("err.uuidConflict".to_string());
    }

    let was_current = auths::get_current()
        .map(|k| k.uuid == uuid && k.auth_type == AuthType::Offline)
        .unwrap_or(false);

    login.delete();
    login.user_name = new_name;
    login.uuid = new_uuid;
    login.save();

    if was_current {
        auths::set_current(Some(login.get_key()));
    }
    emit_account_change(&app);
    Ok(())
}
