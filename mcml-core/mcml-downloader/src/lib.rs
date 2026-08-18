//! 下载器模块
//!
//! 本模块实现了启动器的文件下载引擎，支持多线程并发下载、
//! 断点续传、哈希校验、下载进度回调等功能。
//!
//! # 架构概述
//!
//! ```text
//! start_download_task()          —— 创建下载任务并入队
//!     │
//!     ▼
//! DownloadTask::new()            —— 将文件列表包装为任务
//!     │
//!     ▼
//! DownloadThread (× N)           —— N 个工作线程从队列取文件下载
//!     │
//!     ▼
//! download()                     —— 单文件下载流程（断点续传 + 哈希校验）
//!     │
//!     ▼
//! later_tasks::unpack_native()   —— 下载后处理（解压 native 库等）
//! ```
//!
//! # 核心类型
//!
//! | 类型 | 用途 |
//! |------|------|
//! | [`DownloadItem`] | 单个下载文件的状态跟踪 |
//! | [`DownloadTask`] | 一组下载文件的批量任务管理 |
//! | [`DownloadThread`] | 下载工作线程封装 |
//! | [`IDownloadGui`] | UI 更新回调接口 |
//!
//! # 下载流程
//!
//! 1. 每个文件首先检查是否已存在且哈希匹配 → 跳过下载
//! 2. 支持 `Range` 断点续传（需服务器支持）
//! 3. 下载完成后校验文件大小和哈希值
//! 4. 超过 5 次错误自动放弃当前文件
//! 5. 下载后支持解压 native 库和存档文件

pub mod download_item;
pub mod download_task;

mod download_thread;
pub mod later_tasks;

