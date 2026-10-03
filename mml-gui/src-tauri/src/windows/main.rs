//! 主窗口：新闻 / 游戏事件模型 + 实例数据存储 + IPC 命令（窗口按钮调用的方法）
//!
//! 数据从 Rust 侧获取：实例 / Java / 版本存储在 `MainWindowModel`，
//! 持久化到应用数据目录的 `main_data.json`；前端通过 IPC 调用本模块命令。
//!
//! **分组不在这里**：组名归属与两种顺序都由内核分组表持有
//! （`mml_game::game_group` 的 `group_save.json`），本模块只转发。

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, LazyLock, Mutex, RwLock};
use std::time::Duration;

use tauri::{AppHandle, Emitter, WebviewWindow};

use crate::dtos::main_dto::{LoadState, LogLine, NewsItem};
use crate::dtos::{
    EnvVarLineDto, ErrorEvent, ExitEvent, GroupDto, InstanceArgsDto, InstanceChangeEvent,
    InstanceInfoDto, InstanceLangDto, InstancePatch, JavaInfoDto, LogEvent, MotdDto, StateEvent,
    SystemMemoryDto, VersionInfoDto,
};
use crate::{image_manager, listens, windows};
use mml_config::config_obj::{GCType, RunArgObj, WindowSettingObj};
use mml_game::game_log::{GameLog, GameLogItemObj};
use mml_game::GameInstance;
use mml_game::launcher::instance_setting_obj::{
    AdvanceJvmObj, InstanceSettingObj, ProxyHostObj, ServerObj,
};
use mml_game::launcher::{LogEncoding, ModPackType};
use mml_game::{InstanceLog, InstanceLogType};
use mml_game::loader::LoaderType;
use mml_game::mojang::VersionType;
use uuid::Uuid;

/// 核心加载完成事件（ok：加载成功；error：失败信息，前端据此显示错误页）
#[gui_macros::emit]
pub fn emit_load_done(app: &AppHandle, data: Option<String>) {
    let _ = app.emit(
        listens::LOAD_DONE,
        LoadState {
            ok: data.is_none(),
            error: data,
        },
    );
}

/// 主窗口数据存储：实例 / 启动参数 / 运行状态 / 日志
pub struct MainWindowModel {
    /// 实例列表（遗留数据的兜底存储；核心实例以 mml-game 为准）
    pub instances: Vec<InstanceInfoDto>,
    /// 遗留实例的启动参数（uuid → 参数）
    pub args: HashMap<String, InstanceArgsDto>,
    /// 运行中的实例 uuid
    pub running: HashSet<String>,
}

impl MainWindowModel {
    pub fn new() -> Self {
        Self {
            instances: Vec::new(),
            args: HashMap::new(),
            running: HashSet::new(),
        }
    }
}

/// 从 mml_jvms 读取 Java 列表（配置加载 / 扫描异步进行，未完成时为空）
fn java_list() -> Vec<JavaInfoDto> {
    mml_jvms::get_all_java()
        .iter()
        .map(|j| JavaInfoDto {
            name: j.name.clone(),
            path: j.path.to_string_lossy().to_string(),
            version: j.version.clone(),
            // 遗留占位条目（Java 失效）主版本号为 -1，前端用 0 表示未知
            major: j.major_version.max(0),
            java_type: j.java_type.clone(),
            arch: j.arch.to_string(),
        })
        .collect()
}

/// 当前时间（`时:分:秒.毫秒`，日志行时间戳）
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

/// 取主窗口模型（模型跟随主窗口生命周期，开窗创建、关窗销毁，见 `windows`）
///
/// 这些命令只会由主窗口 webview 调用；模型缺失属异常情形（主窗口未创建），
/// 返回 Err / 空列表由调用方降级，不 panic。
fn model(window: &WebviewWindow) -> Result<Arc<Mutex<MainWindowModel>>, String> {
    windows::window_model(window).ok_or_else(|| "err.modelMissing".to_string())
}

