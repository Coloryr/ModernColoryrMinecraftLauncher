use std::path::PathBuf;
#[cfg(unix)]
use std::{fs, io, path::Path};

use mcml_names::i18_items::error_type::ErrorType;

pub mod r7z_runner;
pub mod zip_runner;

pub enum ArchiveType {
    Zip,
    TarGz,
    R7Z,
}

pub trait IArchive: Send + Sync {
    /// 压缩文件夹
    /// 
    /// - `zip_file`: 压缩包保存位置
    /// - `pack_dir`: 压缩的路径
    /// - `root_path`: 需要剔除的路径
    /// - `filter`: 过滤的文件
    fn compress(
        &self,
        zip_file: &PathBuf,
        pack_dir: &PathBuf,
        root_path: Option<&PathBuf>,
        filter: &Option<Vec<String>>,
    ) -> Result<(), ErrorType>;
    fn decompress(&self, archive_file: &PathBuf, output_dir: &PathBuf) -> Result<(), ErrorType>;
}

pub trait IArchiveGui: Send + Sync {
    fn start(&self, total: usize);
    fn zip_update(&self, filename: Option<String>, current: usize);
    fn file_rename(&self, old_name: &str) -> bool;
    fn done(&self);
}

/// 创建符号链接（跨平台）
#[cfg(unix)]
fn make_symlink(target_path: &Path, link_target: &str) -> io::Result<()> {
    std::os::unix::fs::symlink(link_target, target_path)
}

#[cfg(windows)]
fn make_symlink(target_path: &Path, link_target: &str) -> io::Result<()> {
    let target = Path::new(link_target);
    if target.is_dir() {
        std::os::windows::fs::symlink_dir(target, target_path)
    } else {
        std::os::windows::fs::symlink_file(target, target_path)
    }
}

#[cfg(not(any(unix, windows)))]
fn make_symlink(target_path: &Path, link_target: &str) -> io::Result<()> {
    // 不支持符号链接的平台：写为普通文件（内容为链接目标）
    let mut f = File::create(target_path)?;
    f.write_all(link_target.as_bytes())?;
    Ok(())
}

/// 设置文件/目录的 Unix 权限（仅 Unix）
#[cfg(unix)]
fn set_perms(path: &Path, mode: u32) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(mode))
}

/// 非 Unix 平台无需恢复权限
#[cfg(not(unix))]
fn set_perms(_path: &Path, _mode: u32) -> io::Result<()> {
    Ok(())
}
