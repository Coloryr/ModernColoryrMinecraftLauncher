//! 主窗口：新闻 / 游戏事件模型 + 实例数据存储 + IPC 命令（窗口按钮调用的方法）
//!
//! 数据从 Rust 侧获取：实例 / 分组 / Java / 版本存储在 `MainWindowModel`，
//! 持久化到应用数据目录的 `main_data.json`；前端通过 IPC 调用本模块命令。

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, LazyLock, Mutex, RwLock};
use std::time::Duration;

use tauri::{AppHandle, Emitter, WebviewWindow};

use crate::dtos::main_dto::{LoadState, NewsItem};
use crate::dtos::{ErrorEvent, ExitEvent, InstanceChangeEvent, InstancePatch, LogEvent, StateEvent};
use crate::listens;
use crate::models::{EnvVarLine, InstanceArgs, InstanceInfo, JavaInfo, VersionInfo};
use mcml_config::config_obj::{GCType, RunArgObj, WindowSettingObj};
use mcml_game::GameInstance;
use mcml_game::loader::LoaderType;
use mcml_game::mojang::VersionType;
use mcml_game::launcher::{LogEncoding, ModPackType};
use mcml_game::launcher::instance_setting_obj::{
    AdvanceJvmObj, InstanceSettingObj, ProxyHostObj, ServerObj,
};
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
}

impl MainWindowModel {
    pub fn new() -> Self {
        Self {
            instances: Vec::new(),
            args: HashMap::new(),
            extra_groups: Vec::new(),
            group_order: Vec::new(),
            running: HashSet::new(),
            logs: HashMap::new(),
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

/// 从 mcml_jvms 读取 Java 列表（配置加载 / 扫描异步进行，未完成时为空）
fn java_list() -> Vec<JavaInfo> {
    mcml_jvms::get_all_java()
        .iter()
        .map(|j| JavaInfo {
            name: j.name.clone(),
            path: j.path.to_string_lossy().to_string(),
            version: j.version.clone(),
            // 遗留占位条目（Java 失效）主版本号为 -1，前端用 0 表示未知
            major: j.major_version.max(0) as u32,
            java_type: j.java_type.clone(),
            arch: j.arch.to_string(),
        })
        .collect()
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
                version_type: Some(inst.game_type.id().to_string()),
                loader: inst.loader.id().to_string(),
                loader_version: inst.loader_version.clone(),
                dir: inst.dir.clone(),
                running: mcml_game::is_running(&inst.uuid),
                modpack_type: inst
                    .is_modpack
                    .then(|| inst.modpack_type.id().to_string()),
                pid: inst.pid.clone(),
                fid: inst.fid.clone(),
                server_url: inst.server_url.clone(),
                lang: None,
                log_encoding: Some(
                    if matches!(inst.encoding, LogEncoding::GBK) { "gbk" } else { "utf8" }.to_string(),
                ),
                source: None,
            }
        })
        .collect()
}

/// 获取分组列表（mcml-game::get_group_keys 对接）
#[tauri::command]
pub fn main_get_groups() -> Vec<String> {
    mcml_game::get_group_keys()
}

/// 获取实例的游戏内语言列表（从资源索引查 minecraft/lang/*.json，资源未下载时为空）
#[tauri::command]
pub fn main_get_instance_langs(uuid: String) -> Vec<String> {
    let Ok(id) = Uuid::parse_str(&uuid) else {
        return Vec::new();
    };
    mcml_game::get_instance_langs(&id)
}

