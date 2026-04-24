use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt, AsyncRead};
use tokio::fs::OpenOptions;
use reqwest::Client;
use crate::types::{FileItemObj, DownloadItemState, FileItemLater};

pub struct Downloader {
    client: Client,
}

impl Downloader {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap(),
        }
    }
    
    /// 下载单个文件
    pub async fn download_file(
        &self,
        item: Arc<tokio::sync::Mutex<FileItemObj>>,
        on_progress: Option<Box<dyn Fn(u64, u64) + Send + Sync>>,
    ) -> Result<(), String> {
        // 获取文件信息
        let (url, local_path, overwrite, mut later) = {
            let item_guard = item.lock().await;
            (
                item_guard.url.clone(),
                item_guard.local.clone(),
                item_guard.overwrite,
            )
        };
        
        // 检查文件是否已存在
        let path = std::path::Path::new(&local_path);
        if path.exists() && !overwrite {
            // 文件已存在，直接标记完成并执行 later
            let mut item_guard = item.lock().await;
            item_guard.state = DownloadItemState::Done;
            item_guard.all_size = path.metadata().map(|m| m.len() as i64).unwrap_or(0);
            item_guard.now_size = item_guard.all_size;
            
            if let Some(later) = item_guard.later.take() {
                // 创建文件读取器
                let file = tokio::fs::File::open(path).await.unwrap();
                let mut reader = tokio::io::BufReader::new(file);
                later.run(&mut reader);
            }
            return Ok(());
        }
        
        // 获取文件大小
        let head_response = self.client.head(&url).send().await.map_err(|e| e.to_string())?;
        let total_size = head_response
            .headers()
            .get("content-length")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(0);
        
        // 更新总大小
        {
            let mut item_guard = item.lock().await;
            item_guard.all_size = total_size as i64;
            item_guard.state = DownloadItemState::Download;
        }
        
        // 检查是否支持断点续传
        let supports_resume = head_response
            .headers()
            .get("accept-ranges")
            .map(|v| v.to_str().unwrap_or("") == "bytes")
            .unwrap_or(false);
        
        // 获取已下载大小（断点续传）
        let existing_size = if supports_resume && path.exists() {
            path.metadata().map(|m| m.len()).unwrap_or(0)
        } else {
            0
        };
        
        // 构建请求
        let mut request = self.client.get(&url);
        if existing_size > 0 && existing_size < total_size {
            request = request.header("Range", format!("bytes={}-", existing_size));
        }
        
        let response = request.send().await.map_err(|e| e.to_string())?;
        let status = response.status();
        
        if !status.is_success() && status.as_u16() != 206 {
            return Err(format!("HTTP error: {}", status));
        }
        
        // 打开文件（追加模式）
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .append(existing_size > 0)
            .open(&local_path)
            .await
            .map_err(|e| e.to_string())?;
        
        // 流式下载
        let mut stream = response.bytes_stream();
        use futures_util::StreamExt;
        
        let mut downloaded = existing_size;
        let total = total_size;
        
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| e.to_string())?;
            file.write_all(&chunk).await.map_err(|e| e.to_string())?;
            downloaded += chunk.len() as u64;
            
            // 更新进度
            {
                let mut item_guard = item.lock().await;
                item_guard.now_size = downloaded as i64;
            }
            
            if let Some(ref callback) = on_progress {
                callback(downloaded, total);
            }
        }
        
        file.flush().await.map_err(|e| e.to_string())?;
        
        // 下载完成，执行后续操作
        let mut item_guard = item.lock().await;
        item_guard.state = DownloadItemState::Action;
        
        if let Some(later) = item_guard.later.take() {
            // 打开文件读取器传给 later
            let file = tokio::fs::File::open(&local_path).await.map_err(|e| e.to_string())?;
            let mut reader = tokio::io::BufReader::new(file);
            
            // 在单独的线程中执行 later（避免阻塞）
            tokio::task::spawn_blocking(move || {
                later.run(&mut reader);
            }).await.map_err(|e| format!("Later execution error: {:?}", e))?;
        }
        
        item_guard.state = DownloadItemState::Done;
        
        Ok(())
    }
}