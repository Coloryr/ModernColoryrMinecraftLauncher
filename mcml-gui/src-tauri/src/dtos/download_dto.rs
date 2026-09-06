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

/// 下载任务快照（查询进行中任务列表）
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
}
