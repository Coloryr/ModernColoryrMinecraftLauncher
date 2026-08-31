//! 主窗口：新闻 / 游戏事件模型 + 实例数据存储 + IPC 命令（窗口按钮调用的方法）
//!
//! 数据从 Rust 侧获取：实例 / 分组 / Java / 版本存储在 `MainWindowModel`，
//! 持久化到应用数据目录的 `main_data.json`；前端通过 IPC 调用本模块命令。

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};

use crate::dtos::{ExitEvent, InstanceChangeEvent, InstancePatch, LogEvent, StateEvent};
use crate::models::{InstanceArgs, InstanceInfo, JavaInfo, VersionInfo};

use crate::window_manager::create_window;

// ================= 数据存储 =================

/// 主窗口数据存储：实例 / 启动参数 / 分组 / 运行状态 / 日志
pub struct MainWindowModel {
    data_path: PathBuf,
    pub instances: Vec<InstanceInfo>,
    pub args: HashMap<String, InstanceArgs>,
    pub extra_groups: Vec<String>,
    pub group_order: Vec<String>,
    pub running: HashSet<String>,
    pub logs: HashMap<String, Vec<String>>,
    pub javas: Vec<JavaInfo>,
    pub versions: Vec<VersionInfo>,
}

impl MainWindowModel {
    fn data_path(app: &AppHandle) -> Result<PathBuf, String> {
        let dir = app
            .path()
            .app_data_dir()
            .map_err(|e| format!("无法获取应用数据目录: {e}"))?;
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        Ok(dir.join("main_data.json"))
    }

    /// 初始化：从磁盘加载（无文件则为空存储），并做环境检测
    pub fn init(app: &AppHandle) -> Self {
        let data_path = Self::data_path(app).unwrap_or_else(|_| PathBuf::from("main_data.json"));
        let mut store = Self {
            data_path,
            instances: Vec::new(),
            args: HashMap::new(),
            extra_groups: Vec::new(),
            group_order: Vec::new(),
            running: HashSet::new(),
            logs: HashMap::new(),
            javas: detect_javas(),
            versions: Vec::new(),
        };
        store.load();
        store
    }

    fn load(&mut self) {
        if let Ok(text) = std::fs::read_to_string(&self.data_path) {
            if let Ok(data) = serde_json::from_str::<PersistedData>(&text) {
                self.instances = data.instances;
                self.args = data.args;
                self.extra_groups = data.extra_groups;
                self.group_order = data.group_order;
            }
        }
    }

    pub fn save(&self) {
        let data = PersistedData {
            instances: self.instances.clone(),
            args: self.args.clone(),
            extra_groups: self.extra_groups.clone(),
            group_order: self.group_order.clone(),
        };
        if let Ok(text) = serde_json::to_string_pretty(&data) {
            let _ = std::fs::write(&self.data_path, text);
        }
    }

    /// 全部分组（实例分组 + 手动空分组），保持顺序
    fn all_groups(&self) -> Vec<String> {
        let mut names = Vec::new();
        for inst in &self.instances {
            if let Some(g) = &inst.group {
                if !names.contains(g) {
                    names.push(g.clone());
                }
            }
        }
        for g in &self.extra_groups {
            if !names.contains(g) {
                names.push(g.clone());
            }
        }
        // 按持久顺序排序，新出现的排在末尾
        let ordered: Vec<String> = self
            .group_order
            .iter()
            .filter(|n| names.contains(n))
            .cloned()
            .collect();
        let rest: Vec<String> = names
            .into_iter()
            .filter(|n| !ordered.contains(n))
            .collect();
        [ordered, rest].concat()
    }
}

/// 持久化数据（JSON）
#[derive(Serialize, Deserialize)]
struct PersistedData {
    instances: Vec<InstanceInfo>,
    args: HashMap<String, InstanceArgs>,
    extra_groups: Vec<String>,
    group_order: Vec<String>,
}

