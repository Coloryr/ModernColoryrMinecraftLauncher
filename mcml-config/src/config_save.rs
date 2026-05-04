use std::{
    fs::File,
    io::Write,
    path::PathBuf,
    sync::atomic::{AtomicBool, Ordering},
    thread::Builder,
};

use crossbeam_queue::SegQueue;
use mcml_log;
use serde::Serialize;

pub struct ConfigSaveObj {
    json_string: String,
    file: PathBuf,
    name: String,
}

impl ConfigSaveObj {
    pub fn new<T: Serialize>(
        obj: &T,
        file: PathBuf,
        name: String,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(ConfigSaveObj {
            json_string: serde_json::to_string_pretty(obj)?,
            file,
            name,
        })
    }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut file = File::create(&self.file)?;
        file.write_all(self.json_string.as_bytes())?;
        Ok(())
    }

    pub fn json_str(&self) -> &str {
        &self.json_string
    }
}

// 全局队列
static SAVE_QUEUE: SegQueue<ConfigSaveObj> = SegQueue::new();

static RUN: AtomicBool = AtomicBool::new(true);

/// 保存一个内容
pub fn save<T>(name: String, value: &T, file: &PathBuf)
where
    T: ?Sized + Serialize,
{
    SAVE_QUEUE.push(ConfigSaveObj {
        name,
        file: file.to_path_buf(),
        json_string: serde_json::to_string(value).unwrap(),
    });
}

fn run() {
    mcml_log::info(String::from("Config save thread start"));

    while RUN.load(Ordering::Acquire) {
        while let Some(save_obj) = SAVE_QUEUE.pop() {
            if let Err(e) = save_obj.save() {
                mcml_log::error(format!("Failed to save {}: {}", save_obj.name, e));
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    while let Some(save_obj) = SAVE_QUEUE.pop() {
        if let Err(e) = save_obj.save() {
            mcml_log::error(format!("Failed to save {}: {}", save_obj.name, e));
        }
    }

    mcml_log::info(String::from("Config save thread stop"));
}

// 后台保存线程
pub fn start() {
    let thread = Builder::new()
        .name("Config Save Thread".into())
        .spawn(|| run());

    if thread.is_err() {
        panic!("Config Thread start fail")
    }
}

pub fn stop() {
    RUN.store(false, Ordering::Release);
}
