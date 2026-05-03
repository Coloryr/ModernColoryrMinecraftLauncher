use std::sync::Arc;

use tokio_util::sync::CancellationToken;

use crate::download_item::DownloadItem;

/// 下载任务
pub struct DownloadTask {
    /// 取消
    cancel: CancellationToken,
    // /// gui界面
    // gui: Option<Box<dyn DownloadGuiHandel>>,
    // /// p_gui 进度条
    // p_gui: Option<Box<dyn ProgressGuiHandel>>,
    /// 下载项目列表
    pub items: Vec<Arc<tokio::sync::Mutex<DownloadItem>>>,
    pub total_size: i64,
    pub downloaded_size: i64,
    pub completed_count: usize,
    pub failed_count: usize,
}

impl DownloadTask {
    pub fn new(
        // gui: Option<Box<dyn DownloadGuiHandel>>,
        // p_gui: Option<Box<dyn ProgressGuiHandel>>,
    ) -> Self {
        DownloadTask {
            items: Vec::new(),
            total_size: 0,
            downloaded_size: 0,
            completed_count: 0,
            failed_count: 0,
            cancel: CancellationToken::new(),
            // gui: gui,
            // p_gui: p_gui,
        }
    }

    pub fn cancel(&self) {
        self.cancel.cancel();
    }

    pub fn add_item(&mut self, item: DownloadItem) {
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
