use crossbeam_queue::SegQueue;
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

static LOG_QUEUE: RwLock<SegQueue<LogItem>> = RwLock::new(SegQueue::new());
static LOG_LOCAL: OnceLock<PathBuf> = OnceLock::new();
static LOG_FILE: OnceLock<Mutex<BufWriter<File>>> = OnceLock::new();
static LOG_RUN: AtomicBool = AtomicBool::new(true);
static LOG_SEM: OnceLock<Arc<Semaphore>> = OnceLock::new();

pub fn start(local: String) {
    LOG_SEM.get_or_init(|| Arc::new(Semaphore::new(0)));

    let log_path = PathBuf::from(local).join("logs.log");

    let file = match OpenOptions::new()
        .create(true)
        .append(true)
        .write(true)
        .open(&log_path)
    {
        Ok(f) => f,
        Err(e) => {
            panic!("log open fail: {} - {}", log_path.display(), e);
        }
    };

    if LOG_LOCAL.set(log_path).is_err() {
        panic!("LOG_LOCAL fail");
    }

    if LOG_FILE.set(Mutex::new(BufWriter::new(file))).is_err() {
        panic!("LOG_FILE fail");
    }

    let thread = thread::Builder::new()
        .name("Log thread".into())
        .spawn(|| run());

    if thread.is_err() {
        panic!("Log Thread start fail")
    }
}

pub fn stop() {
    LOG_RUN.store(false, Ordering::Release);
}

fn save() {
    let log = LOG_QUEUE.read().unwrap();
    let mut file = LOG_FILE.get().unwrap().lock().unwrap();

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
        }
    }
}

pub fn run() {
    while LOG_RUN.load(Ordering::Acquire) {
        LOG_SEM.get().unwrap().down();

        save();
    }

    save();
}

pub fn info(text: String) {
    LOG_QUEUE
        .write()
        .unwrap()
        .push(LogItem::new(text, LogLevel::Info));
    LOG_SEM.get().unwrap().up();
}

pub fn warn(text: String) {
    LOG_QUEUE
        .write()
        .unwrap()
        .push(LogItem::new(text, LogLevel::Warn));
    LOG_SEM.get().unwrap().up();
}

pub fn error(text: String) {
    LOG_QUEUE
        .write()
        .unwrap()
        .push(LogItem::new(text, LogLevel::Error));
    LOG_SEM.get().unwrap().up();
}

pub fn failt(text: String) {
    LOG_QUEUE
        .write()
        .unwrap()
        .push(LogItem::new(text, LogLevel::Fault));
    LOG_SEM.get().unwrap().up();
}
