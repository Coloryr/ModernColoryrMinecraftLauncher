//! 压缩包处理模块
//!
//! 提供多种压缩格式的压缩和解压功能：
//!
//! | 格式 | 实现 |
//! |------|------|
//! | Zip | [`zip_reader`] — 基于 `zip` crate |
//! | Tar / TarGz / TarXz | [`tar_reader`] — 基于 `tar` + `flate2`/`xz2` crate |
//! | 7z | [`r7z_reader`] — 基于 `sevenz-rust` crate |
//!
//! # 进度回调
//!
//! 通过 [`IBaseArchiveGui`] trait 支持压缩/解压进度的 UI 回调通知。
//! 使用 [`ArchiveProcess`] 内部追踪进度状态。

use std::{
    io::Read,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

use crate::path_helper;
use mcml_names::{
    i18_items::error_type::{CoreResult, ErrorType},
    names,
};

pub mod base_archive;
pub mod r7z_reader;
pub mod tar_reader;
pub mod zip_reader;

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
        } else if file_name.ends_with(names::TAR_XZ_DOT_EXT)
            || file_name.ends_with(names::TXZ_DOT_EXT)
        {
            Some(TarMode::Xz)
        } else {
            None
        }
    }
}

pub(crate) struct ArchiveProcess {
    /// 进度回调
    gui: Option<Arc<dyn IBaseArchiveGui>>,
    /// 总文件数
    size: AtomicUsize,
    /// 当前处理数
    now: AtomicUsize,
}

impl ArchiveProcess {
    /// 创建进度追踪器
    pub fn new(gui: Option<Arc<dyn IBaseArchiveGui>>) -> Self {
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

    /// 解压完成回调
    pub fn done(&self) {
        if let Some(gui) = &self.gui {
            gui.done();
        }
    }

    /// 检查条目名是否含非法字符，非法时替换为 `_`，并询问 GUI 是否同意替换。
    ///
    /// 返回替换后的条目名；名字合法时返回原名。GUI 不同意替换时返回 `TaskCancel`。
    pub fn check_name(&self, name: &str) -> CoreResult<String> {
        if name
            .split(['/', '\\'])
            .filter(|seg| !seg.is_empty() && *seg != "." && *seg != "..")
            .all(|seg| !path_helper::file_has_invalid_chars(seg))
        {
            return Ok(name.to_string());
        }
        // 非法字符替换为 `_`，GUI 同意则使用，不同意则取消
        let safe_name = replace_invalid_name(name);
        match &self.gui {
            Some(gui) if gui.file_rename(name) => Ok(safe_name),
            Some(_) => Err(ErrorType::TaskCancel),
            None => Ok(safe_name),
        }
    }
}

/// 已打开的压缩包读取句柄：统一各格式的查文件与单文件解压。
///
/// 文件句柄在打开后保持持有，各格式内部决定如何复用（zip 缓存 [`zip::ZipArchive`]，
/// tar / 7z 每次操作从持有的文件句柄克隆出独立文件描述符再重建读取器）。
pub(crate) trait ArchiveHandle: Send {
    /// 读取所有条目
    fn read_entries(&mut self) -> CoreResult<Vec<ArchiveEntryInfo>>;
    /// 读取单个条目内容到内存
    fn read(&mut self, name: &str) -> CoreResult<Vec<u8>>;
    /// 流式读取单个条目
    fn read_stream(&mut self, name: &str) -> CoreResult<Box<dyn Read>>;
    /// 提取单个条目到指定路径
    fn extract_file(
        &mut self,
        name: &str,
        output_path: &Path,
        gui: Option<&dyn IBaseArchiveGui>,
    ) -> CoreResult<()>;
    /// 就地追加文件（仅支持随机可写的格式；其余返回 `InvalidOperation`）
    fn add_files(&mut self, _files: &[(PathBuf, PathBuf)]) -> CoreResult<()> {
        Err(ErrorType::InvalidOperation)
    }
    /// 就地追加内存数据（仅支持随机可写的格式；其余返回 `InvalidOperation`）
    fn add_data(&mut self, _name: &str, _data: &[u8]) -> CoreResult<()> {
        Err(ErrorType::InvalidOperation)
    }
}

/// 压缩包处理显示回调
pub trait IBaseArchiveGui: Send + Sync {
    /// 开始处理压缩包
    ///
    /// - `total`: 总计需要处理的数量
    fn start(&self, total: usize);
    /// 更新压缩包处理信息
    ///
    /// - `filename`: 当前文件名
    /// - `current`: 当前处理数量
    fn update(&self, filename: Option<String>, current: usize);
    /// 解压完成
    fn done(&self);
    /// 文件名含非法字符时询问是否同意替换，返回是否同意（不同意时调用方返回 `TaskCancel`）
    fn file_rename(&self, name: &str) -> bool;
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

/// 替换条目名中的非法字符为 `_`（按路径段处理，保留目录结构）。
pub(crate) fn replace_invalid_name(name: &str) -> String {
    name.split(['/', '\\'])
        .map(|seg| path_helper::replace_file_name(seg))
        .collect::<Vec<_>>()
        .join("/")
}

/// 压缩文件（委托给 [`BaseArchive::compress`]，忽略返回的句柄）。
pub fn compress<P: AsRef<Path>>(
    archive_type: ArchiveType,
    archive_file: P,
    pack_dir: P,
    root_path: Option<P>,
    filter: &Option<Vec<String>>,
    gui: Option<Arc<dyn IBaseArchiveGui>>,
) -> CoreResult<()> {
    BaseArchive::compress(archive_type, archive_file, pack_dir, root_path, filter, gui)?;
    Ok(())
}

/// 解压文件（委托给 [`BaseArchive::decompress`]）。
pub fn decompress<P: AsRef<Path>>(
    archive_type: ArchiveType,
    archive_file: P,
    output_dir: P,
    gui: Option<Arc<dyn IBaseArchiveGui>>,
) -> CoreResult<()> {
    BaseArchive::decompress(archive_type, archive_file, output_dir, gui)
}
