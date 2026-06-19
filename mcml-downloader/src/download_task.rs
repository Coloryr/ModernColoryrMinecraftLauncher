use crossbeam_queue::SegQueue;
use mcml_base::file_item::FileItemObj;
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;

use crate::{download_item::DownloadItem, gen_task_id, task_done, update_task};

/// 下载任务
pub(crate) struct DownloadTask {
    pub id: u64,
    /// 取消
    cancel: CancellationToken,
    /// 下载项目列表
    items: SegQueue<DownloadItem>,
    pub total_size: usize,
    pub completed_count: usize,
    pub failed_count: usize,
    sem: Semaphore,
}

impl DownloadTask {
    /// 创建下载任务
    /// - `items`: 需要下载的文件
    pub fn new(items: Vec<FileItemObj>) -> Self {
        let vec = SegQueue::new();

        for item in items.into_iter().map(|item| DownloadItem::new(item)) {
            vec.push(item);
        }

        let size = vec.len();

        DownloadTask {
            id: gen_task_id(),
            items: vec,
            total_size: size,
            completed_count: 0,
            failed_count: 0,
            cancel: CancellationToken::new(),
            sem: Semaphore::new(0),
        }
    }

    pub fn done(&mut self) {
        self.completed_count += 1;
        update_task(self.id, self.progress());
        if self.items.is_empty() {
            task_done(self);
        }
    }

    pub fn fail(&mut self) {
        self.failed_count += 1;
        update_task(self.id, self.progress());
        if self.items.is_empty() {
            task_done(self);
        }
    }

    /// 取一个下载项目
    pub fn get_item(&self) -> Option<DownloadItem> {
        self.items.pop()
    }

    /// 等待任务结束
    pub async fn wait_done(&self) -> bool {
        let _ = self.sem.acquire().await.unwrap();

        self.total_size == self.completed_count
    }

    /// 取消下载任务
    pub fn cancel(&self) {
        self.cancel.cancel();
    }

    /// 获取下载进度
    fn progress(&self) -> f64 {
        if self.total_size > 0 {
            ((self.completed_count + self.failed_count) as f64 / self.total_size as f64) * 100.0
        } else {
            0.0
        }
    }
}
