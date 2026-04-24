use std::sync::Arc;
use crossbeam_queue::SegQueue;
use crate::types::FileItemObj;
use crate::downloader::Downloader;

pub struct DownloadWorker {
    id: usize,
    queue: Arc<SegQueue<Arc<tokio::sync::Mutex<FileItemObj>>>>,
    downloader: Arc<Downloader>,
}

impl DownloadWorker {
    pub fn new(
        id: usize,
        queue: Arc<SegQueue<Arc<tokio::sync::Mutex<FileItemObj>>>>,
    ) -> Self {
        Self {
            id,
            queue,
            downloader: Arc::new(Downloader::new()),
        }
    }
    
    pub async fn run(&self) {
        println!("Worker {} started", self.id);
        
        loop {
            // 从队列中取出下载项
            if let Some(item) = self.queue.pop() {
                println!("Worker {} downloading item", self.id);
                
                // 更新状态
                {
                    let mut item_guard = item.lock().await;
                    if item_guard.state != crate::types::DownloadItemState::Wait {
                        continue;
                    }
                    item_guard.state = crate::types::DownloadItemState::GetInfo;
                }
                
                // 执行下载
                let result = self.downloader.download_file(item.clone(), None).await;
                
                match result {
                    Ok(_) => {
                        println!("Worker {} completed download", self.id);
                    }
                    Err(e) => {
                        println!("Worker {} failed: {}", self.id, e);
                        let mut item_guard = item.lock().await;
                        item_guard.error += 1;
                        item_guard.state = crate::types::DownloadItemState::Error;
                    }
                }
            } else {
                // 队列为空，短暂休眠
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        }
    }
}