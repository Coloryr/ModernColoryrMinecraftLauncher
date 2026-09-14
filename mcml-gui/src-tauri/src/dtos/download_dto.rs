//! 下载窗口 DTO —— 下载任务事件 / 快照 / 文件进度

use serde::Serialize;

/// 下载任务状态事件（`r#type`：add / remove / update）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadTaskEvent {
    pub r#type: String,
    pub id: u64,
    /// 任务进度（0.0–100.0，add / remove 事件为 0）
    pub progress: f64,
}

/// 下载线程当前文件事件（仅线程的文件或状态变化时发送）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadItemEvent {
    /// 下载线程序号
    pub thread: u32,
    /// 文件名
    pub name: String,
    /// 下载状态 ID（wait / getinfo / download / action / done / error）
    pub state: String,
}

/// 下载任务快照（任务列表 / 总体进度用）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadTaskDto {
    pub id: u64,
    /// 文件总数
    pub total: usize,
    /// 已完成数
    pub completed: usize,
    /// 失败数
    pub failed: usize,
    /// 总大小（字节；元信息未知时为 0）
    pub all_bytes: u64,
    /// 已下载大小（字节）
    pub now_bytes: u64,
    /// 已进行时间（毫秒）
    pub elapsed_ms: u64,
    /// 是否暂停
    pub paused: bool,
}

/// 下载线程当前状态（含当前文件进度与速度）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadThreadDto {
    /// 下载线程序号
    pub thread: u32,
    /// 当前文件名
    pub name: String,
    /// 状态 ID（wait / getinfo / download / pause / init / action / done / error）
    pub state: String,
    /// 当前文件进度（0.0–100.0）
    pub progress: f64,
    /// 当前文件已下载字节
    pub now_bytes: u64,
    /// 当前文件总字节（未知为 0）
    pub all_bytes: u64,
    /// 下载速度（字节/秒）
    pub speed: u64,
}

/// 下载状态快照（任务 + 线程 + 总体速度）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadStatusDto {
    /// 进行中的任务
    pub tasks: Vec<DownloadTaskDto>,
    /// 各下载线程当前状态
    pub threads: Vec<DownloadThreadDto>,
    /// 总体下载速度（字节/秒）
    pub speed: u64,
    /// 是否处于全局暂停
    pub paused: bool,
}
