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

use mcml_base::{file_item::FileItemObj, path_helper};
use mcml_names::{i18_items::error_type::CoreResult, names};
use uuid::Uuid;

use crate::{
    download_item::DownloadItem, download_task::DownloadTask, download_thread::DownloadThread,
};

/// 下载任务状态
pub struct TaskStateObj {
    /// 任务编号
    pub id: u64,
    /// 下载进度
    pub progress: f64,
}

/// 当前下载状态
pub enum DownloadTaskState {
    /// 添加任务 任务编号
    AddTask(u64),
    /// 删除任务 任务编号
    RemoveTask(u64),
    /// 更新下载状态
    UpdateTask(TaskStateObj),
}

/// 下载显示回调
pub trait IDownloadGui {
    /// 下载状态更新
    /// 
    /// - `thread`: 下载线程序号
    /// - `state`: 是否还在下载
    /// - `count`: 下载任务总数
    fn update(&self, thread: u32, file: &Arc<DownloadItem>);
    /// 更新任务进度
    /// 
    /// - `id`: 任务编号
    /// - `now`: 任务进度
    fn update_task(&self, state: DownloadTaskState);
}

/// 一个下载项目
pub(crate) struct DownloadObj {
    /// 下载任务
    pub task: Arc<DownloadTask>,
    /// 下载项目
    pub item: Arc<DownloadItem>,
}

/// 下载线程列表
static THREADS: RwLock<Vec<DownloadThread>> = RwLock::new(Vec::new());
/// 下载任务列表
static TASKS: RwLock<Vec<Arc<DownloadTask>>> = RwLock::new(Vec::new());

/// 下载界面回调
static DOWNLOAD_GUI: OnceLock<Box<dyn IDownloadGui + Sync + Send>> = OnceLock::new();

/// 停止运行
static STOP: AtomicBool = AtomicBool::new(false);

/// 下一个任务编号
static NEXT_TASK_ID: AtomicU64 = AtomicU64::new(1);

/// 下载文件夹
static DOWNLOAD_PATH: OnceLock<PathBuf> = OnceLock::new();

/// 初始化下载文件夹
/// 
/// - `dir`: 基础文件夹
pub fn init<P: AsRef<Path>>(dir: P) -> CoreResult<()> {
    let dir = DOWNLOAD_PATH.get_or_init(|| dir.as_ref().join(names::DOWNLOAD_DIR));
    if !dir.exists() {
        path_helper::create_dir_all(dir)?;
    }

    Ok(())
}

/// 获取下载文件夹
pub fn get_download_path() -> PathBuf {
    DOWNLOAD_PATH.get().unwrap().clone()
}

/// 生成一个临时文件
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

/// 设置界面
pub fn set_gui_handel(gui: Box<dyn IDownloadGui + Sync + Send>) {
    DOWNLOAD_GUI.get_or_init(|| gui);
}

/// 更新文件进度
pub(crate) fn update(thread: u32, file: &Arc<DownloadItem>) {
    if let Some(gui) = DOWNLOAD_GUI.get() {
        gui.as_ref().update(thread, file);
    }
}

/// 更新任务进度
pub(crate) fn update_task(id: u64, progress: f64) {
    if let Some(gui) = DOWNLOAD_GUI.get() {
        gui.as_ref()
            .update_task(DownloadTaskState::UpdateTask(TaskStateObj { id, progress }));
    }
}

/// 添加下载任务
pub(crate) fn add_task(id: u64) {
    if let Some(gui) = DOWNLOAD_GUI.get() {
        gui.as_ref().update_task(DownloadTaskState::AddTask(id));
    }
}

/// 删除任务进度
pub(crate) fn remove_task(id: u64) {
    if let Some(gui) = DOWNLOAD_GUI.get() {
        gui.as_ref().update_task(DownloadTaskState::RemoveTask(id));
    }
}

/// 生成一个任务编号
pub(crate) fn gen_task_id() -> u64 {
    NEXT_TASK_ID.fetch_add(1, Ordering::SeqCst)
}

/// 获取一个下载项目
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

/// 下载任务结束
pub(crate) fn task_done(task: &DownloadTask) {
    let mut tasks = TASKS.write().unwrap();
    let id = task.id;

    tasks.retain(|t| t.id != task.id);

    remove_task(id);
}

/// 启动下载器
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

/// 新建一个下载任务开始下载
/// 
/// - `items`: 需要下载的文件
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

    for item in THREADS.read().unwrap().iter() {
        item.run();
    }

    task_handel.wait_done().await
}
