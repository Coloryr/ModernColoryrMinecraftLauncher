//! 下载窗口
//!
//! 对接 `mcml_downloader`：
//! - 实现 [`DownloadGuiHook`] 作为下载器的 UI 回调，把任务 / 文件状态变化转发为前端事件
//!   （download-task / download-item），同时维护线程状态表（当前文件、状态、速度采样）
//! - `download_get_status` 查询任务 + 线程 + 总体速度快照（前端按固定间隔轮询）
//! - `download_pause_all` / `download_resume_all` / `download_cancel_all` 全局控制下载
//! 窗口本身由 `../window_manager.rs` 统一创建。

use std::{
    collections::HashMap,
    sync::{Arc, LazyLock, Mutex, RwLock},
    time::Instant,
};

use mcml_downloader::{
    DownloadTaskState,
    download_item::{DownloadItem, DownloadItemState},
};
use tauri::{AppHandle, Emitter};

use crate::{
    dtos::{
        DownloadItemEvent, DownloadStatusDto, DownloadTaskDto, DownloadTaskEvent, DownloadThreadDto,
    },
    listens,
};

/// 速度采样最小间隔（秒）：间隔更短的回调不重算速度，避免抖动
const SPEED_WINDOW: f64 = 0.25;
/// 速度过期时间（秒）：超过此时长没有进度回调，视为线程停滞，速度按 0 计
const SPEED_EXPIRE: f64 = 1.5;

/// 下载线程运行状态（回调写入，命令读取）
struct ThreadInfo {
    /// 当前下载项（取进度与字节数）
    item: Arc<DownloadItem>,
    /// 当前文件名
    name: String,
    /// 状态 ID
    state: String,
    /// 上次速度采样的字节数
    sample_bytes: u64,
    /// 上次速度采样时间
    sample_at: Instant,
    /// 采样得到的速度（字节/秒）
    speed: f64,
}

impl ThreadInfo {
    fn new(item: Arc<DownloadItem>, name: String, state: String) -> Self {
        ThreadInfo {
            sample_bytes: item.get_now_size(),
            item,
            name,
            state,
            sample_at: Instant::now(),
            speed: 0.0,
        }
    }

    /// 用最新进度刷新速度采样：换文件（字节回退 / 文件名变化）时重置
    fn sample(&mut self, now: Instant) {
        let bytes = self.item.get_now_size();
        if bytes < self.sample_bytes {
            // 换到新文件（含断点续传重置），重新起算
            self.sample_bytes = bytes;
            self.sample_at = now;
            self.speed = 0.0;
            return;
        }

        let elapsed = now.duration_since(self.sample_at).as_secs_f64();
        if elapsed >= SPEED_WINDOW {
            self.speed = (bytes - self.sample_bytes) as f64 / elapsed;
            self.sample_bytes = bytes;
            self.sample_at = now;
        }
    }
}

/// 下载线程状态表（线程序号 → 状态）
static THREADS: LazyLock<RwLock<HashMap<u32, ThreadInfo>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

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

    /// 更新线程状态表（含速度采样）
    fn track(thread: u32, file: &Arc<DownloadItem>, name: &str, state: &str) {
        let now = Instant::now();
        let mut map = THREADS.write().unwrap();
        match map.get_mut(&thread) {
            Some(info) => {
                // 换文件时重置采样起点
                if info.name != name {
                    info.speed = 0.0;
                    info.sample_at = now;
                    info.sample_bytes = file.get_now_size();
                }
                info.item = file.clone();
                info.name = name.to_string();
                info.state = state.to_string();
                info.sample(now);
            }
            None => {
                let mut info = ThreadInfo::new(file.clone(), name.to_string(), state.to_string());
                info.sample(now);
                map.insert(thread, info);
            }
        }
    }
}

impl mcml_downloader::IDownloadGui for DownloadGuiHook {
    fn update(&self, thread: u32, file: &Arc<DownloadItem>) {
        let name = Self::item_name(file);
        let state = file.get_state();
        let state_num = state.state_to_int();
        let state_id = Self::state_id(state);

        // 线程状态表始终刷新（速度采样需要每次回调）
        Self::track(thread, file, &name, &state_id);

        // 事件按 (文件名, 状态) 去重，避免按块刷屏
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
                state: state_id,
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

/// 查询下载状态快照：任务列表 + 线程状态 + 总体速度
#[tauri::command]
pub fn download_get_status() -> DownloadStatusDto {
    let tasks: Vec<DownloadTaskDto> = mcml_downloader::get_tasks()
        .into_iter()
        .map(|t| DownloadTaskDto {
            id: t.id,
            total: t.total,
            completed: t.completed,
            failed: t.failed,
            all_bytes: t.all_bytes,
            now_bytes: t.now_bytes,
            elapsed_ms: t.elapsed_ms,
            paused: t.paused,
        })
        .collect();

    let now = Instant::now();
    let paused = mcml_downloader::is_paused_all();
    let mut speed = 0u64;
    let mut threads: Vec<DownloadThreadDto> = Vec::new();

    for (thread, info) in THREADS.read().unwrap().iter() {
        // 长时间没有进度回调（停滞 / 暂停 / 已结束）时速度归零
        let idle = now.duration_since(info.sample_at).as_secs_f64() > SPEED_EXPIRE;
        let thread_speed = if idle { 0.0 } else { info.speed };
        speed += thread_speed as u64;

        // 全局暂停期间，工作中的线程状态同步显示为暂停
        let state = if paused && (info.state == "download" || info.state == "getinfo") {
            "pause".to_string()
        } else {
            info.state.clone()
        };

        threads.push(DownloadThreadDto {
            thread: *thread,
            name: info.name.clone(),
            state,
            progress: info.item.progress(),
            now_bytes: info.item.get_now_size(),
            all_bytes: info.item.get_all_size(),
            speed: thread_speed as u64,
        });
    }

    threads.sort_by_key(|t| t.thread);

    DownloadStatusDto {
        tasks,
        threads,
        speed,
        paused,
    }
}

/// 全局暂停：暂停所有下载任务（期间新增任务同样暂停），返回被暂停的任务数
#[tauri::command]
pub fn download_pause_all() -> usize {
    mcml_downloader::pause_all()
}

/// 全局恢复：恢复所有下载任务并唤醒线程，返回被恢复的任务数
#[tauri::command]
pub fn download_resume_all() -> usize {
    mcml_downloader::resume_all()
}

/// 全局停止：取消所有下载任务，返回被停止的任务数
#[tauri::command]
pub fn download_cancel_all() -> usize {
    mcml_downloader::cancel_all()
}
