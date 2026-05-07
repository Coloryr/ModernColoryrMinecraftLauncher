pub mod log_item;

use crossbeam_queue::SegQueue;
use mcml_names::{i18, i18_items::{error_type::ErrorType, info_type::InfoType, panic_type::PanicType, thread_type::ThreadType}, names};
use semrs::Semaphore;

use std::{
    fs::{File, OpenOptions},
    io::{BufWriter, Write},
    path::PathBuf,
    sync::{
        Arc, Mutex, OnceLock, RwLock,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self},
};

use crate::log_item::{LogItem, LogLevel};

// 日志写入队列
static QUEUE: RwLock<SegQueue<LogItem>> = RwLock::new(SegQueue::new());
// 日志文件
static STREAM: OnceLock<Mutex<BufWriter<File>>> = OnceLock::new();
// 是否运行中
static IS_RUN: AtomicBool = AtomicBool::new(true);
// 锁定信号量
static SEM: OnceLock<Arc<Semaphore>> = OnceLock::new();

/// 开始启动日志系统
///
/// - `local`: 日志文件存储路径
pub fn start(local: &PathBuf) {
    SEM.get_or_init(|| Arc::new(Semaphore::new(0)));

    let log_path = local.join(names::NAME_LOG_FILE);

    let file = match OpenOptions::new()
        .create(true)
        .append(true)
        .write(true)
        .open(&log_path)
    {
        Ok(f) => f,
        Err(e) => {
            panic!(
                "{}",
                i18::get_panic(PanicType::LogOpenFail(
                    log_path.display().to_string(),
                    e.to_string()
                ))
            );
        }
    };

    STREAM.set(Mutex::new(BufWriter::new(file))).unwrap();

    thread::Builder::new()
        .name(i18::get_thread(ThreadType::LogThread))
        .spawn(|| run())
        .unwrap();
}

/// 停止日志系统
pub fn stop() {
    IS_RUN.store(false, Ordering::Release);
}

/// 进行一次日志保存
fn save() {
    let log = QUEUE.read().unwrap();
    let mut file = STREAM.get().unwrap().lock().unwrap();

    while !log.is_empty() {
        let item = log.pop();
        if item.is_some() {
            let item1 = item.unwrap();
            file.write_fmt(format_args!(
                "[{}][{}]{}{}",
                item1.get_time(),
                item1.get_level(),
                item1.log,
                if cfg!(windows) { "\r\n" } else { "\n" }
            ))
            .unwrap();
            file.flush().unwrap();
        }
    }
}

fn run() {
    while IS_RUN.load(Ordering::Acquire) {
        SEM.get().unwrap().down();

        save();
    }

    save();
}

pub fn info(text: String) {
    QUEUE
        .write()
        .unwrap()
        .push(LogItem::new(text, LogLevel::Info));
    SEM.get().unwrap().up();
}

pub fn info_type(info: InfoType) {
    QUEUE
        .write()
        .unwrap()
        .push(LogItem::new(i18::get_info(info), LogLevel::Info));
    SEM.get().unwrap().up();
}

pub fn warn(text: String) {
    QUEUE
        .write()
        .unwrap()
        .push(LogItem::new(text, LogLevel::Warn));
    SEM.get().unwrap().up();
}

pub fn error(text: String) {
    QUEUE
        .write()
        .unwrap()
        .push(LogItem::new(text, LogLevel::Error));
    SEM.get().unwrap().up();
}

pub fn error_type(error: ErrorType) {
    QUEUE
        .write()
        .unwrap()
        .push(LogItem::new(i18::get_error(error), LogLevel::Error));
    SEM.get().unwrap().up();
}

pub fn failt(text: String) {
    QUEUE
        .write()
        .unwrap()
        .push(LogItem::new(text, LogLevel::Fault));
    SEM.get().unwrap().up();
}
