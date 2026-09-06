//! 主窗口：新闻 / 游戏事件模型 + 实例数据存储 + IPC 命令（窗口按钮调用的方法）
//!
//! 数据从 Rust 侧获取：实例 / 分组 / Java / 版本存储在 `MainWindowModel`，
//! 持久化到应用数据目录的 `main_data.json`；前端通过 IPC 调用本模块命令。

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, LazyLock, Mutex, RwLock};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, WebviewWindow};

use crate::dtos::main_dto::{LoadState, NewsItem};
use crate::dtos::{ErrorEvent, ExitEvent, InstanceChangeEvent, InstancePatch, LogEvent, StateEvent};
use crate::listens;
use crate::models::{InstanceArgs, InstanceInfo, JavaInfo, VersionInfo};
use mcml_game::launcher::instance_setting_obj::InstanceSettingObj;
use mcml_game::loader::LoaderType;
use mcml_game::mojang::VersionType;
use mcml_game::launcher::{LogEncoding, ModPackType};
use uuid::Uuid;

/// 核心加载完成事件（ok：加载成功；error：失败信息，前端据此显示错误页）
#[gui_macros::emit]
pub fn emit_load_done(app: &AppHandle, data: Option<String>) {
    let _ = app.emit(listens::LOAD_DONE, LoadState {
        ok: data.is_none(),
        error: data
    });
}

/// 主窗口数据存储：实例 / 启动参数 / 分组 / 运行状态 / 日志
pub struct MainWindowModel {
    pub instances: Vec<InstanceInfo>,
    pub args: HashMap<String, InstanceArgs>,
    pub extra_groups: Vec<String>,
    pub group_order: Vec<String>,
    pub running: HashSet<String>,
    pub logs: HashMap<String, Vec<String>>,
    pub javas: Vec<JavaInfo>,
}

