//! 账户窗口：账户模型 + 账户存储 + IPC 命令 + 规格 + 创建操作
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use tauri::{AppHandle, Emitter, State};

use crate::window_manager::create_window;

fn emit_account_change(app: &AppHandle) {
    let _ = app.emit("account-change", ());
}

/// 生成短 uuid
fn gen_uuid() -> String {
    let n = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos() as u64
        ^ (std::process::id() as u64) << 32;
    format!("acc-{:016x}", n)
}

/// 按 uuid 哈希取配色
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

// ================= IPC 命令 =================

/// 获取账户列表 + 当前账户
#[tauri::command]
pub fn account_get_accounts(state: State<'_, Mutex<AccountStore>>) -> AccountStoreDto {
    let s = state.lock().unwrap();
    AccountStoreDto {
        accounts: s.accounts.clone(),
        current_uuid: s.current_uuid.clone(),
    }
}

/// 添加账户（离线 / 皮肤站等由前端传账户类型与名称）
#[tauri::command]
pub fn account_add_account(
    app: AppHandle,
    state: State<'_, Mutex<AccountStore>>,
    name: String,
    account_type: String,
) -> Result<Account, String> {
    let n = name.trim().to_string();
    if n.is_empty() {
        return Err("账户名不能为空".to_string());
    }
    let uuid = gen_uuid();
    let (c1, c2) = palette(&uuid);
    let acc = Account {
        uuid: uuid.clone(),
        name: n,
        account_type,
        avatar_color: format!("linear-gradient(135deg, {c1}, {c2})"),
        skin: c1,
        last_login: "刚刚".into(),
        token_status: "valid".into(),
    };
    let mut s = state.lock().unwrap();
    if s.current_uuid.is_none() {
        s.current_uuid = Some(uuid);
    }
    s.accounts.push(acc.clone());
    s.save();
    emit_account_change(&app);
    Ok(acc)
}

/// 删除账户
#[tauri::command]
pub fn account_remove_account(
    app: AppHandle,
    state: State<'_, Mutex<AccountStore>>,
    uuid: String,
) -> Result<bool, String> {
    let mut s = state.lock().unwrap();
    let before = s.accounts.len();
    s.accounts.retain(|a| a.uuid != uuid);
    if s.current_uuid.as_deref() == Some(&uuid) {
        s.current_uuid = s.accounts.first().map(|a| a.uuid.clone());
    }
    let ok = s.accounts.len() < before;
    if ok {
        s.save();
        emit_account_change(&app);
    }
    Ok(ok)
}

/// 刷新账户 Token（置为有效）
#[tauri::command]
pub fn account_refresh_account_token(
    app: AppHandle,
    state: State<'_, Mutex<AccountStore>>,
    uuid: String,
) -> Result<bool, String> {
    let mut s = state.lock().unwrap();
    let Some(acc) = s.accounts.iter_mut().find(|a| a.uuid == uuid) else {
        return Ok(false);
    };
    acc.token_status = "valid".into();
    s.save();
    emit_account_change(&app);
    Ok(true)
}

/// 设置当前使用账户
#[tauri::command]
pub fn account_set_current_account(
    app: AppHandle,
    state: State<'_, Mutex<AccountStore>>,
    uuid: String,
) -> Result<bool, String> {
    let mut s = state.lock().unwrap();
    if !s.accounts.iter().any(|a| a.uuid == uuid) {
        return Ok(false);
    }
    s.current_uuid = Some(uuid);
    s.save();
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