/// 检测系统 Java（扫描常见安装目录 + JAVA_HOME）
fn detect_javas() -> Vec<JavaInfo> {
    let mut list = Vec::new();
    // JAVA_HOME 单独处理（避免闭包借用冲突）
    if let Ok(home) = std::env::var("JAVA_HOME") {
        let exe = std::path::Path::new(&home).join("bin").join("java.exe");
        if exe.exists() {
            list.push(JavaInfo {
                name: "JAVA_HOME".to_string(),
                path: exe.to_string_lossy().to_string(),
                version: String::new(),
                major: 0,
                java_type: String::new(),
                arch: String::new(),
            });
        }
    }
    let mut push_dir = |dir: &str| {
        let base = std::path::Path::new(dir);
        if let Ok(entries) = std::fs::read_dir(base) {
            for entry in entries.flatten() {
                let exe = entry.path().join("bin").join("java.exe");
                if exe.exists() {
                    list.push(JavaInfo {
                        name: entry.file_name().to_string_lossy().to_string(),
                        path: exe.to_string_lossy().to_string(),
                        version: String::new(),
                        major: 0,
                        java_type: String::new(),
                        arch: String::new(),
                    });
                }
            }
        }
    };
    for dir in [
        "C:\\Program Files\\Java",
        "C:\\Program Files\\Eclipse Adoptium",
        "C:\\Program Files\\Microsoft",
        "C:\\Program Files\\Zulu",
        "C:\\Program Files (x86)\\Java",
    ] {
        push_dir(dir);
    }
    list
}

fn now_time() -> String {
    let d = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = d.as_secs() % 86400;
    let ms = d.subsec_millis();
    format!(
        "{:02}:{:02}:{:02}.{:03}",
        secs / 3600,
        (secs % 3600) / 60,
        secs % 60,
        ms
    )
}

// ================= IPC 命令 =================

/// 初始化核心：返回数据目录（前端启动时调用）
#[tauri::command]
pub fn main_init_core(app: AppHandle, local_dir: Option<String>, user_name: String) -> Result<String, String> {
    let dir = MainWindowModel::data_path(&app)?;
    println!("[init_core] local_dir={local_dir:?} user={user_name} data_dir={}", dir.display());
    Ok(dir.to_string_lossy().to_string())
}

/// 获取实例列表（合并运行状态）
#[tauri::command]
pub fn main_get_instances(state: State<'_, Mutex<MainWindowModel>>) -> Vec<InstanceInfo> {
    let store = state.lock().unwrap();
    store
        .instances
        .iter()
        .map(|i| {
            let mut i = i.clone();
            i.running = store.running.contains(&i.uuid);
            i
        })
        .collect()
}

/// 获取分组列表（实例分组 + 手动空分组）
#[tauri::command]
pub fn main_get_groups(state: State<'_, Mutex<MainWindowModel>>) -> Vec<String> {
    state.lock().unwrap().all_groups()
}

/// 获取 Java 列表（系统检测）
#[tauri::command]
pub fn main_get_java_list(state: State<'_, Mutex<MainWindowModel>>) -> Vec<JavaInfo> {
    state.lock().unwrap().javas.clone()
}

/// 获取游戏版本列表（从 Mojang 版本清单拉取，缓存于存储）
#[tauri::command]
pub async fn main_get_versions(
    state: State<'_, Mutex<MainWindowModel>>,
) -> Result<Vec<VersionInfo>, String> {
    {
        let store = state.lock().unwrap();
        if !store.versions.is_empty() {
            return Ok(store.versions.clone());
        }
    }
    let fetched = tauri::async_runtime::spawn_blocking(fetch_versions)
        .await
        .map_err(|e| e.to_string())?;
    let mut store = state.lock().unwrap();
    store.versions = fetched.clone();
    Ok(fetched)
}

