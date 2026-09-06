//! 下载窗口
//!
//! 对接 `mcml_downloader`：
//! - 实现 [`DownloadGuiHook`] 作为下载器的 UI 回调，把任务 / 文件状态
//!   变化转发为前端事件（download-task / download-item）
//! - `download_get_tasks` 查询进行中的任务快照
//! - `download_cancel_task` 取消一个下载任务
//! 窗口本身由 `../window_manager.rs` 统一创建。

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use mcml_downloader::{
    DownloadTaskState,
    download_item::{DownloadItem, DownloadItemState},
};
use tauri::{AppHandle, Emitter};

use crate::{
    dtos::{DownloadItemEvent, DownloadTaskDto, DownloadTaskEvent},
    listens,
};

/// 下载线程 → 最近一次上报的 (文件名, 状态)
///
/// 下载器按 HTTP 块粒度回调 `update`，这里去重：同一线程
/// 文件名与状态均未变化时不重复发事件。
type LastItem = HashMap<u32, (String, u32)>;

/// 下载器 UI 回调桥接（转发为 Tauri 事件）
pub struct DownloadGuiHook {
    app: AppHandle,
    last: Mutex<LastItem>,
}

impl DownloadGuiHook {
    pub fn new(app: AppHandle) -> Self {
        DownloadGuiHook {
            app,
            last: Mutex::new(HashMap::new()),
        }
    }

    /// 文件名：优先用 FileItemObj.name，为空则取路径末段
    fn item_name(file: &Arc<DownloadItem>) -> String {
        if !file.base.name.is_empty() {
            return file.base.name.clone();
        }
        file.base
            .file
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default()
    }

    fn state_id(state: DownloadItemState) -> String {
        match state {
            DownloadItemState::Wait => "wait",
            DownloadItemState::Download => "download",
            DownloadItemState::GetInfo => "getinfo",
            DownloadItemState::Pause => "pause",
            DownloadItemState::Init => "init",
            DownloadItemState::Action => "action",
            DownloadItemState::Done => "done",
            DownloadItemState::Error => "error",
        }
        .into()
    }
}

impl mcml_downloader::IDownloadGui for DownloadGuiHook {
    fn update(&self, thread: u32, file: &Arc<DownloadItem>) {
        let name = Self::item_name(file);
        let state = file.get_state();
        let state_num = state.state_to_int();

        {
            let mut last = self.last.lock().unwrap();
            match last.get(&thread) {
                Some((old_name, old_state)) if *old_name == name && *old_state == state_num => {
                    return;
                }
                _ => {
                    last.insert(thread, (name.clone(), state_num));
                }
            }
        }

        emit_download_item(
            &self.app,
            DownloadItemEvent {
                thread,
                name,
                state: Self::state_id(state),
            },
        );
    }

    fn update_task(&self, state: DownloadTaskState) {
        let event = match state {
            DownloadTaskState::AddTask(id) => DownloadTaskEvent {
                r#type: "add".into(),
                id,
                progress: 0.0,
            },
            DownloadTaskState::RemoveTask(id) => DownloadTaskEvent {
                r#type: "remove".into(),
                id,
                progress: 0.0,
            },
            DownloadTaskState::UpdateTask(obj) => DownloadTaskEvent {
                r#type: "update".into(),
                id: obj.id,
                progress: obj.progress,
            },
        };
        emit_download_task(&self.app, event);
    }
}

/// 任务列表事件
#[gui_macros::emit]
pub fn emit_download_task(app: &AppHandle, event: DownloadTaskEvent) {
    let _ = app.emit(listens::DOWNLOAD_TASK, event);
}

/// 下载线程文件状态事件
#[gui_macros::emit]
pub fn emit_download_item(app: &AppHandle, event: DownloadItemEvent) {
    let _ = app.emit(listens::DOWNLOAD_ITEM, event);
}

/// 查询进行中的下载任务快照
#[tauri::command]
pub fn download_get_tasks() -> Vec<DownloadTaskDto> {
    mcml_downloader::get_tasks()
        .into_iter()
        .map(|t| DownloadTaskDto {
            id: t.id,
            total: t.total,
            completed: t.completed,
            failed: t.failed,
        })
        .collect()
}

/// 取消一个下载任务（任务不存在返回 false）
#[tauri::command]
pub fn download_cancel_task(id: u64) -> bool {
    mcml_downloader::cancel_task(id)
}
