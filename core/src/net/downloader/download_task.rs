use std::sync::Arc;

use tokio_util::sync::CancellationToken;

use crate::{
    net::downloader::file_item::FileItemObj,
    objs::{
        handels::{download_gui_handel::DownloadGuiHandel, progress_gui_handel::ProgressGuiHandel},
    },
};

/// 下载任务
pub struct DownloadTask {
    /// 取消
    cancel: CancellationToken,
    /// gui界面
    gui: Option<Box<dyn DownloadGuiHandel>>,
    /// p_gui 进度条
    p_gui: Option<Box<dyn ProgressGuiHandel>>,
    /// 下载项目列表
    pub items: Vec<Arc<tokio::sync::Mutex<FileItemObj>>>,
    pub total_size: i64,
    pub downloaded_size: i64,
    pub completed_count: usize,
    pub failed_count: usize,
}

impl DownloadTask {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            total_size: 0,
            downloaded_size: 0,
            completed_count: 0,
            failed_count: 0,
        }
    }

    pub fn add_item(&mut self, item: FileItemObj) {
        self.total_size += item.all_size;
        self.items.push(Arc::new(tokio::sync::Mutex::new(item)));
    }

    pub fn progress(&self) -> f64 {
        if self.total_size > 0 {
            (self.downloaded_size as f64 / self.total_size as f64) * 100.0
        } else {
            0.0
        }
    }
}
