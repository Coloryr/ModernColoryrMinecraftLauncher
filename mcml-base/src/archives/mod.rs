use std::{
    path::{Path, PathBuf},
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
    pub fn try_from_path(path: &Path) -> Option<Self> {
        let file_name = path.file_name()?.to_string_lossy().to_lowercase();

        if file_name.ends_with(names::TAR_GZ_EXT) || file_name.ends_with(names::TGZ_EXT) {
            Some(TarMode::Gz)
        } else if file_name.ends_with(names::TAR_XZ_EXT)
            || file_name.ends_with(names::TXZ_EXT)
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
    fn compress(
        &self,
        archive_file: &Path,
        pack_dir: &Path,
        root_path: Option<&Path>,
        filter: &Option<Vec<String>>,
    ) -> Result<(), ErrorType>;
    fn decompress(&self, archive_file: &Path, output_dir: &Path) -> Result<(), ErrorType>;
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
/// - `archive_file`: 压缩包位置
/// - `pack_dir`: 需要压缩的位置
/// - `root_path`: 相对路径
/// - `filter`: 文件过滤
/// - `gui`: 显示回调
pub fn compress<P: AsRef<Path>>(
    archive_type: ArchiveType,
    archive_file: P,
    pack_dir: P,
    root_path: Option<P>,
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

    precess.compress(
        archive_file.as_ref(),
        pack_dir.as_ref(),
        root_path.as_ref().map(|p| p.as_ref()),
        filter,
    )
}

/// 压缩文件
/// - `archive_type`: 压缩包类型
/// - `archive_file`: 压缩包位置
/// - `output_dir`: 解压路径
/// - `gui`: 显示回调
pub fn decompress<P: AsRef<Path>>(
    archive_type: ArchiveType,
    archive_file: P,
    output_dir: P,
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

    precess.decompress(archive_file.as_ref(), output_dir.as_ref())
}
