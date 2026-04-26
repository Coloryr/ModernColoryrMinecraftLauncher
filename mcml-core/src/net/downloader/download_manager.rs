pub mod download_manager {
    use crossbeam_queue::SegQueue;

    use crate::{
        events::core_stop_event::core_stop_event,
        net::downloader::{
            download_item::DownloadItem, download_task::DownloadTask,
            download_thread::DownloadThread,
        },
    };

    static DOWNLOAD_LIST: SegQueue<DownloadItem> = SegQueue::new();
    static THREADS: Vec<DownloadThread> = Vec::new();
    static TASKS: Vec<DownloadTask> = Vec::new();

    static STOP: bool = false;

    pub fn init() {
        core_stop_event::add_stop_handler(stop);
    }

    fn stop() {
        if STOP {
            return;
        }
        while !DOWNLOAD_LIST.is_empty() {
            DOWNLOAD_LIST.pop();
        }
        for item in TASKS.iter().clone() {
            item.cancel();
        }
        for item in THREADS.iter() {
            item.download_stop();
        }
    }

    pub fn get_state() -> bool {
        false
    }
}
