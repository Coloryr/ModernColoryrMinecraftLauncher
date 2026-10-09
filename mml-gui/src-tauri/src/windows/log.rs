//! 游戏日志窗口：实例运行日志与历史日志文件的查看
//!
//! 实时日志流走全局 `game-log` 事件（壳层在 lib.rs setup 里订阅 mml-core
//! `add_run_log` 转发，前端按事件负载里的实例 uuid 过滤），历史运行日志复用
//! `main_get_game_log` 拉取；本模块补日志文件浏览（logs / crash-reports 目录）。
//! 窗口目标实例由 `window_open_window` 的 instance 参数带入（URL query），
//! 窗口已存在时壳层聚焦并发 `log-focus` 事件通知已开窗口切换。
//!
//! **日志行转换也归本模块**：核心日志条目 → 前端行的 `log_line_from_item`、实时日志
//! 事件转发（`forward_run_log` / `emit_game_log` / `emit_log_line`）都放在这里。
//! 主窗口只负责"入口"（按钮开窗）与启动 / 退出时推一行状态，转换逻辑不跟着它走。

use std::path::PathBuf;

use mml_game::GameInstance;
use mml_game::game_log::{GameLog, GameLogItemObj, InstanceRuntimeLog};
use tauri::{AppHandle, Emitter};
use uuid::Uuid;

use crate::dtos::LogEvent;
use crate::dtos::{LogFocusDto, main_dto::LogLine};
use crate::listens;
use mml_game::{InstanceLog, InstanceLogType};

/// 切换目标实例事件（窗口已存在时再次打开会推送，前端据此切换实例）
#[gui_macros::emit]
pub fn emit_log_focus(app: &AppHandle, event: LogFocusDto) {
    let _ = app.emit(listens::LOG_FOCUS, event);
}

/// 解析实例 uuid
fn parse_instance(uuid: &str) -> Result<GameInstance, String> {
    let uuid = Uuid::parse_str(uuid).map_err(|_| "err.uuid".to_string())?;
    mml_game::get_instance(&uuid).ok_or_else(|| "err.gameNotFound".to_string())
}

/// 列出实例的日志文件（logs 与 crash-reports 目录，绝对路径）
#[tauri::command]
pub fn log_get_files(uuid: String) -> Result<Vec<String>, String> {
    let instance = parse_instance(&uuid)?;
    let game = instance.read().unwrap();
    Ok(game
        .get_log_files()
        .iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect())
}

/// 读取单个日志文件并解析为日志行
///
/// - `path`: `log_get_files` 返回的路径（仅接受列表内的路径，避免任意文件读取）
#[tauri::command]
pub fn log_read_file(uuid: String, path: String) -> Result<Vec<LogLine>, String> {
    let instance = parse_instance(&uuid)?;
    let game = instance.read().unwrap();
    let path = PathBuf::from(&path);
    if !game.get_log_files().contains(&path) {
        return Err("err.fileNotFound".to_string());
    }
    let runtime = InstanceRuntimeLog::from_file(&path, game.encoding);
    let logs = runtime.logs.read().unwrap();
    Ok(logs.iter().map(log_line_from_item).collect())
}

// ==================== 日志行转换与事件转发 ====================

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

/// 游戏日志事件
#[gui_macros::emit]
fn emit_game_log(app: &AppHandle, event: LogEvent) {
    let _ = app.emit(listens::GAME_LOG, event);
}

/// 发出一行游戏日志（自动解析 thread / level / category 筛选字段）
///
/// 解析用 mml-core `InstanceRuntimeLog::add_game_log` 同一套正则，
/// 保证事件里的字段与核心侧处理结果一致
pub(crate) fn emit_log_line(app: &AppHandle, uuid: &str, text: &str, clear: bool) {
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
        GameLog::Text(s) => (
            item_time(item),
            s.clone(),
            String::new(),
            String::new(),
            String::new(),
        ),
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

/// 获取实例的实时运行日志快照（实例不存在 / 没在跑返回空列表）
#[tauri::command]
pub fn log_get_runtime(uuid: String) -> Vec<LogLine> {
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