/// 解析核心实例配置 -> 前端启动参数 DTO
fn args_from_core(inst: &InstanceSettingObj) -> InstanceArgs {
    let mut a = InstanceArgs::default();
    if let Some(jvm) = &inst.jvm_arg {
        if let Some(v) = jvm.max_memory {
            a.memory = v as i64;
        }
        if let Some(v) = jvm.min_memory {
            a.min_memory = v as i64;
        }
        a.gc = match jvm.gc_mode {
            Some(GCType::G1GC) => "g1gc",
            Some(GCType::ZGC) => "zgc",
            Some(GCType::None) => "none",
            _ => "auto",
        }
        .into();
        if let Some(s) = &jvm.jvm_args {
            a.jvm_args = s.lines().map(String::from).collect();
        }
        if let Some(s) = &jvm.game_args {
            a.game_args = s.lines().map(String::from).collect();
        }
        if let Some(s) = &jvm.jvm_env {
            a.env_vars = s.lines().filter_map(|l| l.split_once('=').map(|(k, v)| EnvVarLine {
                key: k.to_string(),
                value: v.to_string(),
            })).collect();
        }
        if let Some(v) = jvm.launch_pre_run {
            a.pre_enabled = v;
        }
        if let Some(s) = &jvm.pre_run_arg {
            a.pre_cmd = s.clone();
        }
        if let Some(v) = jvm.launch_post_run {
            a.post_enabled = v;
        }
        if let Some(s) = &jvm.post_run_arg {
            a.post_cmd = s.clone();
        }
    }
    if let Some(w) = &inst.window {
        if let Some(v) = w.full_screen {
            a.fullscreen = v;
        }
        if let Some(v) = w.width {
            a.width = v as i64;
        }
        if let Some(v) = w.height {
            a.height = v as i64;
        }
    }
    a.java_name = inst.jvm_name.clone().unwrap_or_default();
    a.java_path = inst.jvm_local.clone().unwrap_or_default();
    if let Some(adv) = &inst.advance_jvm {
        a.main_class = adv.main_class.clone().unwrap_or_default();
        if let Some(s) = &adv.class_path {
            a.class_path = s.split(';').map(str::trim).filter(|s| !s.is_empty()).map(String::from).collect();
        }
    }
    if let Some(p) = &inst.proxy_host {
        a.proxy_ip = p.ip.clone().unwrap_or_default();
        a.proxy_port = p.port.unwrap_or(0) as i64;
        a.proxy_user = p.user.clone().unwrap_or_default();
        a.proxy_pass = p.password.clone().unwrap_or_default();
    }
    if let Some(s) = &inst.start_server {
        a.join_server = s.enable;
        a.server_ip = s.ip.clone().unwrap_or_default();
        a.server_port = s.port.unwrap_or(0) as i64;
    }
    a
}

/// 前端启动参数 DTO -> 写入核心实例配置（不保存，由调用方 save）
fn apply_args_to_core(inst: &mut InstanceSettingObj, a: &InstanceArgs) {
    let jvm = inst.jvm_arg.get_or_insert_with(RunArgObj::default);
    jvm.max_memory = Some(a.memory.max(0) as u32);
    jvm.min_memory = Some(a.min_memory.max(0) as u32);
    jvm.gc_mode = Some(match a.gc.as_str() {
        "g1gc" => GCType::G1GC,
        "zgc" => GCType::ZGC,
        "none" => GCType::None,
        _ => GCType::Auto,
    });
    // 自定义 GC 参数没有独立字段：并入附加 JVM 参数一起下发
    let mut jvm_lines: Vec<String> = a.jvm_args.clone();
    if a.gc == "custom" && !a.gc_custom.trim().is_empty() {
        jvm_lines.extend(a.gc_custom.lines().map(String::from));
    }
    jvm.jvm_args = Some(jvm_lines.join("\n"));
    jvm.game_args = Some(a.game_args.join("\n"));
    jvm.jvm_env = Some(
        a.env_vars
            .iter()
            .map(|v| format!("{}={}", v.key, v.value))
            .collect::<Vec<_>>()
            .join("\n"),
    );
    jvm.launch_pre_run = Some(a.pre_enabled);
    jvm.pre_run_arg = Some(a.pre_cmd.clone());
    jvm.launch_post_run = Some(a.post_enabled);
    jvm.post_run_arg = Some(a.post_cmd.clone());

    let win = inst.window.get_or_insert_with(WindowSettingObj::default);
    win.full_screen = Some(a.fullscreen);
    win.width = Some(a.width.clamp(0, u16::MAX as i64) as u16);
    win.height = Some(a.height.clamp(0, u16::MAX as i64) as u16);

    inst.jvm_name = (!a.java_name.is_empty()).then(|| a.java_name.clone());
    inst.jvm_local = (!a.java_path.is_empty()).then(|| a.java_path.clone());

    let adv = inst.advance_jvm.get_or_insert_with(AdvanceJvmObj::default);
    adv.main_class = (!a.main_class.is_empty()).then(|| a.main_class.clone());
    adv.class_path = (!a.class_path.is_empty()).then(|| a.class_path.join(";"));

    let proxy = inst.proxy_host.get_or_insert_with(ProxyHostObj::default);
    proxy.ip = (!a.proxy_ip.is_empty()).then(|| a.proxy_ip.clone());
    proxy.port = (a.proxy_port > 0).then(|| a.proxy_port as u16);
    proxy.user = (!a.proxy_user.is_empty()).then(|| a.proxy_user.clone());
    proxy.password = (!a.proxy_pass.is_empty()).then(|| a.proxy_pass.clone());

    let server = inst.start_server.get_or_insert_with(ServerObj::default);
    server.enable = a.join_server && !a.server_ip.trim().is_empty();
    server.ip = (!a.server_ip.is_empty()).then(|| a.server_ip.clone());
    server.port = (a.server_port > 0).then(|| a.server_port as u16);
}