/// 从 Mojang 版本清单拉取版本列表（release 优先、版本号降序）；失败返回空
fn fetch_versions() -> Vec<VersionInfo> {
    const MANIFEST: &str = "https://launchermeta.mojang.com/mc/game/version_manifest_v2.json";
    #[derive(serde::Deserialize)]
    struct Manifest {
        versions: Vec<ManifestVersion>,
    }
    #[derive(serde::Deserialize)]
    struct ManifestVersion {
        id: String,
        #[serde(rename = "type")]
        version_type: String,
    }
    let fetch = || -> Result<String, String> {
        let resp = ureq::get(MANIFEST)
            .timeout(Duration::from_secs(4))
            .call()
            .map_err(|e| e.to_string())?;
        resp.into_string().map_err(|e| e.to_string())
    };
    let Ok(body) = fetch() else {
        return Vec::new();
    };
    let Ok(manifest) = serde_json::from_str::<Manifest>(&body) else {
        return Vec::new();
    };
    let mut list: Vec<VersionInfo> = manifest
        .versions
        .into_iter()
        .map(|v| VersionInfo { id: v.id, version_type: v.version_type })
        .collect();
    list.sort_by(|a, b| {
        let ra = (a.version_type == "release") as i32;
        let rb = (b.version_type == "release") as i32;
        rb.cmp(&ra).then_with(|| compare_version(&b.id, &a.id))
    });
    list.truncate(80);
    list
}

/// 语义版本比较（"1.21.1" vs "1.20.4"）
fn compare_version(a: &str, b: &str) -> std::cmp::Ordering {
    let pa: Vec<u64> = a
        .trim_start_matches(|c: char| !c.is_ascii_digit())
        .split('.')
        .filter_map(|n| n.parse::<u64>().ok())
        .collect();
    let pb: Vec<u64> = b
        .trim_start_matches(|c: char| !c.is_ascii_digit())
        .split('.')
        .filter_map(|n| n.parse::<u64>().ok())
        .collect();
    for i in 0..pa.len().max(pb.len()) {
        let x = pa.get(i).copied().unwrap_or(0);
        let y = pb.get(i).copied().unwrap_or(0);
        if x != y {
            return x.cmp(&y);
        }
    }
    std::cmp::Ordering::Equal
}

