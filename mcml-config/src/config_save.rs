use std::{
    fs::File,
    io::Write,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    },
    thread::Builder,
};

use mcml_log;
use mcml_names::{i18, thread_type::ThreadType};
use serde::Serialize;

pub struct ConfigSaveObj {
    /// 保存的内容
    json: String,
    /// 保存的文件
    file: PathBuf,
    /// 任务名字
    name: String,
}

impl ConfigSaveObj {
    pub fn new<T: Serialize>(
        obj: &T,
        file: PathBuf,
        name: String,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(ConfigSaveObj {
            json: serde_json::to_string_pretty(obj)?,
            file,
            name,
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

/// 保存一个内容
pub fn save<T>(name: String, value: &T, file: &PathBuf)
where
    T: ?Sized + Serialize,
{
    let mut queue = QUEUE.lock().unwrap();
    // 移除所有同名的旧任务
    queue.retain(|obj| obj.name != name);
    queue.push(ConfigSaveObj {
        name,
        file: file.to_path_buf(),
        json: serde_json::to_string(value).unwrap(),
    });
}

fn run() {
    mcml_log::info(String::from("Config save thread start"));

    while IS_RUN.load(Ordering::Acquire) {
        let items = {
            let mut queue = QUEUE.lock().unwrap();
            std::mem::take(&mut *queue)
        };
        for save_obj in items {
            if let Err(e) = save_obj.save() {
                mcml_log::error(format!("Failed to save {}: {}", save_obj.name, e));
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    let items = {
        let mut queue = QUEUE.lock().unwrap();
        std::mem::take(&mut *queue)
    };
    for save_obj in items {
        if let Err(e) = save_obj.save() {
            mcml_log::error(format!("Failed to save {}: {}", save_obj.name, e));
        }
    }

    mcml_log::info(String::from("Config save thread stop"));
}

// 后台保存线程
pub fn start() {
    Builder::new()
        .name(i18::get_thread(ThreadType::ConfigSaveThread))
        .spawn(|| run()).unwrap();
}

pub fn stop() {
    IS_RUN.store(false, Ordering::Release);
}