/// 获取实例列表（mml-game::get_instances 对接，合并运行状态）
#[tauri::command]
pub fn main_get_instances() -> Vec<InstanceInfoDto> {
    mml_game::get_instances()
        .into_iter()
        .map(|inst| {
            let inst = inst.read().unwrap();
            InstanceInfoDto {
                uuid: inst.uuid.to_string(),
                name: inst.name.clone(),
                // 分组归属与组内次序都在 mml-game 的分组表里（group_save.json）：
                // 实例配置（game.json）与 guisetting.json 都没有这两个字段了
                group: mml_game::get_instance_group(&inst.uuid).map(|g| g.to_string()),
                version: inst.version.clone(),
                version_type: Some(inst.game_type.id().to_string()),
                loader: String::from(inst.loader.to_string()),
                loader_version: inst.loader_version.clone(),
                dir: inst.dir.clone(),
                running: mml_game::is_running(&inst.uuid),
                modpack_type: inst.is_modpack.then(|| inst.modpack_type.to_string()),
                pid: inst.pid.clone(),
                fid: inst.fid.clone(),
                server_url: inst.server_url.clone(),
                lang: None,
                log_encoding: Some(
                    if matches!(inst.encoding, LogEncoding::GBK) {
                        "gbk"
                    } else {
                        "utf8"
                    }
                    .to_string(),
                ),
                source: None,
                // 组内次序 = 分组表里的 `order`（实例 uuid → 次序）；不在任何分组里时排到最后
                order: mml_game::get_instance_order(&inst.uuid).unwrap_or(i32::MAX),
            }
        })
        .collect()
}

/// 获取分组列表（含空分组）
///
/// 返回 uuid + 组名：分组以 uuid 为身份、组名只是显示数据，
/// 前端要用 uuid 回传（移动实例 / 删组 / 调序）。
#[tauri::command]
pub fn main_get_groups() -> Vec<GroupDto> {
    mml_game::get_group_list()
        .into_iter()
        .map(|g| GroupDto {
            uuid: g.uuid.to_string(),
            name: g.name,
        })
        .collect()
}

/// 获取实例的游戏内语言列表（从资源索引查 minecraft/lang/*.json，资源未下载时为空）
///
/// 每项带显示名：名字取自对应语言文件里的 `language.name`（如 zh_cn → 简体中文），
/// 读不到时回落为语言代码（见 `mml_game::get_instance_langs`）。
///
/// 查询要读整份资源索引 JSON、再逐个读语言文件，是同步重活：丢到阻塞线程执行，
/// 不要占住 async runtime —— 否则同期的其它 IPC 与图片协议请求会被一起拖住。
#[tauri::command]
pub async fn main_get_instance_langs(uuid: String) -> Vec<InstanceLangDto> {
    let res = tauri::async_runtime::spawn_blocking(move || {
        let Ok(id) = Uuid::parse_str(&uuid) else {
            return Vec::new();
        };
        mml_game::get_instance_langs(&id)
            .into_iter()
            .map(|item| InstanceLangDto {
                code: item.code,
                name: item.name,
            })
            .collect()
    })
    .await;

    res.unwrap_or_default()
}

