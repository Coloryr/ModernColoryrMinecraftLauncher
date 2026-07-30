//! 压缩包处理模块
//!
//! 提供多种压缩格式的压缩和解压功能：
//!
//! | 格式 | 实现 |
//! |------|------|
//! | Zip | [`zip_runner`] — 基于 `zip` crate |
//! | Tar / TarGz / TarXz | [`tar_runner`] — 基于 `tar` + `flate2`/`xz2` crate |
//! | 7z | [`r7z_runner`] — 基于 `sevenz-rust` crate |
//!
//! # 进度回调
//!
//! 通过 [`IArchiveGui`] trait 支持压缩/解压进度的 UI 回调通知。
//! 使用 [`ArchiveProcess`] 内部追踪进度状态。

use std::{
    path::{Path, PathBuf}, sync::{Arc, atomic::{AtomicUsize, Ordering}},
};

use mcml_names::{i18_items::error_type::CoreResult, names};

pub mod base_archive;
pub mod r7z_runner;
pub mod tar_runner;
pub mod zip_runner;

pub use base_archive::{ArchiveEntryInfo, BaseArchive};

/// 压缩包类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchiveType {
    Zip,
    R7Z,
    Tar,
    TarGz,
    TarXz,
}

/// 压缩模式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TarMode {
    Gz,
    Xz,
}

impl TarMode {
    /// 根据文件名后缀自动判断（不返回 Result，失败时返回 None）
    pub fn try_from_path(path: &Path) -> Option<Self> {
        let file_name = path.file_name()?.to_string_lossy().to_lowercase();

        if file_name.ends_with(names::TAR_GZ_DOT_EXT) || file_name.ends_with(names::TGZ_DOT_EXT) {
            Some(TarMode::Gz)
        } else if file_name.ends_with(names::TAR_XZ_DOT_EXT) || file_name.ends_with(names::TXZ_DOT_EXT) {
            Some(TarMode::Xz)
        } else {
            None
        }
    }
}

pub(crate) struct ArchiveProcess {
    /// 进度回调
    gui: Option<Arc<dyn IArchiveGui>>,
    /// 总文件数
    size: AtomicUsize,
    /// 当前处理数
    now: AtomicUsize,
}

impl ArchiveProcess {
    /// 创建进度追踪器
    pub fn new(gui: Option<Arc<dyn IArchiveGui>>) -> Self {
        Self {
            gui,
            size: AtomicUsize::new(0),
            now: AtomicUsize::new(0),
        }
    }

    /// 设置总文件数
    pub fn set_count(&self, count: usize) {
        self.size.store(count, Ordering::SeqCst);
        if let Some(gui) = &self.gui {
            gui.start(count);
        }
    }

    /// 更新当前处理进度
    pub fn add_now(&self, path: &PathBuf) {
        let now = self.now.fetch_add(1, Ordering::SeqCst) + 1;
        if let Some(gui) = &self.gui {
            let filename = path.display().to_string();
            gui.update(Some(filename), now);
        }
    }
}

/// 压缩包执行器
pub(crate) trait ArchiveRun: Send + Sync {
    /// 压缩
    fn compress(
        &self,
        archive_file: &Path,
        pack_dir: &Path,
        root_path: Option<&Path>,
        filter: &Option<Vec<String>>,
    ) -> CoreResult<()>;
    /// 解压
    fn decompress(&self, archive_file: &Path, output_dir: &Path) -> CoreResult<()>;
}

/// 压缩包处理显示回调
pub trait IArchiveGui: Send + Sync {
    /// 开始处理压缩包
    /// 
    /// - `total`: 总计需要处理的数量
    fn start(&self, total: usize);
    /// 更新压缩包处理信息
    /// 
    /// - `filename`: 当前文件名
    /// - `current`: 当前处理数量
    fn update(&self, filename: Option<String>, current: usize);
}

/// 归一化路径分隔符为 `/`
fn normalize_path(path: &Path) -> String {
    path.to_string_lossy().to_string().replace('\\', "/") // Windows \ 转换为 /
}

/// 检查路径是否匹配任一排除规则
fn should_exclude(path: &Path, patterns: &[String]) -> bool {
    let normalized_path = normalize_path(path);
    patterns.iter().any(|pattern| {
        let normalized_pattern = pattern.replace('\\', "/");
        normalized_path.contains(&normalized_pattern)
    })
}

/// 压缩文件（委托给 [`BaseArchive::compress`]，忽略返回的句柄）。
pub fn compress<P: AsRef<Path>>(
    archive_type: ArchiveType,
    archive_file: P,
    pack_dir: P,
    root_path: Option<P>,
    filter: &Option<Vec<String>>,
    gui: Option<Arc<dyn IArchiveGui>>,
) -> CoreResult<()> {
    BaseArchive::compress(archive_type, archive_file, pack_dir, root_path, filter, gui)?;
    Ok(())
}

/// 解压文件（委托给 [`BaseArchive::decompress`]）。
pub fn decompress<P: AsRef<Path>>(
    archive_type: ArchiveType,
    archive_file: P,
    output_dir: P,
    gui: Option<Arc<dyn IArchiveGui>>,
) -> CoreResult<()> {
    BaseArchive::decompress(archive_type, archive_file, output_dir, gui)
}
