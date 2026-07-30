//! 下载任务模块
//!
//! 定义批量下载任务的结构体 [`DownloadTask`]，
//! 管理一组文件的下载队列、进度统计和完成通知。

use std::sync::atomic::{AtomicUsize, Ordering};

use crossbeam_queue::SegQueue;
use mcml_base::file_item::FileItemObj;
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;

use crate::download_item::DownloadItem;

/// 下载任务
///
/// 封装一组待下载文件的队列，提供进度跟踪、取消和完成等待功能。
/// 使用无锁队列 [`SegQueue`] 存储待下载项，支持多线程并发取件。
pub(crate) struct DownloadTask {
    /// 任务编号（全局唯一自增 ID）
    pub id: u64,
    /// 取消令牌，用于批量取消任务中的所有下载
    cancel: CancellationToken,
    /// 待下载文件队列（无锁）
    items: SegQueue<DownloadItem>,
    /// 文件总数量
    pub total_size: usize,
    /// 已完成数量
    pub completed_count: AtomicUsize,
    /// 失败数量
    pub failed_count: AtomicUsize,
    /// 任务完成信号量（调用 `wait_done()` 时阻塞直到任务完成）
    sem: Semaphore,
}

impl DownloadTask {
    /// 创建下载任务
    ///
    /// # 参数
    ///
    /// - `items`: 需要下载的文件信息列表
    pub fn new(items: Vec<FileItemObj>) -> Self {
        let vec = SegQueue::new();

        for item in items.into_iter().map(|item| DownloadItem::new(item)) {
            vec.push(item);
        }

        let size = vec.len();

        DownloadTask {
            id: crate::gen_task_id(),
            items: vec,
            total_size: size,
            completed_count: AtomicUsize::new(0),
            failed_count: AtomicUsize::new(0),
            cancel: CancellationToken::new(),
            sem: Semaphore::new(0),
        }
    }

    /// 检查任务是否完成，完成时发送信号
    fn check_done(&self) {
        crate::update_task(self.id, self.progress());
        if self.items.is_empty() {
            crate::task_done(self);
            self.sem.add_permits(1);
        }
    }

    /// 标记一个文件下载成功
    pub fn done(&self) {
        self.completed_count.fetch_add(1, Ordering::SeqCst);
        self.check_done();
    }

    /// 标记一个文件下载失败
    pub fn fail(&self) {
        self.failed_count.fetch_add(1, Ordering::SeqCst);
        self.check_done();
    }

    /// 从队列中取出一个待下载文件（无锁操作）
    pub fn get_item(&self) -> Option<DownloadItem> {
        self.items.pop()
    }

    /// 异步等待任务全部完成
    ///
    /// # 返回值
    ///
    /// `true` — 全部文件下载成功
    /// `false` — 有文件下载失败
    pub async fn wait_done(&self) -> bool {
        let _ = self.sem.acquire().await.unwrap();

        self.total_size == self.completed_count.load(Ordering::SeqCst)
    }

    /// 取消此下载任务
    pub fn cancel(&self) {
        self.cancel.cancel();
    }

    /// 获取下载进度百分比（包含已完成和失败的文件）
    fn progress(&self) -> f64 {
        if self.total_size > 0 {
            let completed = self.completed_count.load(Ordering::SeqCst);
            let failed = self.failed_count.load(Ordering::SeqCst);
            ((completed + failed) as f64 / self.total_size as f64) * 100.0
        } else {
            0.0
        }
    }
}
