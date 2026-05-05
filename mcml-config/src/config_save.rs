use std::{
    fs::File,
    io::Write,
    path::PathBuf,
    sync::{
        Arc, Mutex, OnceLock, atomic::{AtomicBool, Ordering}
    },
    thread::Builder,
};

use mcml_log;
use mcml_names::{error_type::ErrorType, i18, thread_type::ThreadType};
use semrs::Semaphore;
use serde::Serialize;
use uuid::Uuid;

pub struct ConfigSaveObj {
    /// 保存的内容
    json: String,
    /// 保存的文件
    file: PathBuf,
    /// 任务标识
    uuid: Uuid,
}

impl ConfigSaveObj {
    pub fn new<T: Serialize>(
        obj: &T,
        file: PathBuf,
        uuid: Uuid,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(ConfigSaveObj {
            json: serde_json::to_string_pretty(obj)?,
            file,
            uuid,
        })
    }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut file = File::create(&self.file)?;
        file.write_all(self.json.as_bytes())?;
        Ok(())
    }

    pub fn json_str(&self) -> &str {
        &self.json
    }
}

/// 全局队列
static QUEUE: Mutex<Vec<ConfigSaveObj>> = Mutex::new(Vec::new());
/// 是否在运行
static IS_RUN: AtomicBool = AtomicBool::new(true);
// 锁定信号量
static SEM: OnceLock<Arc<Semaphore>> = OnceLock::new();

/// 保存一个内容
pub fn save<T>(uuid: Uuid, value: &T, file: &PathBuf)
where
    T: ?Sized + Serialize,
{
    let mut queue = QUEUE.lock().unwrap();
    // 移除所有同名的旧任务
    queue.retain(|obj| obj.uuid != uuid);
    queue.push(ConfigSaveObj {
        uuid,
        file: file.to_path_buf(),
        json: serde_json::to_string(value).unwrap(),
    });

    SEM.get().unwrap().up();
}

fn save_now() {
    let items = {
        let mut queue = QUEUE.lock().unwrap();
        std::mem::take(&mut *queue)
    };
    for save_obj in items {
        if let Err(e) = save_obj.save() {
            mcml_log::error_type(ErrorType::ConfigSaveError(
                e.to_string(),
                save_obj.file.display().to_string(),
            ));
        }
    }
}

fn run() {
    while IS_RUN.load(Ordering::Acquire) {
        SEM.get().unwrap().down();

        save_now();
    }

    save_now();
}

// 后台保存线程
pub fn start() {
    SEM.get_or_init(|| Arc::new(Semaphore::new(0)));

    Builder::new()
        .name(i18::get_thread(ThreadType::ConfigSaveThread))
        .spawn(|| run())
        .unwrap();
}

pub fn stop() {
    IS_RUN.store(false, Ordering::Release);
}