impl MainWindowModel {
    pub fn init(app: &AppHandle) -> Self {
        Self {
            instances: Vec::new(),
            args: HashMap::new(),
            extra_groups: Vec::new(),
            group_order: Vec::new(),
            running: HashSet::new(),
            logs: HashMap::new(),
            javas: detect_javas(),
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

/// 取主窗口模型（模型跟随主窗口生命周期，开窗创建、关窗销毁，见 window_manager）
///
/// 这些命令只会由主窗口 webview 调用；模型缺失属异常情形（主窗口未创建），
/// 返回 Err / 空列表由调用方降级，不 panic。
fn model(window: &WebviewWindow) -> Result<Arc<Mutex<MainWindowModel>>, String> {
    crate::window_manager::window_model(window)
        .ok_or_else(|| "主窗口模型未初始化".to_string())
}

/// 获取实例列表（mcml-game::get_instances 对接，合并运行状态）
#[tauri::command]
pub fn main_get_instances() -> Vec<InstanceInfo> {
    mcml_game::get_instances()
        .into_iter()
        .map(|inst| {
            let inst = inst.read().unwrap();
            InstanceInfo {
                uuid: inst.uuid.to_string(),
                name: inst.name.clone(),
                group: inst.group.clone(),
                version: inst.version.clone(),
                version_type: Some(version_type_name(inst.game_type).to_string()),
                loader: loader_name(inst.loader).to_string(),
                loader_version: inst.loader_version.clone(),
                dir: inst.dir.clone(),
                running: mcml_game::is_running(&inst.uuid),
                modpack_type: modpack_type_name(&inst),
                pid: inst.pid.clone(),
                fid: inst.fid.clone(),
                lang: None,
                log_encoding: Some(
                    if matches!(inst.encoding, LogEncoding::GBK) { "gbk" } else { "utf8" }.to_string(),
                ),
                source: None,
            }
        })
        .collect()
}

/// VersionType -> 前端版本类型名
fn version_type_name(t: VersionType) -> &'static str {
    match t {
        VersionType::Release => "release",
        VersionType::Snapshot => "snapshot",
        _ => "other",
    }
}

/// LoaderType -> 前端加载器名（与 InstanceMetaPanel 的 LOADERS 一致）
fn loader_name(loader: LoaderType) -> &'static str {
    match loader {
        LoaderType::Normal => "原版",
        LoaderType::Forge => "Forge",
        LoaderType::Fabric => "Fabric",
        LoaderType::Quilt => "Quilt",
        LoaderType::NeoForge => "NeoForge",
        LoaderType::OptiFine => "OptiFine",
        LoaderType::LiteLoader => "LiteLoader",
        LoaderType::Custom => "自定义",
    }
}

/// 整合包平台名（非整合包返回 None）
fn modpack_type_name(inst: &InstanceSettingObj) -> Option<String> {
    if !inst.is_modpack {
        return None;
    }
    Some(
        match inst.modpack_type {
            ModPackType::CurseForge => "CurseForge",
            ModPackType::Modrinth => "Modrinth",
            ModPackType::McMod => "McMod",
            ModPackType::ServerPack => "ServerPack",
            ModPackType::None => "本地",
        }
        .to_string(),
    )
}

/// 获取分组列表（mcml-game::get_group_keys 对接）
#[tauri::command]
pub fn main_get_groups() -> Vec<String> {
    mcml_game::get_group_keys()
}

/// 获取 Java 列表（系统检测）
#[tauri::command]
pub fn main_get_java_list(window: WebviewWindow) -> Vec<JavaInfo> {
    let Ok(store) = model(&window) else {
        eprintln!("[main_get_java_list] 主窗口模型未初始化");
        return Vec::new();
    };
    store.lock().unwrap().javas.clone()
}

/// 版本列表缓存（进程级：主窗口 / 添加实例窗口共用，不挂在窗口模型上）
static VERSIONS_CACHE: LazyLock<RwLock<Vec<VersionInfo>>> =
    LazyLock::new(|| RwLock::new(Vec::new()));

/// 获取游戏版本列表（从 mcml-core 拉取版本清单，缓存于进程）
#[tauri::command]
pub async fn main_get_versions() -> Result<Vec<VersionInfo>, String> {
    {
        let cache = VERSIONS_CACHE.read().unwrap();
        if !cache.is_empty() {
            return Ok(cache.clone());
        }
    }
    // 核心加载未完成（mcml_net::init 未跑）时 HTTP 客户端不可用，返回空列表
    if !mcml_net::is_init() {
        return Ok(Vec::new());
    }
    let fetched = fetch_versions().await;
    *VERSIONS_CACHE.write().unwrap() = fetched.clone();
    Ok(fetched)
}

/// 强制刷新版本列表（清空缓存重新从版本清单拉取）
#[tauri::command]
pub async fn main_refresh_versions() -> Result<Vec<VersionInfo>, String> {
    VERSIONS_CACHE.write().unwrap().clear();
    main_get_versions().await
}

/// 获取 Minecraft 官方新闻（Mojang 新闻接口，按页拉取；核心加载完成后可调用）
#[tauri::command]
pub async fn main_get_news(page: Option<u32>) -> Result<Vec<NewsItem>, String> {
    // 核心加载未完成（mcml_net::init 未跑）时返回空列表，前端稍后会重新拉取
    if !mcml_net::is_init() {
        return Ok(Vec::new());
    }
    let page = page.unwrap_or(1);
    let news = mcml_net::mojang_api::get_minecraft_news(page)
        .await
        .map_err(|err| err.to_string())?;
    Ok(news
        .article_grid
        .into_iter()
        .enumerate()
        .map(|(i, grid)| {
            // 相对路径补全域名，否则 webview 加载不到图片
            let full = |path: String| {
                if path.starts_with("http") {
                    path
                } else {
                    format!("https://www.minecraft.net{path}")
                }
            };
            NewsItem {
                id: i as i64,
                title: grid.default_tile.title,
                date: grid.default_tile.sub_header,
                tag: grid.primary_category,
                image: full(grid.default_tile.image.image_url),
                url: full(grid.article_url),
            }
        })
        .collect())
}

/// 用系统浏览器打开网址（新闻原文跳转等）
#[tauri::command]
pub fn main_open_url(url: String) {
    mcml_sys::open_helper::open_url(&url);
}

/// 从 mcml-core 拉取版本清单（按配置源：官方 / BMCLAPI），
/// release 优先、版本号降序；失败返回空
async fn fetch_versions() -> Vec<VersionInfo> {
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
    let Ok(bytes) = mcml_net::mojang_api::get_versions(None).await else {
        return Vec::new();
    };
    let Ok(manifest) = serde_json::from_slice::<Manifest>(&bytes) else {
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
    // 不截断：快照 / 旧版类型也要有数据，否则切换版本类型后过滤结果为空
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

/// 实例数据变更事件（type：add / edit / remove / group）
#[gui_macros::emit]
pub fn emit_instance_change(app: &AppHandle, r#type: &str) {
    let _ = app.emit(
        listens::INSTANCE_CHANGE,
        InstanceChangeEvent { r#type: r#type.into() },
    );
}

/// 启动状态事件
#[gui_macros::emit]
fn emit_launch_state(app: &AppHandle, event: StateEvent) {
    let _ = app.emit(listens::LAUNCH_STATE, event);
}

/// 游戏日志事件
#[gui_macros::emit]
fn emit_game_log(app: &AppHandle, event: LogEvent) {
    let _ = app.emit(listens::GAME_LOG, event);
}

/// 游戏退出事件
#[gui_macros::emit]
fn emit_game_exit(app: &AppHandle, event: ExitEvent) {
    let _ = app.emit(listens::GAME_EXIT, event);
}

/// 启动失败事件（预留：接入 mcml-core 启动链后使用）
#[allow(dead_code)]
#[gui_macros::emit]
fn emit_launch_error(app: &AppHandle, event: ErrorEvent) {
    let _ = app.emit(listens::LAUNCH_ERROR, event);
}

/// 添加空分组
#[tauri::command]
pub fn main_add_group(app: AppHandle, window: WebviewWindow, name: String) -> Result<bool, String> {
    let n = name.trim().to_string();
    if n.is_empty() {
        return Ok(false);
    }
    // 核心分组表：同名已存在则拒绝
    if !mcml_game::add_group(&n) {
        return Ok(false);
    }
    let store = model(&window)?;
    let mut store = store.lock().unwrap();
    if !store.all_groups().contains(&n) {
        store.extra_groups.push(n.clone());
    }
    emit_instance_change(&app, "group");
    Ok(true)
}

/// 删除空分组
#[tauri::command]
pub fn main_remove_group(app: AppHandle, window: WebviewWindow, name: String) -> Result<bool, String> {
    if name.trim().is_empty() {
        // 空白分组即默认分组，不允许删除
        return Ok(false);
    }
    // 核心分组表：组内实例移入默认分组
    mcml_game::remove_group(&name);
    let store = model(&window)?;
    let mut store = store.lock().unwrap();
    let before = store.extra_groups.len();
    store.extra_groups.retain(|g| g != &name);
    store.group_order.retain(|g| g != &name);
    let ok = store.extra_groups.len() < before;
    if ok {
        emit_instance_change(&app, "group");
    }
    Ok(ok)
}

/// 调整分组显示顺序
#[tauri::command]
pub fn main_move_group(app: AppHandle, window: WebviewWindow, name: String, index: i64) -> Result<bool, String> {
    let store = model(&window)?;
    let mut store = store.lock().unwrap();
    let mut list = store.all_groups();
    let from = list.iter().position(|g| g == &name).ok_or_else(|| "分组不存在".to_string())?;
    list.remove(from);
    let at = (index as usize).min(list.len());
    list.insert(at, name.clone());
    store.group_order = list;
    emit_instance_change(&app, "group");
    Ok(true)
}

/// 创建实例
#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub fn main_create_instance(
    app: AppHandle,
    window: WebviewWindow,
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
    let store = model(&window)?;
    let mut store = store.lock().unwrap();
    store.instances.insert(0, inst.clone());
    store.args.insert(uuid, InstanceArgs::default());
    drop(store);
    emit_instance_change(&app, "add");
    Ok(inst)
}

/// 重命名实例（核心实例走 mcml-game：重名报错、实例目录跟随改名）
#[tauri::command]
pub fn main_rename_instance(app: AppHandle, window: WebviewWindow, uuid: String, name: String) -> Result<bool, String> {
    let n = name.trim().to_string();
    if n.is_empty() {
        return Ok(false);
    }
    // 核心实例：mcml-game 重命名（重名返回 Err 由前端提示）
    if let Ok(id) = Uuid::parse_str(&uuid) {
        if mcml_game::get_instance(&id).is_some() {
            mcml_game::rename_instance(&id, &n).map_err(|e| e.to_string())?;
            emit_instance_change(&app, "edit");
            return Ok(true);
        }
    }
    // 遗留数据（假 uuid）：只改本地存储
    let store = model(&window)?;
    let mut store = store.lock().unwrap();
    let Some(inst) = store.instances.iter_mut().find(|i| i.uuid == uuid) else {
        return Ok(false);
    };
    inst.name = n.clone();
    inst.dir = n;
    emit_instance_change(&app, "edit");
    Ok(true)
}

/// 前端加载器显示名 -> LoaderType（[`loader_name`] 的逆映射）
fn loader_from_name(name: &str) -> Option<LoaderType> {
    match name {
        "原版" => Some(LoaderType::Normal),
        "Forge" => Some(LoaderType::Forge),
        "Fabric" => Some(LoaderType::Fabric),
        "Quilt" => Some(LoaderType::Quilt),
        "NeoForge" => Some(LoaderType::NeoForge),
        "OptiFine" => Some(LoaderType::OptiFine),
        "LiteLoader" => Some(LoaderType::LiteLoader),
        "自定义" => Some(LoaderType::Custom),
        _ => None,
    }
}

/// 前端整合包平台名 -> ModPackType（None / 空 = 非整合包）
fn modpack_type_from_name(name: Option<&str>) -> ModPackType {
    match name {
        Some("CurseForge") => ModPackType::CurseForge,
        Some("Modrinth") => ModPackType::Modrinth,
        Some("McMod") => ModPackType::McMod,
        Some("ServerPack") => ModPackType::ServerPack,
        _ => ModPackType::None,
    }
}

/// 更新实例元信息（补丁式；核心实例直接写配置并保存，遗留数据只改本地存储）
#[tauri::command]
pub fn main_update_instance(app: AppHandle, window: WebviewWindow, uuid: String, patch: InstancePatch) -> Result<bool, String> {
    // 核心实例：写真实配置
    let core = Uuid::parse_str(&uuid)
        .ok()
        .and_then(|id| mcml_game::get_instance(&id).map(|inst| (id, inst)));
    if let Some((id, instance)) = core {
        // 名字改动走 rename（实例目录跟随改名，重名报错）
        if let Some(v) = &patch.name {
            let n = v.trim();
            if !n.is_empty() && instance.read().unwrap().name != *n {
                mcml_game::rename_instance(&id, n).map_err(|e| e.to_string())?;
            }
        }
        // 分组切换走 mcml-game（自动建组 + 保存实例 + 发事件）
        if let Some(v) = patch.group.clone() {
            // 空白分组即默认分组
            let group = match v {
                Some(g) if !g.trim().is_empty() => Some(g),
                _ => None,
            };
            mcml_game::move_group(vec![id], group);
        }
        {
            let mut obj = instance.write().unwrap();
            if let Some(v) = &patch.version {
                obj.version = v.clone();
            }
            if let Some(v) = &patch.version_type {
                obj.game_type = match v.as_str() {
                    "release" => VersionType::Release,
                    "snapshot" => VersionType::Snapshot,
                    _ => obj.game_type,
                };
            }
            if let Some(v) = &patch.loader {
                if let Some(l) = loader_from_name(v) {
                    obj.loader = l;
                }
            }
            if let Some(v) = &patch.loader_version {
                obj.loader_version = Some(v.clone());
            }
            if let Some(v) = patch.modpack_type.clone() {
                obj.modpack_type = modpack_type_from_name(v.as_deref());
                obj.is_modpack = obj.modpack_type != ModPackType::None;
            }
            if let Some(v) = patch.pid.clone() {
                obj.pid = v;
            }
            if let Some(v) = patch.fid.clone() {
                obj.fid = v;
            }
            if let Some(v) = &patch.log_encoding {
                obj.encoding = match v.as_str() {
                    "gbk" => LogEncoding::GBK,
                    _ => LogEncoding::UTF8,
                };
            }
            // lang 无核心字段，仅前端显示
            obj.save();
        }
    }
    let store = model(&window)?;
    let mut store = store.lock().unwrap();
    if let Some(inst) = store.instances.iter_mut().find(|i| i.uuid == uuid) {
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
    }
    drop(store);
    emit_instance_change(&app, "edit");
    Ok(true)
}

/// 删除实例（核心实例走 mcml-game：删除实例与文件；同时清理本地运行态缓存）
#[tauri::command]
pub fn main_delete_instance(app: AppHandle, window: WebviewWindow, uuid: String) -> Result<bool, String> {
    // 核心实例：删除实例数据与文件
    let mut ok = false;
    if let Ok(id) = Uuid::parse_str(&uuid) {
        if mcml_game::get_instance(&id).is_some() {
            mcml_game::delete_instance(&id).map_err(|e| e.to_string())?;
            ok = true;
        }
    }
    // 本地存储：清理遗留实例项与运行态缓存（运行状态 / 日志 / 启动参数）
    let store = model(&window)?;
    let mut store = store.lock().unwrap();
    let before = store.instances.len();
    store.instances.retain(|i| i.uuid != uuid);
    store.args.remove(&uuid);
    store.running.remove(&uuid);
    store.logs.remove(&uuid);
    if store.instances.len() < before {
        ok = true;
    }
    drop(store);
    if ok {
        emit_instance_change(&app, "remove");
    }
    Ok(ok)
}

/// 移动实例到 (分组, 组内位置)：支持同组排序与跨组移动
#[tauri::command]
pub fn main_move_instance(app: AppHandle, window: WebviewWindow, uuid: String, group: Option<String>, index: i64) -> Result<bool, String> {
    // 核心实例：分组切换走 mcml-game（自动建组 + 保存实例 + 发事件）
    if let Ok(id) = Uuid::parse_str(&uuid) {
        if mcml_game::get_instance(&id).is_some() {
            // 空白分组即默认分组
            let group = if group.as_deref().map_or(true, |g| g.trim().is_empty()) {
                None
            } else {
                group.clone()
            };
            mcml_game::move_group(vec![id], group);
        }
    }
    let store = model(&window)?;
    let mut store = store.lock().unwrap();
    let Some(pos) = store.instances.iter().position(|i| i.uuid == uuid) else {
        return Ok(true);
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
    emit_instance_change(&app, "edit");
    Ok(true)
}

/// 启动游戏（占位：标记运行 + 发事件；接入核心后替换为真实启动）
#[tauri::command]
pub fn main_launch_game(app: AppHandle, window: WebviewWindow, uuid: String, user_name: String) -> Result<(), String> {
    println!("[launch_game] uuid={uuid} user={user_name}");
    let store = model(&window)?;
    {
        let mut store = store.lock().unwrap();
        if store.running.contains(&uuid) {
            return Err("实例已在运行中".to_string());
        }
        store.running.insert(uuid.clone());
    }
    emit_launch_state(&app, StateEvent { uuid: uuid.clone(), state: "launching".into() });
    emit_game_log(&app, LogEvent { uuid: uuid.clone(), time: now_time(), text: "游戏启动中…".into(), clear: true });

    // 占位：3 秒后发出退出事件（真实启动需接入 mcml-core）
    // 直接持有模型的 Arc：模型跟随主窗口，销毁后后台线程仍能安全收尾
    let store2 = store.clone();
    let uuid2 = uuid.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_secs(3));
        emit_game_log(&app, LogEvent { uuid: uuid2.clone(), time: now_time(), text: "游戏进程已退出".into(), clear: false });
        emit_game_exit(&app, ExitEvent { uuid: uuid2.clone(), code: 0 });
        let mut s = store2.lock().unwrap();
        s.running.remove(&uuid2);
    });
    Ok(())
}

/// 停止游戏
#[tauri::command]
pub fn main_stop_game(app: AppHandle, window: WebviewWindow, uuid: String) -> Result<(), String> {
    let store = model(&window)?;
    let mut store = store.lock().unwrap();
    store.running.remove(&uuid);
    emit_game_exit(&app, ExitEvent { uuid, code: 0 });
    Ok(())
}

/// 获取实例日志
#[tauri::command]
pub fn main_get_game_log(window: WebviewWindow, uuid: String) -> Vec<String> {
    let Ok(store) = model(&window) else {
        eprintln!("[main_get_game_log] 主窗口模型未初始化");
        return Vec::new();
    };
    store.lock().unwrap().logs.get(&uuid).cloned().unwrap_or_default()
}

/// 获取运行中实例
#[tauri::command]
pub fn main_get_running(window: WebviewWindow) -> Vec<String> {
    let Ok(store) = model(&window) else {
        eprintln!("[main_get_running] 主窗口模型未初始化");
        return Vec::new();
    };
    store.lock().unwrap().running.iter().cloned().collect()
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

