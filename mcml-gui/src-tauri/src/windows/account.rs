//! 账户窗口：账户模型 + 账户存储 + IPC 命令 + 规格 + 创建操作
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};

use super::create;

/// 账户信息（窗口专属模型）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub uuid: String,
    pub name: String,
    /// 账户类型：offline（离线）/ microsoft（微软）/ littleskin / authlib / nide8 等
    #[serde(rename = "type")]
    pub account_type: String,
    /// 头像渐变起点色（CSS）
    pub avatar_color: String,
    /// 皮肤主色（SVG 生成用）
    pub skin: String,
    /// 最后登录时间
    pub last_login: String,
    /// Token 状态：valid / expired
    pub token_status: String,
}

impl Account {
    /// Token 是否有效
    pub fn is_valid(&self) -> bool {
        self.token_status == "valid"
    }

    /// Token 是否过期
    pub fn is_expired(&self) -> bool {
        self.token_status == "expired"
    }

    /// 是否为微软账户
    pub fn is_microsoft(&self) -> bool {
        self.account_type == "microsoft"
    }

    /// 是否为离线账户
    pub fn is_offline(&self) -> bool {
        self.account_type == "offline"
    }
}

/// 账户存储（持久化到 accounts.json）
pub struct AccountStore {
    data_path: PathBuf,
    pub accounts: Vec<Account>,
    pub current_uuid: Option<String>,
}

/// 账户列表视图（IPC 返回 / 磁盘持久化）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountStoreView {
    pub accounts: Vec<Account>,
    pub current_uuid: Option<String>,
}

impl AccountStore {
    fn data_path(app: &AppHandle) -> Result<PathBuf, String> {
        let dir = app
            .path()
            .app_data_dir()
            .map_err(|e| format!("无法获取应用数据目录: {e}"))?;
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        Ok(dir.join("accounts.json"))
    }

    /// 初始化：从磁盘加载
    pub fn init(app: &AppHandle) -> Self {
        let data_path = Self::data_path(app).unwrap_or_else(|_| PathBuf::from("accounts.json"));
        let mut store = Self {
            data_path,
            accounts: Vec::new(),
            current_uuid: None,
        };
        store.load();
        store
    }

    fn load(&mut self) {
        if let Ok(text) = std::fs::read_to_string(&self.data_path) {
            if let Ok(data) = serde_json::from_str::<AccountStoreView>(&text) {
                self.accounts = data.accounts;
                self.current_uuid = data.current_uuid;
            }
        }
    }

    pub fn save(&self) {
        let data = AccountStoreView {
            accounts: self.accounts.clone(),
            current_uuid: self.current_uuid.clone(),
        };
        if let Ok(text) = serde_json::to_string_pretty(&data) {
            let _ = std::fs::write(&self.data_path, text);
        }
    }
}

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
    let h = uuid.bytes().fold(0u32, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u32));
    let (c1, c2) = P[(h as usize) % P.len()];
    (c1.to_string(), c2.to_string())
}

// ================= IPC 命令 =================

/// 获取账户列表 + 当前账户
#[tauri::command]
pub fn get_accounts(state: State<'_, Mutex<AccountStore>>) -> AccountStoreView {
    let s = state.lock().unwrap();
    AccountStoreView {
        accounts: s.accounts.clone(),
        current_uuid: s.current_uuid.clone(),
    }
}

/// 添加账户（离线 / 皮肤站等由前端传账户类型与名称）
#[tauri::command]
pub fn add_account(
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
pub fn remove_account(app: AppHandle, state: State<'_, Mutex<AccountStore>>, uuid: String) -> Result<bool, String> {
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
pub fn refresh_account_token(app: AppHandle, state: State<'_, Mutex<AccountStore>>, uuid: String) -> Result<bool, String> {
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
pub fn set_current_account(app: AppHandle, state: State<'_, Mutex<AccountStore>>, uuid: String) -> Result<bool, String> {
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
    create(app, LABEL, TITLE, WIDTH, HEIGHT)
}
