use std::{
    mem,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex, OnceLock,
        atomic::{AtomicBool, Ordering},
    },
    thread::Builder,
};

use mcml_base::{path_helper, serialize_tools};
use mcml_log;
use mcml_names::{
    i18,
    i18_items::{
        error_type::{CoreResult, ErrorType, FileSystemErrorData},
        thread_type::ThreadType,
    },
};
use semrs::Semaphore;
use serde::Serialize;
use uuid::Uuid;

/// 保存任务
pub struct ConfigSaveObj {
    /// 保存的内容
    json: String,
    /// 保存的文件
    file: PathBuf,
    /// 任务标识
    uuid: Uuid,
}

impl ConfigSaveObj {
    pub fn new<T: Serialize>(obj: &T, file: PathBuf, uuid: Uuid) -> CoreResult<Self> {
        Ok(ConfigSaveObj {
            json: serialize_tools::json_to_string(obj)?,
            file,
            uuid,
        })
    }

    pub fn save(&self) -> CoreResult<()> {
        path_helper::write_text(&self.file, &self.json)
    }
}

/// 全局队列
static QUEUE: Mutex<Vec<ConfigSaveObj>> = Mutex::new(Vec::new());
/// 是否在运行
static IS_RUN: AtomicBool = AtomicBool::new(true);
// 锁定信号量
static SEM: OnceLock<Arc<Semaphore>> = OnceLock::new();

/// 保存一个内容
pub fn save<T, P: AsRef<Path>>(uuid: Uuid, obj: &T, file: P)
where
    T: Serialize,
{
    let mut queue = QUEUE.lock().unwrap();
    // 移除所有同名的旧任务
    queue.retain(|obj| obj.uuid != uuid);
    queue.push(ConfigSaveObj::new(obj, file.as_ref().to_path_buf(), uuid).unwrap());

    SEM.get().unwrap().up();
}

/// 执行一次保存
fn save_now() {
    let items = {
        let mut queue = QUEUE.lock().unwrap();
        mem::take(&mut *queue)
    };
    for save_obj in items {
        if let Err(err) = save_obj.save() {
            mcml_log::error_type(err);
        }
    }
}

// 后台保存线程
pub fn start() {
    SEM.get_or_init(|| Arc::new(Semaphore::new(0)));

    Builder::new()
        .name(i18::get_thread(ThreadType::ConfigSaveThread))
        .spawn(|| {
            while IS_RUN.load(Ordering::Acquire) {
                SEM.get().unwrap().down();

                save_now();
            }

            save_now();
        })
        .unwrap();
}

pub fn stop() {
    IS_RUN.store(false, Ordering::Release);
}