use std::{
    path::{Path, PathBuf},
    sync::{
        Arc, OnceLock, RwLock,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
};

use mcml_base::{file_item::FileItemObj};
use mcml_names::{i18_items::error_type::CoreResult, names};
use mcml_sys::path_helper;
use uuid::Uuid;

use crate::{
    download_item::DownloadItem, download_task::DownloadTask, download_thread::DownloadThread,
};

/// 下载任务进度快照
pub struct TaskStateObj {
    /// 任务编号
    pub id: u64,
    /// 下载进度（0.0–100.0）
    pub progress: f64,
}

/// 下载任务状态变更事件
pub enum DownloadTaskState {
    /// 新任务已添加
    AddTask(u64),
    /// 任务已移除
    RemoveTask(u64),
    /// 任务进度更新
    UpdateTask(TaskStateObj),
}

/// 下载器 UI 回调接口
///
/// 实现此 trait 以接收下载引擎的状态更新通知。
pub trait IDownloadGui {
    /// 单个文件下载状态更新
    ///
    /// # 参数
    ///
    /// - `thread`: 下载线程序号
    /// - `file`: 当前正在下载的文件信息
    fn update(&self, thread: u32, file: &Arc<DownloadItem>);

    /// 下载任务进度更新
    ///
    /// # 参数
    ///
    /// - `state`: 任务状态变更类型
    fn update_task(&self, state: DownloadTaskState);
}

/// 下载项目（关联任务和具体文件）
pub(crate) struct DownloadObj {
    /// 所属下载任务
    pub task: Arc<DownloadTask>,
    /// 当前下载的文件项
    pub item: Arc<DownloadItem>,
}

/// 下载线程列表
static THREADS: RwLock<Vec<DownloadThread>> = RwLock::new(Vec::new());
/// 下载任务队列
static TASKS: RwLock<Vec<Arc<DownloadTask>>> = RwLock::new(Vec::new());

/// 下载器 UI 回调（全局单例）
static DOWNLOAD_GUI: OnceLock<Box<dyn IDownloadGui + Sync + Send>> = OnceLock::new();

/// 下载器停止标志
static STOP: AtomicBool = AtomicBool::new(false);

/// 自增任务编号
static NEXT_TASK_ID: AtomicU64 = AtomicU64::new(1);

/// 临时下载文件夹路径
static DOWNLOAD_PATH: OnceLock<PathBuf> = OnceLock::new();

/// 初始化下载文件夹
///
/// # 参数
///
/// - `dir`: 程序运行根目录，下载文件夹将创建在 `{dir}/downloads/` 下
pub fn init<P: AsRef<Path>>(dir: P) -> CoreResult<()> {
    let dir = DOWNLOAD_PATH.get_or_init(|| dir.as_ref().join(names::DOWNLOAD_DIR));
    if !dir.exists() {
        path_helper::create_dir_all(dir)?;
    }

    Ok(())
}

/// 获取下载临时文件夹路径
pub fn get_download_path() -> PathBuf {
    DOWNLOAD_PATH.get().unwrap().clone()
}

/// 生成一个随机的临时文件路径（UUID v4）
///
/// 保证不与已有文件冲突。
pub fn gen_temp_file() -> PathBuf {
    loop {
        let file = DOWNLOAD_PATH
            .get()
            .unwrap()
            .join(Uuid::new_v4().to_string());
        if file.exists() {
            continue;
        }
        return file;
    }
}

/// 设置下载器 UI 回调
///
/// 应在启动下载前调用一次。
pub fn set_gui_handel(gui: Box<dyn IDownloadGui + Sync + Send>) {
    DOWNLOAD_GUI.get_or_init(|| gui);
}

/// 通知 UI：文件下载进度更新
pub(crate) fn update(thread: u32, file: &Arc<DownloadItem>) {
    if let Some(gui) = DOWNLOAD_GUI.get() {
        gui.as_ref().update(thread, file);
    }
}

/// 通知 UI：任务进度更新
pub(crate) fn update_task(id: u64, progress: f64) {
    if let Some(gui) = DOWNLOAD_GUI.get() {
        gui.as_ref()
            .update_task(DownloadTaskState::UpdateTask(TaskStateObj { id, progress }));
    }
}

/// 通知 UI：新任务已添加
pub(crate) fn add_task(id: u64) {
    if let Some(gui) = DOWNLOAD_GUI.get() {
        gui.as_ref().update_task(DownloadTaskState::AddTask(id));
    }
}

/// 通知 UI：任务已移除
pub(crate) fn remove_task(id: u64) {
    if let Some(gui) = DOWNLOAD_GUI.get() {
        gui.as_ref().update_task(DownloadTaskState::RemoveTask(id));
    }
}

/// 生成下一个任务编号（原子自增）
pub(crate) fn gen_task_id() -> u64 {
    NEXT_TASK_ID.fetch_add(1, Ordering::SeqCst)
}

/// 从任务队列中获取一个待下载的文件项
///
/// 遍历所有未完成任务，返回第一个有可用下载项的任务。
pub(crate) fn get_item() -> Option<DownloadObj> {
    let read = TASKS.read().unwrap();
    if read.is_empty() {
        return None;
    }
    for task in read.iter() {
        let item = task.get_item();
        if item.is_none() {
            continue;
        } else {
            return Some(DownloadObj {
                task: task.clone(),
                item: Arc::new(item.unwrap()),
            });
        }
    }
    return None;
}

/// 标记任务完成并从队列中移除
pub(crate) fn task_done(task: &DownloadTask) {
    let mut tasks = TASKS.write().unwrap();
    let id = task.id;

    tasks.retain(|t| t.id != task.id);

    remove_task(id);
}

/// 启动下载器
///
/// 根据配置中的下载线程数创建工作线程池。
/// 应在下载前调用一次。
pub fn start() {
    let config = mcml_config::read_config();
    let mut thread = config.http.download_thread;
    if thread <= 0 {
        thread = 5;
    }

    let mut list = THREADS.write().unwrap();
    for index in 0..thread {
        list.push(DownloadThread::new(index));
    }
}

/// 停止下载器
///
/// 设置停止标志，取消所有正在进行的任务，等待所有线程退出。
pub fn stop() {
    if STOP.load(Ordering::SeqCst) {
        return;
    }
    STOP.store(true, Ordering::SeqCst);
    for item in TASKS.write().unwrap().iter() {
        item.cancel();
    }
    for item in THREADS.write().unwrap().iter_mut() {
        item.stop();
    }
}

/// 创建新下载任务并开始下载
///
/// # 参数
///
/// - `items`: 需要下载的文件列表
///
/// # 返回值
///
/// `true` — 全部下载成功
/// `false` — 下载被停止或有文件下载失败
pub async fn start_download_task(items: Vec<FileItemObj>) -> bool {
    if STOP.load(Ordering::SeqCst) {
        return false;
    }
    let task = DownloadTask::new(items);
    let task = Arc::new(task);
    let task_handel = task.clone();
    let id = task.id;

    TASKS.write().unwrap().push(task);

    add_task(id);

    // 唤醒所有工作线程
    for item in THREADS.read().unwrap().iter() {
        item.run();
    }

    task_handel.wait_done().await
}
