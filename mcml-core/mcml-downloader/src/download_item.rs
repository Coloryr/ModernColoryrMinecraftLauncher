//! 下载项目模块
//!
//! 定义单个下载文件的状态跟踪结构体 [`DownloadItem`]，
//! 使用原子变量实现线程安全的状态和进度更新。

use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};

use mcml_base::file_item::FileItemObj;

/// 下载项的状态机
///
/// ```text
/// Wait → Init → GetInfo → Download → Done
///                    ↓          ↓
///                 Action     Error
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DownloadItemState {
    /// 等待分配到下载线程
    Wait,
    /// 正在下载文件数据
    Download,
    /// 正在获取文件元信息（大小等）
    GetInfo,
    /// 暂停
    Pause,
    /// 初始化中
    Init,
    /// 执行下载后处理（解压等）
    Action,
    /// 下载完成
    Done,
    /// 下载出错
    Error,
}

impl DownloadItemState {
    /// 将状态转换为整数（用于原子存储）
    pub fn state_to_int(&self) -> u32 {
        match self {
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

    /// 从整数恢复状态
    pub fn int_to_state(value: u32) -> DownloadItemState {
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

/// 单个下载文件的状态跟踪
///
/// 所有字段使用原子变量，支持多线程安全的读写。
pub struct DownloadItem {
    /// 文件基本信息（URL、路径、哈希等）
    pub base: FileItemObj,
    /// 下载时是否覆盖已存在的文件
    pub overwrite: bool,
    /// 文件总大小（字节）
    all_size: AtomicU64,
    /// 已下载大小（字节）
    now_size: AtomicU64,
    /// 当前下载状态
    state: AtomicU32,
    /// 累计错误次数
    error: AtomicU32,
}

impl DownloadItem {
    /// 创建下载项目
    ///
    /// # 参数
    ///
    /// - `file`: 文件基本信息
    pub fn new(file: FileItemObj) -> Self {
        DownloadItem {
            base: file,
            overwrite: false,
            all_size: AtomicU64::new(0),
            now_size: AtomicU64::new(0),
            state: AtomicU32::new(0),
            error: AtomicU32::new(0),
        }
    }

    /// 设置是否覆盖已存在文件（构建器模式）
    pub fn set_overwrite(mut self, overwrite: bool) -> Self {
        self.overwrite = overwrite;
        self
    }

    /// 获取当前下载进度百分比（0.0–100.0）
    pub fn progress(&self) -> f64 {
        let size = self.all_size.load(Ordering::Acquire);
        if size > 0 {
            (self.now_size.load(Ordering::Acquire) as f64 / size as f64) * 100.0
        } else {
            0.0
        }
    }

    /// 累加已下载字节数
    pub fn add_progress(&self, size: u64) {
        self.now_size.fetch_add(size, Ordering::Relaxed);
    }

    /// 设置已下载字节数（用于断点续传恢复）
    pub fn set_now_size(&self, size: u64) {
        self.now_size.store(size, Ordering::Relaxed);
    }

    /// 设置文件总大小
    pub fn set_all_size(&self, size: u64) {
        self.all_size.store(size, Ordering::Relaxed);
    }

    /// 获取文件总大小
    pub fn get_all_size(&self) -> u64 {
        self.all_size.load(Ordering::Acquire)
    }

    /// 累加错误计数
    pub fn add_error(&self) {
        self.error.fetch_add(1, Ordering::Relaxed);
    }

    /// 设置当前下载状态
    pub fn set_state(&self, state: DownloadItemState) {
        self.state.store(state.state_to_int(), Ordering::Relaxed);
    }

    /// 获取当前下载状态
    pub fn get_state(&self) -> DownloadItemState {
        DownloadItemState::int_to_state(self.state.load(Ordering::Acquire))
    }

    /// 获取已下载字节数
    pub fn get_now_size(&self) -> u64 {
        self.now_size.load(Ordering::Acquire)
    }
}
