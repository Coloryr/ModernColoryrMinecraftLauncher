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

/// 下载任务快照（GUI 查询用）
#[derive(Debug, Clone)]
pub struct TaskSnapshot {
    /// 任务编号
    pub id: u64,
    /// 文件总数
    pub total: usize,
    /// 已完成数
    pub completed: usize,
    /// 失败数
    pub failed: usize,
}

/// 获取当前进行中任务的快照列表（下载窗口查询用）
pub fn get_tasks() -> Vec<TaskSnapshot> {
    let read = TASKS.read().unwrap();
    read.iter()
        .map(|t| TaskSnapshot {
            id: t.id,
            total: t.total_size,
            completed: t.completed_count.load(Ordering::SeqCst),
            failed: t.failed_count.load(Ordering::SeqCst),
        })
        .collect()
}

/// 取消指定任务：移出队列并唤醒完成等待（在途文件自然结束，剩余文件不再下载）
///
/// 取消成功返回 `true`，任务不存在（已完成 / 已取消）返回 `false`
pub fn cancel_task(id: u64) -> bool {
    let task = {
        let mut tasks = TASKS.write().unwrap();
        let pos = match tasks.iter().position(|t| t.id == id) {
            Some(pos) => pos,
            None => return false,
        };
        tasks.remove(pos)
    };
    task.cancel();
    task_done(&task);
    true
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

// ============================================================================
// 测试支持与单元测试
// ============================================================================

#[cfg(test)]
pub(crate) mod test_util {
    //! 测试公共辅助：全局状态（日志/配置/基础目录/GUI 回调）在同一个测试
    //! 二进制中只能初始化一次，这里用 `OnceLock` + `Once` 保证幂等。

    use std::fs;
    use std::path::PathBuf;
    use std::sync::{Mutex, Once, OnceLock};

    /// GUI 回调记录到的事件列表
    pub static GUI_EVENTS: OnceLock<Mutex<Vec<String>>> = OnceLock::new();

    /// 测试运行根目录（临时目录下的唯一子目录）
    static RUN_DIR: OnceLock<PathBuf> = OnceLock::new();
    /// 保证初始化流程只执行一次
    static INIT: Once = Once::new();

    /// 测试专用的 GUI 回调，把所有通知记录为字符串
    struct TestGui;

    impl super::IDownloadGui for TestGui {
        fn update(
            &self,
            thread: u32,
            file: &std::sync::Arc<super::download_item::DownloadItem>,
        ) {
            let mut events = GUI_EVENTS.get().unwrap().lock().unwrap();
            events.push(format!(
                "file:{}:{}:{:.1}",
                thread,
                file.base.name,
                file.progress()
            ));
        }

        fn update_task(&self, state: super::DownloadTaskState) {
            let mut events = GUI_EVENTS.get().unwrap().lock().unwrap();
            match state {
                super::DownloadTaskState::AddTask(id) => events.push(format!("AddTask:{id}")),
                super::DownloadTaskState::RemoveTask(id) => events.push(format!("RemoveTask:{id}")),
                super::DownloadTaskState::UpdateTask(obj) => {
                    events.push(format!("UpdateTask:{}:{:.0}", obj.id, obj.progress))
                }
            }
        }
    }

    /// 在临时目录下创建唯一的子目录（测试用）
    pub fn make_temp_dir(name: &str) -> PathBuf {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "mcml-downloader-unit-{}-{}-{}",
            name,
            std::process::id(),
            COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// 初始化测试环境（进程内只执行一次），返回测试运行根目录
    pub fn ensure_env() -> PathBuf {
        INIT.call_once(|| {
            let dir = make_temp_dir("run");

            GUI_EVENTS
                .set(Mutex::new(Vec::new()))
                .ok()
                .unwrap_or_else(|| panic!("GUI_EVENTS 初始化失败"));

            // 日志系统（下载线程内部记录错误时依赖信号量已初始化）
            mcml_log::start(&dir).unwrap();
            // 基础目录
            mcml_base::init(&dir);
            // 配置系统
            mcml_config::init(&dir).unwrap();
            // GUI 回调（全局单例）
            super::set_gui_handel(Box::new(TestGui));

            RUN_DIR.set(dir).unwrap();
        });

        RUN_DIR.get().unwrap().clone()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::Ordering;

    use mcml_base::file_item::FileItemObj;

    use crate::download_task::DownloadTask;

    use super::test_util::{GUI_EVENTS, ensure_env};

    /// 构造一个简单的下载文件项
    fn file_item(name: &str) -> FileItemObj {
        FileItemObj {
            name: name.to_string(),
            ..Default::default()
        }
    }

    /// 任务队列：FIFO 出队 + 计数
    #[test]
    fn task_queue_fifo_and_counts() {
        ensure_env();

        let task = DownloadTask::new(vec![file_item("a"), file_item("b")]);
        assert_eq!(task.total_size, 2);
        assert_eq!(task.completed_count.load(Ordering::SeqCst), 0);
        assert_eq!(task.failed_count.load(Ordering::SeqCst), 0);

        // SegQueue 是先进先出
        let first = task.get_item().unwrap();
        assert_eq!(first.base.name, "a");
        let second = task.get_item().unwrap();
        assert_eq!(second.base.name, "b");
        // 队列取空后返回 None
        assert!(task.get_item().is_none());
    }

    /// 成功完成的任务：wait_done 返回 true，并从全局任务队列移除
    #[test]
    fn task_done_waits_and_completes() {
        ensure_env();

        let task = DownloadTask::new(vec![file_item("a")]);
        let id = task.id;

        // 队列非空时任务未完成，wait_done 不会被提前唤醒
        // （这里先取走唯一文件再标记完成，保证不会阻塞）
        let _item = task.get_item().unwrap();
        task.done();
        assert_eq!(task.completed_count.load(Ordering::SeqCst), 1);

        let rt = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap();
        let ok = rt.block_on(task.wait_done());
        assert!(ok);

        // 完成后任务已从全局队列移除（任务原本也不在队列中，此处确认无副作用）
        assert!(super::get_tasks().iter().all(|t| t.id != id));
    }

    /// 存在失败文件的任务：wait_done 返回 false
    #[test]
    fn task_failed_reports_false() {
        ensure_env();

        let task = DownloadTask::new(vec![file_item("a")]);
        let _item = task.get_item().unwrap();
        task.fail();
        assert_eq!(task.failed_count.load(Ordering::SeqCst), 1);

        let rt = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap();
        let ok = rt.block_on(task.wait_done());
        assert!(!ok);
    }

    /// 任务进度通过 GUI 回调上报（1/2 完成 = 50%）
    #[test]
    fn task_progress_notifies_gui() {
        ensure_env();

        let task = DownloadTask::new(vec![file_item("a"), file_item("b")]);
        let id = task.id;

        let _item = task.get_item().unwrap();
        task.done();

        let events = GUI_EVENTS.get().unwrap().lock().unwrap();
        assert!(
            events
                .iter()
                .any(|e| e == &format!("UpdateTask:{id}:50")),
            "应收到 50% 进度事件，实际事件: {:?}",
            *events
        );
    }
}