fn emit_instance_change(app: &AppHandle, r#type: &str) {
    let _ = app.emit("instance-change", InstanceChangeEvent { r#type: r#type.into() });
}

/// 添加空分组
#[tauri::command]
pub fn main_add_group(app: AppHandle, state: State<'_, Mutex<MainWindowModel>>, name: String) -> Result<bool, String> {
    let n = name.trim().to_string();
    if n.is_empty() {
        return Ok(false);
    }
    let mut store = state.lock().unwrap();
    let exists = store.all_groups().contains(&n);
    if exists {
        return Ok(false);
    }
    store.extra_groups.push(n);
    store.save();
    emit_instance_change(&app, "group");
    Ok(true)
}

/// 删除空分组
#[tauri::command]
pub fn main_remove_group(app: AppHandle, state: State<'_, Mutex<MainWindowModel>>, name: String) -> Result<bool, String> {
    let mut store = state.lock().unwrap();
    let before = store.extra_groups.len();
    store.extra_groups.retain(|g| g != &name);
    store.group_order.retain(|g| g != &name);
    let ok = store.extra_groups.len() < before;
    if ok {
        store.save();
        emit_instance_change(&app, "group");
    }
    Ok(ok)
}

/// 调整分组显示顺序
#[tauri::command]
pub fn main_move_group(app: AppHandle, state: State<'_, Mutex<MainWindowModel>>, name: String, index: i64) -> Result<bool, String> {
    let mut store = state.lock().unwrap();
    let mut list = store.all_groups();
    let from = list.iter().position(|g| g == &name).ok_or_else(|| "分组不存在".to_string())?;
    list.remove(from);
    let at = (index as usize).min(list.len());
    list.insert(at, name.clone());
    store.group_order = list;
    store.save();
    emit_instance_change(&app, "group");
    Ok(true)
}

/// 创建实例
#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub fn main_create_instance(
    app: AppHandle,
    state: State<'_, Mutex<MainWindowModel>>,
    name: String,
    version: String,
    loader: Option<String>,
    loader_version: Option<String>,
    group: Option<String>,
    modpack_type: Option<String>,
    source: Option<String>,
) -> Result<InstanceInfo, String> {
    let uuid = format!("mcml-{}", uuid_short());
    let dir = name.clone();
    let inst = InstanceInfo {
        uuid: uuid.clone(),
        name,
        group,
        version,
        version_type: Some("release".into()),
        loader: loader.unwrap_or_else(|| "原版".into()),
        loader_version,
        dir,
        running: false,
        modpack_type,
        pid: None,
        fid: None,
        lang: None,
        log_encoding: None,
        source,
    };
    let mut store = state.lock().unwrap();
    store.instances.insert(0, inst.clone());
    store.args.insert(uuid, InstanceArgs::default());
    store.save();
    drop(store);
    emit_instance_change(&app, "add");
    Ok(inst)
}

/// 重命名实例
#[tauri::command]
pub fn main_rename_instance(app: AppHandle, state: State<'_, Mutex<MainWindowModel>>, uuid: String, name: String) -> Result<bool, String> {
    let n = name.trim().to_string();
    if n.is_empty() {
        return Ok(false);
    }
    let mut store = state.lock().unwrap();
    let Some(inst) = store.instances.iter_mut().find(|i| i.uuid == uuid) else {
        return Ok(false);
    };
    inst.name = n.clone();
    inst.dir = n;
    store.save();
    emit_instance_change(&app, "edit");
    Ok(true)
}

/// 更新实例元信息（补丁式）
#[tauri::command]
pub fn main_update_instance(app: AppHandle, state: State<'_, Mutex<MainWindowModel>>, uuid: String, patch: InstancePatch) -> Result<bool, String> {
    let mut store = state.lock().unwrap();
    let Some(inst) = store.instances.iter_mut().find(|i| i.uuid == uuid) else {
        return Ok(false);
    };
    if let Some(v) = patch.group {
        inst.group = v;
    }
    if let Some(v) = patch.name {
        inst.name = v;
    }
    if let Some(v) = patch.version {
        inst.version = v;
    }
    if let Some(v) = patch.version_type {
        inst.version_type = Some(v);
    }
    if let Some(v) = patch.loader {
        inst.loader = v;
    }
    if let Some(v) = patch.loader_version {
        inst.loader_version = Some(v);
    }
    if let Some(v) = patch.modpack_type {
        inst.modpack_type = v;
    }
    if let Some(v) = patch.pid {
        inst.pid = v;
    }
    if let Some(v) = patch.fid {
        inst.fid = v;
    }
    if let Some(v) = patch.lang {
        inst.lang = Some(v);
    }
    if let Some(v) = patch.log_encoding {
        inst.log_encoding = Some(v);
    }
    store.save();
    emit_instance_change(&app, "edit");
    Ok(true)
}

/// 删除实例
#[tauri::command]
pub fn main_delete_instance(app: AppHandle, state: State<'_, Mutex<MainWindowModel>>, uuid: String) -> Result<bool, String> {
    let mut store = state.lock().unwrap();
    let before = store.instances.len();
    store.instances.retain(|i| i.uuid != uuid);
    store.args.remove(&uuid);
    store.running.remove(&uuid);
    store.logs.remove(&uuid);
    let ok = store.instances.len() < before;
    if ok {
        store.save();
        emit_instance_change(&app, "remove");
    }
    Ok(ok)
}

/// 移动实例到 (分组, 组内位置)：支持同组排序与跨组移动
#[tauri::command]
pub fn main_move_instance(app: AppHandle, state: State<'_, Mutex<MainWindowModel>>, uuid: String, group: Option<String>, index: i64) -> Result<bool, String> {
    let mut store = state.lock().unwrap();
    let Some(pos) = store.instances.iter().position(|i| i.uuid == uuid) else {
        return Ok(false);
    };
    let mut inst = store.instances.remove(pos);
    inst.group = group.clone();
    let others: Vec<String> = store
        .instances
        .iter()
        .filter(|i| i.group == group)
        .map(|i| i.uuid.clone())
        .collect();
    let anchor = others.get((index as usize).min(others.len())).cloned();
    let at = match &anchor {
        Some(a) => store.instances.iter().position(|i| &i.uuid == a).unwrap_or(store.instances.len()),
        None => store.instances.len(),
    };
    store.instances.insert(at, inst);
    store.save();
    emit_instance_change(&app, "edit");
    Ok(true)
}

/// 启动游戏（占位：标记运行 + 发事件；接入核心后替换为真实启动）
#[tauri::command]
pub fn main_launch_game(app: AppHandle, state: State<'_, Mutex<MainWindowModel>>, uuid: String, user_name: String) -> Result<(), String> {
    println!("[launch_game] uuid={uuid} user={user_name}");
    {
        let mut store = state.lock().unwrap();
        if store.running.contains(&uuid) {
            return Err("实例已在运行中".to_string());
        }
        store.running.insert(uuid.clone());
        store.save();
    }
    let _ = app.emit(
        "launch-state",
        StateEvent { uuid: uuid.clone(), state: "launching".into() },
    );
    let _ = app.emit(
        "game-log",
        LogEvent { uuid: uuid.clone(), time: now_time(), text: "游戏启动中…".into(), clear: true },
    );

    // 占位：3 秒后发出退出事件（真实启动需接入 mcml-core）
    let app2 = app.clone();
    let uuid2 = uuid.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_secs(3));
        let _ = app2.emit(
            "game-log",
            LogEvent { uuid: uuid2.clone(), time: now_time(), text: "游戏进程已退出".into(), clear: false },
        );
        let _ = app2.emit("game-exit", ExitEvent { uuid: uuid2.clone(), code: 0 });
        let store = app2.state::<Mutex<MainWindowModel>>();
        let mut s = store.lock().unwrap();
        s.running.remove(&uuid2);
        s.save();
    });
    Ok(())
}

