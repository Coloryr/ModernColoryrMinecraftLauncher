//! 日志窗口：规格 + 创建操作 + 日志读取命令
//!
//! 日志只有单一文件（`<运行目录>/logs.log`，`[时间][级别]内容` 逐行追加），
//! 三种视图都在这一个文件上切：
//! - 运行日志：最后一次启动标记（CoreStart）到文件结尾，即本次会话
//! - 历史日志：最后一次启动标记之前的全部内容，即以往会话
//! - 错误日志：全文件中的 `[Error]` / `[Fault]` 行
//!
//! 读取都从文件尾部截断到 [`LOG_MAX_LINES`] 行，避免长期运行后内容过大拖垮前端。

use std::path::PathBuf;

use mml_base::get_base_dir;
use mml_names::names;

/// 单次返回的最大行数（从尾部截断）
const LOG_MAX_LINES: usize = 10000;

/// 日志文件路径
fn log_file() -> PathBuf {
    get_base_dir().join(names::LOG_FILE)
}

/// 会话起始标记（CoreStart 文案的中英文前缀，日志级别固定为 Info）
fn is_session_start(line: &str) -> bool {
    line.contains("M²L启动，版本") || line.contains("M²L started, version")
}

/// 读取全部日志行（文件不存在视为空日志）
fn read_lines() -> Result<Vec<String>, String> {
    let text = std::fs::read_to_string(log_file()).map_err(|e| e.to_string())?;
    Ok(text.lines().map(String::from).collect())
}

/// 取尾部最多 max 行并拼回文本
fn tail(lines: &[String], max: usize) -> String {
    let start = lines.len().saturating_sub(max);
    lines[start..].join("\n")
}

/// 当前会话日志（从最后一次启动标记到文件结尾）
#[tauri::command]
pub fn log_read_runtime() -> Result<String, String> {
    let lines = read_lines()?;
    let start = lines.iter().rposition(|l| is_session_start(l)).unwrap_or(0);
    Ok(tail(&lines[start..], LOG_MAX_LINES))
}

/// 历史会话日志（最后一次启动标记之前的全部内容）
#[tauri::command]
pub fn log_read_history() -> Result<String, String> {
    let lines = read_lines()?;
    let end = lines
        .iter()
        .rposition(|l| is_session_start(l))
        .unwrap_or(lines.len());
    Ok(tail(&lines[..end], LOG_MAX_LINES))
}

/// 错误日志（全文件中的 Error / Fault 行）
#[tauri::command]
pub fn log_read_errors() -> Result<String, String> {
    let lines = read_lines()?;
    let errs: Vec<String> = lines
        .into_iter()
        .filter(|l| l.contains("[Error]") || l.contains("[Fault]"))
        .collect();
    Ok(tail(&errs, LOG_MAX_LINES))
}
