//! 账户窗口：账户数据 + IPC 命令 + 规格 + 创建操作
//!
//! 账户数据统一由 `mcml_auth::auths`（mcml-auth 全局账户存储）管理，
//! 持久化到核心数据目录的 auth.json；本模块只负责 IPC 转接与窗口规格。
use mcml_auth::{LoginObj, auths};
use tauri::{AppHandle, Emitter};
use uuid::Uuid;

use crate::dtos::account::{AccountStoreDto, AccountStoreViewDto, auth_type_from_str};
use crate::window_manager::create_window;

fn emit_account_change(app: &AppHandle) {
    let _ = app.emit("account-change", ());
}

// ================= IPC 命令 =================

/// 获取账户列表 + 当前账户
#[tauri::command]
pub fn account_get_accounts() -> AccountStoreViewDto {
    AccountStoreViewDto {
        accounts: auths::get_all()
            .iter()
            .map(AccountStoreDto::from_login)
            .collect(),
        current_uuid: auths::get_current(),
    }
}

/// 添加账户（离线 / 皮肤站等由前端传账户类型与名称）
#[tauri::command]
pub fn account_add_account(
    app: AppHandle,
    name: String,
    account_type: String,
) -> Result<AccountStoreDto, String> {
    let n = name.trim().to_string();
    if n.is_empty() {
        return Err("账户名不能为空".to_string());
    }
    let uuid = Uuid::new_v4().to_string();
    let mut login = LoginObj::new(n, uuid.clone(), String::new(), String::new());
    login.auth_type = auth_type_from_str(&account_type);
    if auths::get_current().is_none() {
        auths::set_current(Some(uuid.clone()));
    }
    login.save();
    emit_account_change(&app);
    Ok(AccountStoreDto::from_login(&login))
}

/// 删除账户
#[tauri::command]
pub fn account_remove_account(app: AppHandle, uuid: String) -> Result<bool, String> {
    let Some(login) = auths::get_by_uuid(uuid.clone()) else {
        return Ok(false);
    };
    login.delete();
    if auths::get_current().as_deref() == Some(&uuid) {
        auths::set_current(auths::get_all().first().map(|a| a.uuid.clone()));
    }
    emit_account_change(&app);
    Ok(true)
}

/// 刷新账户 Token（占位：离线占位账户无真实令牌，接入登录后走 LoginObj::refresh）
#[tauri::command]
pub fn account_refresh_account_token(app: AppHandle, uuid: String) -> Result<bool, String> {
    if auths::get_by_uuid(uuid).is_none() {
        return Ok(false);
    }
    emit_account_change(&app);
    Ok(true)
}

/// 设置当前使用账户
#[tauri::command]
pub fn account_set_current_account(app: AppHandle, uuid: String) -> Result<bool, String> {
    if auths::get_by_uuid(uuid.clone()).is_none() {
        return Ok(false);
    }
    auths::set_current(Some(uuid));
    emit_account_change(&app);
    Ok(true)
}

/// 窗口规格（模型）
pub const LABEL: &str = "mcml-account";
pub const TITLE: &str = "账户管理";
pub const WIDTH: f64 = 920.0;
pub const HEIGHT: f64 = 640.0;

/// 打开账户窗口
pub fn open(app: &AppHandle) -> Result<(), String> {
    create_window(app, LABEL, TITLE, WIDTH, HEIGHT)
}