/// 停止游戏
#[tauri::command]
pub fn main_stop_game(app: AppHandle, state: State<'_, Mutex<MainWindowModel>>, uuid: String) -> Result<(), String> {
    {
        let mut store = state.lock().unwrap();
        store.running.remove(&uuid);
        store.save();
    }
    let _ = app.emit("game-exit", ExitEvent { uuid, code: 0 });
    Ok(())
}

/// 获取实例日志
#[tauri::command]
pub fn main_get_game_log(state: State<'_, Mutex<MainWindowModel>>, uuid: String) -> Vec<String> {
    state.lock().unwrap().logs.get(&uuid).cloned().unwrap_or_default()
}

/// 获取运行中实例
#[tauri::command]
pub fn main_get_running(state: State<'_, Mutex<MainWindowModel>>) -> Vec<String> {
    state.lock().unwrap().running.iter().cloned().collect()
}

/// 生成短 uuid（无 uuid 依赖时的简单替代）
fn uuid_short() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let n = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos() as u64
        ^ (std::process::id() as u64) << 32;
    format!("{:016x}", n)
}

// ================= 窗口规格 =================

pub const LABEL: &str = "main";
pub const TITLE: &str = "MCML 启动器";
pub const WIDTH: f64 = 1100.0;
pub const HEIGHT: f64 = 720.0;

/// 打开主窗口（由 window_manager 创建，一般不需要）
pub fn open(app: &AppHandle) -> Result<(), String> {
    create_window(app, LABEL, TITLE, WIDTH, HEIGHT)
}