/// 解析核心实例配置 -> 前端启动参数 DTO
fn args_from_core(inst: &InstanceSettingObj) -> InstanceArgsDto {
    let mut a = InstanceArgsDto::default();
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
            a.env_vars = s
                .lines()
                .filter_map(|l| {
                    l.split_once('=').map(|(k, v)| EnvVarLineDto {
                        key: k.to_string(),
                        value: v.to_string(),
                    })
                })
                .collect();
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
            a.class_path = s
                .split(';')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(String::from)
                .collect();
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
fn apply_args_to_core(inst: &mut InstanceSettingObj, a: &InstanceArgsDto) {
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
pub(crate) fn core_instance(uuid: &str) -> Option<(Uuid, GameInstance)> {
    Uuid::parse_str(uuid)
        .ok()
        .and_then(|id| mml_game::get_instance(&id).map(|inst| (id, inst)))
}

/// 获取实例启动参数（核心实例读配置；遗留数据读内存缓存）
#[tauri::command]
pub fn main_get_instance_args(window: WebviewWindow, uuid: String) -> InstanceArgsDto {
    if let Some((_, instance)) = core_instance(&uuid) {
        let obj = instance.read().unwrap();
        return args_from_core(&obj);
    }
    if let Ok(store) = model(&window) {
        if let Some(a) = store.lock().unwrap().args.get(&uuid) {
            return a.clone();
        }
    }
    InstanceArgsDto::default()
}

/// 更新实例启动参数（核心实例写配置并保存；遗留数据只更新内存缓存）
#[tauri::command]
pub fn main_update_instance_args(
    app: AppHandle,
    window: WebviewWindow,
    uuid: String,
    args: InstanceArgsDto,
) -> Result<bool, String> {
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

/// 获取 Java 列表（来自 mml_jvms，配置加载 / 扫描异步进行）
#[tauri::command]
pub fn main_get_java_list() -> Vec<JavaInfoDto> {
    java_list()
}

/// 获取本机内存（MiB），供启动参数里的内存设置显示参考值
///
/// 前端拿不到物理内存，只能问后端；查询失败时核心返回 `u64::MAX` 哨兵值，
/// 这里统一折算成 0，由前端决定不显示。
#[tauri::command]
pub fn main_get_system_memory() -> SystemMemoryDto {
    let total = mml_sys::memory_helper::get_memory_size();
    let free = mml_sys::memory_helper::get_memory_free();

    SystemMemoryDto {
        total: if total == u64::MAX { 0 } else { total },
        free: if free == u64::MAX { 0 } else { free },
    }
}

/// 版本列表缓存（进程级：主窗口 / 添加实例窗口共用，不挂在窗口模型上）
static VERSIONS_CACHE: LazyLock<RwLock<Vec<VersionInfoDto>>> =
    LazyLock::new(|| RwLock::new(Vec::new()));

/// 获取游戏版本列表（从 mml-core 拉取版本清单，缓存于进程）
#[tauri::command]
pub async fn main_get_versions() -> Result<Vec<VersionInfoDto>, String> {
    {
        let cache = VERSIONS_CACHE.read().unwrap();
        if !cache.is_empty() {
            return Ok(cache.clone());
        }
    }
    let fetched = fetch_versions().await;
    *VERSIONS_CACHE.write().unwrap() = fetched.clone();
    Ok(fetched)
}

/// 强制刷新版本列表（清空缓存重新从版本清单拉取）
#[tauri::command]
pub async fn main_refresh_versions() -> Result<Vec<VersionInfoDto>, String> {
    VERSIONS_CACHE.write().unwrap().clear();
    main_get_versions().await
}

/// 获取 Minecraft 官方新闻（Mojang 新闻接口，按页拉取；核心加载完成后可调用）
#[tauri::command]
pub async fn main_get_news(page: Option<u32>) -> Result<Vec<NewsItem>, String> {
    let page = page.unwrap_or(1);
    let news = mml_net::mojang_api::get_minecraft_news(page)
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
    mml_sys::open_helper::open_url(&url);
}

/// 从 mml-core 拉取版本清单（按配置源：官方 / BMCLAPI），
/// 按类型分组排序（正式版 > 快照 > 旧版 Beta > 旧版 Alpha），
/// 组内保持清单顺序（清单本身按新旧排列）；失败返回空
async fn fetch_versions() -> Vec<VersionInfoDto> {
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
    let Ok(bytes) = mml_net::mojang_api::get_versions(None).await else {
        return Vec::new();
    };
    let Ok(manifest) = serde_json::from_slice::<Manifest>(&bytes) else {
        return Vec::new();
    };
    let mut list: Vec<VersionInfoDto> = manifest
        .versions
        .into_iter()
        .map(|v| VersionInfoDto {
            id: v.id,
            version_type: v.version_type,
        })
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
        InstanceChangeEvent {
            r#type: r#type.into(),
        },
    );
}

/// Java 列表变更事件（mml_jvms 回调触发：添加 / 删除 / 配置加载完成）
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

/// 发出一行游戏日志（自动解析 thread / level / category 筛选字段）
///
/// 解析用 mml-core `InstanceRuntimeLog::add_game_log` 同一套正则，
/// 保证事件里的字段与核心侧处理结果一致
fn emit_log_line(app: &AppHandle, uuid: &str, text: &str, clear: bool) {
    let obj = mml_game::game_log::InstanceRuntimeLog::parse_game_log_line(text);
    emit_game_log(
        app,
        LogEvent {
            uuid: uuid.to_string(),
            time: now_time(),
            text: text.to_string(),
            thread: obj.thread,
            level: obj.level.as_str().to_string(),
            category: obj.category,
            clear,
        },
    );
}

/// 内核日志条目的捕获时间（`时:分:秒.毫秒`）
fn item_time(item: &GameLogItemObj) -> String {
    item.time.format("%H:%M:%S%.3f").to_string()
}

/// 内核日志条目 → 前端日志行
///
/// 启动器消息变体（耗时 / 路径 / 参数等）转为可读文本；标准游戏日志直接取
/// 解析四字段，行内无时间戳时回退条目捕获时间
pub(crate) fn log_line_from_item(item: &GameLogItemObj) -> LogLine {
    let (mut time, text, thread, level, category) = match &item.log {
        GameLog::GameLog(obj) => (
            obj.time.clone(),
            obj.log.clone(),
            obj.thread.clone(),
            obj.level.as_str().to_string(),
            obj.category.clone(),
        ),
        GameLog::Text(s) => (item_time(item), s.clone(), String::new(), String::new(), String::new()),
        GameLog::RuntimeLib(p) => (
            item_time(item),
            format!("运行库：{}", p.display()),
            String::new(),
            String::new(),
            String::new(),
        ),
        GameLog::JavaRedirect => (
            item_time(item),
            String::from("Java 输出已重定向"),
            String::new(),
            String::new(),
            String::new(),
        ),
        GameLog::JavaLocalRedirect => (
            item_time(item),
            String::from("Java 切换回本地查找"),
            String::new(),
            String::new(),
            String::new(),
        ),
        GameLog::LoginTime(d) => (
            item_time(item),
            format!("登录用时：{:.2}s", d.as_secs_f64()),
            String::new(),
            String::new(),
            String::new(),
        ),
        GameLog::ServerPackCheckTime(d) => (
            item_time(item),
            format!("服务器包检查用时：{:.2}s", d.as_secs_f64()),
            String::new(),
            String::new(),
            String::new(),
        ),
        GameLog::CheckGameFileTime(d) => (
            item_time(item),
            format!("检查游戏文件用时：{:.2}s", d.as_secs_f64()),
            String::new(),
            String::new(),
            String::new(),
        ),
        GameLog::DownloadFileTime(d) => (
            item_time(item),
            format!("文件下载用时：{:.2}s", d.as_secs_f64()),
            String::new(),
            String::new(),
            String::new(),
        ),
        GameLog::LaunchTime(d) => (
            item_time(item),
            format!("启动用时：{:.2}s", d.as_secs_f64()),
            String::new(),
            String::new(),
            String::new(),
        ),
        GameLog::CmdPreTime(d) => (
            item_time(item),
            format!("启动前执行用时：{:.2}s", d.as_secs_f64()),
            String::new(),
            String::new(),
            String::new(),
        ),
        GameLog::CmdPostTime(d) => (
            item_time(item),
            format!("启动后执行用时：{:.2}s", d.as_secs_f64()),
            String::new(),
            String::new(),
            String::new(),
        ),
        GameLog::LaunchArgs(s) => (
            item_time(item),
            s.clone(),
            String::new(),
            String::new(),
            String::new(),
        ),
        GameLog::JavaPath(p) => (
            item_time(item),
            format!("Java 路径：{}", p.display()),
            String::new(),
            String::new(),
            String::new(),
        ),
    };
    if time.is_empty() {
        time = item_time(item);
    }
    LogLine {
        time,
        text,
        thread,
        level,
        category,
    }
}

/// 转发内核实例运行日志到前端（lib.rs setup 里订阅 `mml_game::add_run_log`）
pub(crate) fn forward_run_log(app: &AppHandle, log: &InstanceLog) {
    let uuid = log.uuid.to_string();
    match &log.log {
        InstanceLogType::AddLog(item) => {
            let line = log_line_from_item(item);
            emit_game_log(
                app,
                LogEvent {
                    uuid,
                    time: line.time,
                    text: line.text,
                    thread: line.thread,
                    level: line.level,
                    category: line.category,
                    clear: false,
                },
            );
        }
        InstanceLogType::ClearLog => {
            emit_game_log(
                app,
                LogEvent {
                    uuid,
                    time: String::new(),
                    text: String::new(),
                    thread: String::new(),
                    level: String::new(),
                    category: String::new(),
                    clear: true,
                },
            );
        }
    }
}

/// 游戏退出事件
#[gui_macros::emit]
fn emit_game_exit(app: &AppHandle, event: ExitEvent) {
    let _ = app.emit(listens::GAME_EXIT, event);
}

/// 启动失败事件（预留：接入 mml-core 启动链后使用）
#[allow(dead_code)]
#[gui_macros::emit]
fn emit_launch_error(app: &AppHandle, event: ErrorEvent) {
    let _ = app.emit(listens::LAUNCH_ERROR, event);
}

/// 新建分组
///
/// 分组表在 mml-game（`group_save.json`），GUI 不再自己另存一份 ——
/// 之前模型里那份 `extra_groups` 既不落盘、又与内核表各说各话，
/// 新建的空分组下一次 `getGroups` 就被内核列表覆盖掉了。
///
/// # 返回值
///
/// 返回新分组的 uuid；名字为空或重名返回 `None`（前端据此提示"已存在"）
#[tauri::command]
pub fn main_add_group(app: AppHandle, name: String) -> Result<Option<String>, String> {
    let Some(uuid) = mml_game::add_group(&name) else {
        return Ok(None);
    };
    emit_instance_change(&app, "group");
    Ok(Some(uuid.to_string()))
}

/// 删除分组（组内实例移入默认分组）
#[tauri::command]
pub fn main_remove_group(app: AppHandle, uuid: String) -> Result<bool, String> {
    // 空白 uuid 就是默认分组，不允许删除
    let Some(uuid) = windows::parse_group_id(Some(uuid)) else {
        return Ok(false);
    };
    // 核心分组表：组内实例移入默认分组，并逐个发变更通知
    if !mml_game::remove_group(&uuid) {
        return Ok(false);
    }
    emit_instance_change(&app, "group");
    Ok(true)
}

/// 调整分组显示顺序（默认分组恒在首位，不参与排序）
///
/// 顺序由内核分组表保存：拿当前顺序、把该组挪到 `index` 位，再整表提交。
/// 只改内存里的数组是不行的 —— 文件里存的还是旧顺序，重启就回来了。
#[tauri::command]
pub fn main_move_group(app: AppHandle, uuid: String, index: i64) -> Result<bool, String> {
    let Some(uuid) = windows::parse_group_id(Some(uuid)) else {
        return Err("err.uuid".to_string());
    };

    let mut list: Vec<Uuid> = mml_game::get_group_list().into_iter().map(|g| g.uuid).collect();
    let from = list
        .iter()
        .position(|g| *g == uuid)
        .ok_or_else(|| "err.groupNotFound".to_string())?;
    list.remove(from);
    let at = (index.max(0) as usize).min(list.len());
    list.insert(at, uuid);

    mml_game::reorder_groups(&list);
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
) -> Result<InstanceInfoDto, String> {
    let uuid = format!("mml-{}", uuid_short());
    let dir = name.clone();
    let inst = InstanceInfoDto {
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
        // 新建的实例排到组末：给一个足够大的值，下次整组重编号时会被压回正常区间
        order: i32::MAX,
    };
    let store = model(&window)?;
    let mut store = store.lock().unwrap();
    store.instances.insert(0, inst.clone());
    store.args.insert(uuid, InstanceArgsDto::default());
    drop(store);
    emit_instance_change(&app, "add");
    Ok(inst)
}

/// 重命名实例（核心实例走 mml-game：重名报错、实例目录跟随改名）
#[tauri::command]
pub fn main_rename_instance(
    app: AppHandle,
    window: WebviewWindow,
    uuid: String,
    name: String,
) -> Result<bool, String> {
    let n = name.trim().to_string();
    if n.is_empty() {
        return Ok(false);
    }
    // 核心实例：mml-game 重命名（重名返回 Err 由前端提示）
    if let Ok(id) = Uuid::parse_str(&uuid) {
        if mml_game::get_instance(&id).is_some() {
            mml_game::rename_instance(&id, &n).map_err(|e| e.to_string())?;
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
pub fn main_update_instance(
    app: AppHandle,
    window: WebviewWindow,
    uuid: String,
    patch: InstancePatch,
) -> Result<bool, String> {
    // 核心实例：写真实配置
    let core = Uuid::parse_str(&uuid)
        .ok()
        .and_then(|id| mml_game::get_instance(&id).map(|inst| (id, inst)));
    if let Some((id, instance)) = core {
        // 名字改动走 rename（实例目录跟随改名，重名报错）
        if let Some(v) = &patch.name {
            let n = v.trim();
            if !n.is_empty() && instance.read().unwrap().name != *n {
                mml_game::rename_instance(&id, n).map_err(|e| e.to_string())?;
            }
        }
        // 分组切换走 mml-game 的分组表（追加到目标组末尾、发事件）
        if let Some(v) = patch.group.clone() {
            // 前端传的是分组 uuid；空 / 非法 = 默认分组
            mml_game::move_group(vec![id], windows::parse_group_id(v));
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
                if let Some(l) = LoaderType::from_string(v) {
                    obj.loader = l;
                }
            }
            if let Some(v) = patch.loader_version.clone() {
                obj.loader_version = v;
            }
            if let Some(v) = patch.modpack_type.clone() {
                obj.modpack_type = ModPackType::from_string(v.as_deref().unwrap_or("none"));
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

/// 删除实例：挪回收站较慢，放后台线程执行避免卡住 UI（前端显示滚动进度条）
/// （核心实例走 mml-game：删除实例与文件；同时清理本地运行态缓存）
#[tauri::command]
pub async fn main_delete_instance(
    app: AppHandle,
    window: WebviewWindow,
    uuid: String,
) -> Result<bool, String> {
    let ok = tauri::async_runtime::spawn_blocking(move || -> Result<bool, String> {
        // 核心实例：删除实例数据与文件
        let mut ok = false;
        if let Ok(id) = Uuid::parse_str(&uuid) {
            if mml_game::get_instance(&id).is_some() {
                mml_game::delete_instance(&id).map_err(|e| e.to_string())?;
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
        if store.instances.len() < before {
            ok = true;
        }
        drop(store);
        Ok(ok)
    })
    .await
    .map_err(|e| e.to_string())??;

    if ok {
        emit_instance_change(&app, "remove");
    }
    Ok(ok)
}

/// 移动实例到 (分组, 组内位置)：支持同组排序与跨组移动
///
/// 归属与组内次序都在内核分组表（`group_save.json`）：内核一次完成"落组 + 插到 index 位"，
/// 前端下次拉列表就按表的数组顺序拿到新的 `order`。
///
/// 这里不再改各实例的 `guisetting.json` —— 那个 `Order` 已经是旧机制，
/// 两份顺序各写各的正是"拖完看着对了、重启就乱"的来源。
#[tauri::command]
pub fn main_move_instance(
    app: AppHandle,
    uuid: String,
    group: Option<String>,
    index: i64,
) -> Result<bool, String> {
    let Ok(id) = Uuid::parse_str(&uuid) else {
        return Err("err.uuid".to_string());
    };
    if mml_game::get_instance(&id).is_none() {
        return Err("err.gameNotFound".to_string());
    }

    // 前端传的是分组 uuid；空 / 非法 = 默认分组
    let group = windows::parse_group_id(group);

    mml_game::move_instance(&id, group, index.max(0) as usize);

    emit_instance_change(&app, "edit");
    Ok(true)
}

/// 启动游戏（占位：标记运行 + 发事件；接入核心后替换为真实启动）
///
/// 启动用户名由后端自己从当前账户解析（`auths::get_current()`），前端不传
#[tauri::command]
pub fn main_launch_game(
    app: AppHandle,
    window: WebviewWindow,
    uuid: String,
) -> Result<(), String> {
    println!("[launch_game] uuid={uuid}");
    let store = model(&window)?;
    {
        let mut store = store.lock().unwrap();
        if store.running.contains(&uuid) {
            return Err("err.instanceRunning".to_string());
        }
        store.running.insert(uuid.clone());
    }
    emit_launch_state(
        &app,
        StateEvent {
            uuid: uuid.clone(),
            state: "launching".into(),
            progress: None,
        },
    );
    emit_log_line(&app, &uuid, "游戏启动中…", true);

    // 占位：3 秒后发出退出事件（真实启动需接入 mml-core）
    // 直接持有模型的 Arc：模型跟随主窗口，销毁后后台线程仍能安全收尾
    let store2 = store.clone();
    let uuid2 = uuid.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_secs(3));
        emit_log_line(&app, &uuid2, "游戏进程已退出", false);
        emit_game_exit(
            &app,
            ExitEvent {
                uuid: uuid2.clone(),
                code: 0,
            },
        );
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

/// 获取实例日志（内核运行日志快照；实例不存在返回空列表）
#[tauri::command]
pub fn main_get_game_log(uuid: String) -> Vec<LogLine> {
    let Ok(id) = Uuid::parse_str(&uuid) else {
        return Vec::new();
    };
    let Some(instance) = mml_game::get_instance(&id) else {
        return Vec::new();
    };
    let game = instance.read().unwrap();
    let Some(runtime) = game.get_runtime_log() else {
        return Vec::new();
    };
    runtime.iter().map(log_line_from_item).collect()
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

/// `mml-image` 协议访问前缀（前端拼实例图标等地址用）
#[tauri::command]
pub fn main_image_base_url() -> String {
    image_manager::image_base_url().to_string()
}

/// 核心加载状态（前端兜底：启动太快时 load-done 事件会先于页面监听发出而被错过）
#[tauri::command]
pub fn main_load_state() -> LoadState {
    LoadState {
        ok: mml_core::get_state(),
        error: None,
    }
}

/// 查询服务器 MOTD（地址 host 或 host:port，端口缺省 25565）
#[tauri::command]
pub async fn main_get_motd(address: String) -> MotdDto {
    let (ip, port) = parse_motd_addr(&address);
    mml_game::game_motd::get_server_info(&ip, port).await.into()
}

/// MOTD 展示的默认端口
const MOTD_DEFAULT_PORT: u16 = 25565;

/// 解析服务器地址：host / host:port / [IPv6]:port / 裸 IPv6
fn parse_motd_addr(address: &str) -> (String, u16) {
    let addr = address.trim();
    if addr.is_empty() {
        return (String::new(), MOTD_DEFAULT_PORT);
    }
    // [IPv6]:port
    if let Some(rest) = addr.strip_prefix('[') {
        if let Some((host, port)) = rest.split_once(']') {
            let port = port
                .strip_prefix(':')
                .and_then(|p| p.parse().ok())
                .unwrap_or(MOTD_DEFAULT_PORT);
            return (host.to_string(), port);
        }
    }
    // host:port（只有一个冒号才算 host:port，多个冒号视为裸 IPv6）
    if let Some((host, port)) = addr.rsplit_once(':') {
        if !host.contains(':')
            && !port.is_empty()
            && let Ok(p) = port.parse::<u16>()
        {
            return (host.to_string(), p);
        }
    }
    (addr.to_string(), MOTD_DEFAULT_PORT)
}

#[cfg(test)]
mod tests {
    use super::parse_motd_addr;

    #[test]
    fn test_parse_motd_addr() {
        assert_eq!(parse_motd_addr("mc.example.com"), ("mc.example.com".into(), 25565));
        assert_eq!(parse_motd_addr("mc.example.com:25566"), ("mc.example.com".into(), 25566));
        assert_eq!(parse_motd_addr("  mc.example.com:25566 "), ("mc.example.com".into(), 25566));
        assert_eq!(parse_motd_addr("[::1]:25565"), ("::1".into(), 25565));
        assert_eq!(parse_motd_addr("::1"), ("::1".into(), 25565));
        assert_eq!(parse_motd_addr(""), (String::new(), 25565));
    }
}
