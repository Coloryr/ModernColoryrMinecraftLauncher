pub mod download_item;
pub mod download_task;

mod download_thread;

use std::sync::RwLock;

use crossbeam_queue::SegQueue;

use crate::{download_item::DownloadItem, download_task::DownloadTask, download_thread::DownloadThread};

static LIST: SegQueue<DownloadItem> = SegQueue::new();
static THREADS: RwLock<Vec<DownloadThread>> = RwLock::new(Vec::new());
static TASKS: RwLock<Vec<DownloadTask>> = RwLock::new(Vec::new());

static STOP: bool = false;

pub fn start() {
    let binding = mcml_config::CONFIG.read().unwrap();
    let config = binding.get().unwrap();
    let mut thread = config.http.download_thread;
    if thread <= 0 {
        thread = 5;
    }

    let mut list= THREADS.write().unwrap();
    for index in 0..thread {
        list.push(DownloadThread::new(index));
    }
}

pub fn stop() {
    if STOP {
        return;
    }
    while !LIST.is_empty() {
        LIST.pop();
    }
    for item in TASKS.read().unwrap().iter() {
        item.cancel();
    }
    for item in THREADS.read().unwrap().iter() {
        item.download_stop();
    }
}

pub fn get_state() -> bool {
    false
}
