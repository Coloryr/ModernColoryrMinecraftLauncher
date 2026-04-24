use std::sync::Arc;

use tokio::sync::Mutex;

use crate::{net::downloader::download_task::DownloadTask, objs::file_item_obj::FileItemObj};

pub struct DownloadItem {
    pub task: Arc<Mutex<DownloadTask>>,
    pub file: Box<FileItemObj>,
}

impl DownloadItem {
    pub fn new(task: Arc<Mutex<DownloadTask>>, file: Box<FileItemObj>) -> Self {
        DownloadItem { task, file }
    }
}
