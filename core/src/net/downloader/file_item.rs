use tokio::io::AsyncRead;

/// 下载项状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DownloadItemState {
    Wait,     // 等待中
    Download, // 下载中
    GetInfo,  // 获取信息
    Pause,    // 暂停
    Init,     // 初始化中
    Action,   // 执行后续操作
    Done,     // 完成
    Error,    // 错误
}

pub trait FileItemLater: Send + Sync {
    fn run(&self, reader: &mut dyn AsyncRead);
}

pub struct FileItemObj {
    /// 文件名
    pub name: String,
    /// 下载地址
    pub url: String,
    /// 文件位置
    pub local: String,
    /// 下载时是否覆盖
    pub overwrite: bool,
    /// 总体大小
    pub all_size: i64,
    /// 已下载大小
    pub now_size: i64,
    /// 下载状态
    pub state: DownloadItemState,
    /// 错误次数
    pub error: i32,
    /// 校验
    pub md5: Option<String>,
    /// 校验
    pub sha1: Option<String>,
    /// 校验
    pub sha256: Option<String>,
    /// 下载完成后调用
    pub later: Option<Box<dyn FileItemLater>>,
}

impl FileItemObj {
    pub fn new(name: String, url: String, local: String) -> Self {
        FileItemObj {
            name,
            url,
            local,
            overwrite: false,
            all_size: 0,
            now_size: 0,
            state: DownloadItemState::Init,
            error: 0,
            md5: None,
            sha1: None,
            sha256: None,
            later: None,
        }
    }

    pub fn with_later(mut self, later: impl FileItemLater + 'static) -> Self {
        self.later = Some(Box::new(later));
        self
    }

    pub fn with_md5(mut self, md5: String) -> Self {
        self.md5 = Some(md5);
        self
    }

    pub fn with_overwrite(mut self, overwrite: bool) -> Self {
        self.overwrite = overwrite;
        self
    }

    pub fn progress(&self) -> f64 {
        if self.all_size > 0 {
            (self.now_size as f64 / self.all_size as f64) * 100.0
        } else {
            0.0
        }
    }
}
