//! 游戏日志窗口：实例运行日志与历史日志文件的查看
//!
//! 实时日志流走全局 `game-log` 事件（壳层在 lib.rs setup 里订阅 mml-core
//! `add_run_log` 转发，前端按事件负载里的实例 uuid 过滤），历史运行日志复用
//! `main_get_game_log` 拉取；本模块补日志文件浏览（logs / crash-reports 目录）。
//! 窗口目标实例由 `window_open_window` 的 instance 参数带入（URL query），
//! 窗口已存在时壳层聚焦并发 `log-focus` 事件通知已开窗口切换。

use std::path::PathBuf;

use mml_game::GameInstance;
use mml_game::game_log::InstanceRuntimeLog;
use tauri::{AppHandle, Emitter};
use uuid::Uuid;

use crate::dtos::{LogFocusDto, main_dto::LogLine};
use crate::listens;
use crate::windows::main::log_line_from_item;

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
    if !game.get_log_files().iter().any(|p| *p == path) {
        return Err("err.fileNotFound".to_string());
    }
    let runtime = InstanceRuntimeLog::from_file(&path, game.encoding);
    let logs = runtime.logs.read().unwrap();
    Ok(logs.iter().map(log_line_from_item).collect())
}
