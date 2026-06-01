#[cfg(not(unix))]
use std::path::Path;
use std::{
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};

use mcml_names::{i18_items::error_type::ErrorType, names};

use crate::archives::{r7z_runner::R7zProcess, tar_runner::TarProcess, zip_runner::ZipProcess};

pub mod r7z_runner;
pub mod tar_runner;
pub mod zip_runner;

pub enum ArchiveType {
    Zip,
    R7Z,
    Tar,
    TarGz,
    TarXz,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TarMode {
    Gz,
    Xz,
}

impl TarMode {
    /// 根据文件名后缀自动判断（不返回 Result，失败时返回 None）
    pub fn try_from_path(path: &PathBuf) -> Option<Self> {
        let file_name = path.file_name()?.to_string_lossy().to_lowercase();

        if file_name.ends_with(names::NAME_TAR_GZ_EXT) || file_name.ends_with(names::NAME_TGZ_EXT) {
            Some(TarMode::Gz)
        } else if file_name.ends_with(names::NAME_TAR_XZ_EXT)
            || file_name.ends_with(names::NAME_TXZ_EXT)
        {
            Some(TarMode::Xz)
        } else {
            None
        }
    }
}

pub(crate) struct ArchiveProcess {
    gui: Option<Box<dyn IArchiveGui + Send + Sync>>,
    size: AtomicUsize,
    now: AtomicUsize,
}

impl ArchiveProcess {
    pub fn new(gui: Option<Box<dyn IArchiveGui + Send + Sync>>) -> Self {
        Self {
            gui,
            size: AtomicUsize::new(0),
            now: AtomicUsize::new(0),
        }
    }

    pub fn set_count(&self, count: usize) {
        self.size.store(count, Ordering::SeqCst);
        if let Some(gui) = &self.gui {
            gui.start(count);
        }
    }

    pub fn add_now(&self, path: &PathBuf) {
        let now = self.now.fetch_add(1, Ordering::SeqCst) + 1;
        if let Some(gui) = &self.gui {
            let filename = path.display().to_string();
            gui.update(Some(filename), now);
        }
    }
}

pub(crate) trait IArchive: Send + Sync {
    /// 压缩文件夹
    ///
    /// - `archive_file`: 压缩包保存位置
    /// - `pack_dir`: 压缩的路径
    /// - `root_path`: 需要剔除的路径
    /// - `filter`: 过滤的文件
    fn compress(
        &self,
        archive_file: &PathBuf,
        pack_dir: &PathBuf,
        root_path: Option<&PathBuf>,
        filter: &Option<Vec<String>>,
    ) -> Result<(), ErrorType>;
    fn decompress(&self, archive_file: &PathBuf, output_dir: &PathBuf) -> Result<(), ErrorType>;
}

pub trait IArchiveGui: Send + Sync {
    fn start(&self, total: usize);
    fn update(&self, filename: Option<String>, current: usize);
}

fn normalize_path(path: &Path) -> String {
    path.to_string_lossy().to_string().replace('\\', "/") // Windows \ 转换为 /
}

fn should_exclude(path: &Path, patterns: &[String]) -> bool {
    let normalized_path = normalize_path(path);
    patterns.iter().any(|pattern| {
        let normalized_pattern = pattern.replace('\\', "/");
        normalized_path.contains(&normalized_pattern)
    })
}

/// 压缩文件
/// - `archive_type`: 压缩包类型
/// - ``
pub fn compress(
    archive_type: ArchiveType,
    archive_file: &PathBuf,
    pack_dir: &PathBuf,
    root_path: Option<&PathBuf>,
    filter: &Option<Vec<String>>,
    gui: Option<Box<dyn IArchiveGui + Send + Sync>>,
) -> Result<(), ErrorType> {
    let precess: Box<dyn IArchive + Send + Sync> = match archive_type {
        ArchiveType::Zip => Box::new(ZipProcess::new(ArchiveProcess::new(gui))),
        ArchiveType::R7Z => Box::new(R7zProcess::new(ArchiveProcess::new(gui))),
        ArchiveType::Tar => Box::new(TarProcess::new(ArchiveProcess::new(gui), None)),
        ArchiveType::TarGz => {
            Box::new(TarProcess::new(ArchiveProcess::new(gui), Some(TarMode::Gz)))
        }
        ArchiveType::TarXz => {
            Box::new(TarProcess::new(ArchiveProcess::new(gui), Some(TarMode::Xz)))
        }
    };

    precess.compress(archive_file, pack_dir, root_path, filter)
}

/// 压缩文件
/// - `archive_type`: 压缩包类型
/// - ``
pub fn decompress(
    archive_type: ArchiveType,
    archive_file: &PathBuf,
    output_dir: &PathBuf,
    gui: Option<Box<dyn IArchiveGui + Send + Sync>>,
) -> Result<(), ErrorType> {
    let precess: Box<dyn IArchive + Send + Sync> = match archive_type {
        ArchiveType::Zip => Box::new(ZipProcess::new(ArchiveProcess::new(gui))),
        ArchiveType::R7Z => Box::new(R7zProcess::new(ArchiveProcess::new(gui))),
        ArchiveType::Tar => Box::new(TarProcess::new(ArchiveProcess::new(gui), None)),
        ArchiveType::TarGz => {
            Box::new(TarProcess::new(ArchiveProcess::new(gui), Some(TarMode::Gz)))
        }
        ArchiveType::TarXz => {
            Box::new(TarProcess::new(ArchiveProcess::new(gui), Some(TarMode::Xz)))
        }
    };

    precess.decompress(archive_file, output_dir)
}
