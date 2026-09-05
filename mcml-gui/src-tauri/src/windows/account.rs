//! 账户窗口
use std::fmt::Display;
use std::sync::RwLock;

use mcml_auth::{AuthType, LoginObj, auths, oauth};
use mcml_net::mojang_api;
use mcml_sys::open_helper;
use tauri::{AppHandle, Emitter};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::dtos::account_dto::{
    AccountOAuthDto, AccountOAuthStateDto, AccountStoreDto, AccountStoreViewDto, auth_type_from_str,
};
use crate::listens;

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

/// 添加账户（离线 / 皮肤站等由前端传账户类型与名称；微软走设备码登录流程）
#[tauri::command]
pub async fn account_add_account(
    app: AppHandle,
    name: String,
    account_type: String,
) -> Result<AccountStoreDto, String> {
    let auth_type = auth_type_from_str(&account_type);
    if auth_type == AuthType::OAuth {
        return microsoft_login(&app).await;
    }

    let n = name.trim().to_string();
    if n.is_empty() {
        return Err("账户名不能为空".to_string());
    }
    let uuid = Uuid::new_v4().to_string();
    let mut login = LoginObj::new(n, uuid.clone(), String::new(), String::new());
    login.auth_type = auth_type;
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
    let code = oauth::get_code().await.map_err(|e| e.to_string())?;

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
