use std::sync::{
    Mutex,
    atomic::{AtomicU32, AtomicU64, Ordering},
};

use mcml_base::file_item::FileItemObj;

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

pub struct DownloadItem {
    /// 文件信息
    pub base: FileItemObj,
    /// 下载时是否覆盖
    pub overwrite: bool,
    /// 总体大小
    all_size: u64,
    /// 已下载大小
    now_size: AtomicU64,
    /// 下载状态
    state: AtomicU32,
    /// 错误次数
    error: AtomicU32,
}

impl DownloadItem {
    /// 创建下载项目
    /// - `file`: 文件信息
    pub fn new(file: FileItemObj) -> Self {
        DownloadItem {
            base: file,
            overwrite: false,
            all_size: 0,
            now_size: AtomicU64::new(0),
            state: AtomicU32::new(0),
            error: AtomicU32::new(0),
        }
    }

    /// 设置是否覆盖
    pub fn set_overwrite(mut self, overwrite: bool) -> Self {
        self.overwrite = overwrite;
        self
    }

    /// 获取当前文件进度
    pub fn progress(&self) -> f64 {
        if self.all_size > 0 {
            (self.now_size.load(Ordering::Acquire) as f64 / self.all_size as f64) * 100.0
        } else {
            0.0
        }
    }

    pub fn add_progress(&self, size: u64) {
        self.now_size.fetch_add(size, Ordering::Relaxed);
    }

    pub fn set_all_size(&mut self, size: u64) {
        self.all_size = size;
    }

    pub fn get_all_size(&self) -> u64 {
        self.all_size
    }

    pub fn add_error(&self) {
        self.error.fetch_add(1, Ordering::Relaxed);
    }

    pub fn set_state(&self, state: DownloadItemState) {
        self.state
            .store(Self::state_to_int(state), Ordering::Relaxed);
    }

    pub fn get_state(&self) -> DownloadItemState {
        Self::int_to_state(self.state.load(Ordering::Acquire))
    }

    fn state_to_int(state: DownloadItemState) -> u32 {
        match state {
            DownloadItemState::Wait => 0,
            DownloadItemState::Download => 1,
            DownloadItemState::GetInfo => 2,
            DownloadItemState::Pause => 3,
            DownloadItemState::Init => 4,
            DownloadItemState::Action => 5,
            DownloadItemState::Done => 6,
            DownloadItemState::Error => 7,
        }
    }

    fn int_to_state(value: u32) -> DownloadItemState {
        match value {
            0 => DownloadItemState::Wait,
            1 => DownloadItemState::Download,
            2 => DownloadItemState::GetInfo,
            3 => DownloadItemState::Pause,
            4 => DownloadItemState::Init,
            5 => DownloadItemState::Action,
            6 => DownloadItemState::Done,
            7 => DownloadItemState::Error,
            _ => DownloadItemState::Error,
        }
    }
}