/// 取核心实例（uuid 解析 + 存在性检查）
fn core_instance(uuid: &str) -> Option<(Uuid, GameInstance)> {
    Uuid::parse_str(uuid)
        .ok()
        .and_then(|id| mcml_game::get_instance(&id).map(|inst| (id, inst)))
}

/// 获取实例启动参数（核心实例读配置；遗留数据读内存缓存）
#[tauri::command]
pub fn main_get_instance_args(window: WebviewWindow, uuid: String) -> InstanceArgs {
    if let Some((_, instance)) = core_instance(&uuid) {
        let obj = instance.read().unwrap();
        return args_from_core(&obj);
    }
    if let Ok(store) = model(&window) {
        if let Some(a) = store.lock().unwrap().args.get(&uuid) {
            return a.clone();
        }
    }
    InstanceArgs::default()
}

/// 更新实例启动参数（核心实例写配置并保存；遗留数据只更新内存缓存）
#[tauri::command]
pub fn main_update_instance_args(app: AppHandle, window: WebviewWindow, uuid: String, args: InstanceArgs) -> Result<bool, String> {
    if let Some((_, instance)) = core_instance(&uuid) {
        {
            let mut obj = instance.write().unwrap();
            apply_args_to_core(&mut obj, &args);
            obj.save();
        }
        emit_instance_change(&app, "edit");
        return Ok(true);
    }
    let store = model(&window)?;
    store.lock().unwrap().args.insert(uuid, args);
    Ok(true)
}

/// 获取 Java 列表（来自 mcml_jvms，配置加载 / 扫描异步进行）
#[tauri::command]
pub fn main_get_java_list() -> Vec<JavaInfo> {
    java_list()
}

/// 添加 Java（mcml_jvms 校验有效后加入列表并保存配置）
#[tauri::command]
pub fn main_add_java(name: String, path: String) -> bool {
    mcml_jvms::add_item(name, path).is_some()
}

/// 删除指定名称的 Java
#[tauri::command]
pub fn main_remove_java(name: String) {
    mcml_jvms::remove(&name);
}

/// 扫描系统已安装的 Java（注册表 / 常见路径，耗时查询放线程池）并返回最新列表
#[tauri::command]
pub async fn main_scan_java() -> Vec<JavaInfo> {
    let _ = tauri::async_runtime::spawn_blocking(mcml_jvms::scan_java).await;
    java_list()
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
/// 按类型分组排序（正式版 > 快照 > 旧版 Beta > 旧版 Alpha），
/// 组内保持清单顺序（清单本身按新旧排列）；失败返回空
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
    // 只按类型分组排序，组内不动：清单本身即最新在前，
    // 按版本号数值重排会把快照（25w14a）排到 1.21.x 之上、打乱 rc / 旧版顺序
    list.sort_by_key(|v| match v.version_type.as_str() {
        "release" => 0,
        "snapshot" => 1,
        "old_beta" => 2,
        "old_alpha" => 3,
        _ => 4,
    });
    // 不截断：快照 / 旧版类型也要有数据，否则切换版本类型后过滤结果为空
    list
}

/// 实例数据变更事件（type：add / edit / remove / group）
#[gui_macros::emit]
pub fn emit_instance_change(app: &AppHandle, r#type: &str) {
    let _ = app.emit(
        listens::INSTANCE_CHANGE,
        InstanceChangeEvent { r#type: r#type.into() },
    );
}

/// Java 列表变更事件（mcml_jvms 回调触发：添加 / 删除 / 配置加载完成）
#[gui_macros::emit]
pub fn emit_java_change(app: &AppHandle) {
    let _ = app.emit(listens::JAVA_CHANGE, ());
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
        loader: loader.unwrap_or_else(|| "normal".into()),
        loader_version,
        dir,
        running: false,
        modpack_type,
        pid: None,
        fid: None,
        server_url: None,
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
                obj.game_type = VersionType::from_id(v);
            }
            if let Some(v) = &patch.loader {
                if let Some(l) = LoaderType::from_id(v) {
                    obj.loader = l;
                }
            }
            if let Some(v) = patch.loader_version.clone() {
                obj.loader_version = v;
            }
            if let Some(v) = patch.modpack_type.clone() {
                obj.modpack_type = ModPackType::from_id(v.as_deref().unwrap_or("none"));
                obj.is_modpack = obj.modpack_type != ModPackType::None;
            }
            if let Some(v) = patch.pid.clone() {
                obj.pid = v;
            }
            if let Some(v) = patch.fid.clone() {
                obj.fid = v;
            }
            if let Some(v) = patch.server_url.clone() {
                obj.server_url = v;
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
            inst.loader_version = v;
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
        if let Some(v) = patch.server_url {
            inst.server_url = v;
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

