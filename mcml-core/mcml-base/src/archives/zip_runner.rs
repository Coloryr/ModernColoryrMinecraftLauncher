#[cfg(windows)]
use mcml_names::i18_items::error_type::CoreResult;
use mcml_names::i18_items::error_type::{
    ArchiveErrorData, ErrorData, ErrorType, FileSystemErrorData,
};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, DateTime, ZipArchive, ZipWriter};

use std::fs::{self};
use std::io::{self, Read};
#[cfg(windows)]
use std::path::Path;
use std::sync::atomic::Ordering;

#[cfg(unix)]
use std::{fs, io, path::Path};

use crate::archives::{self, ArchiveProcess, ArchiveRun};
use crate::path_helper;

pub(crate) struct ZipProcess {
    base: ArchiveProcess,
}

impl ArchiveRun for ZipProcess {
    fn compress(
        &self,
        archive_file: &Path,
        pack_dir: &Path,
        root_path: Option<&Path>,
        filter: &Option<Vec<String>>,
    ) -> Result<(), ErrorType> {
        let root_path = match root_path {
            Some(path) => path,
            None => pack_dir,
        };
        self.zip(archive_file, pack_dir, root_path, filter)
    }

    fn decompress(&self, archive_file: &Path, output_dir: &Path) -> Result<(), ErrorType> {
        self.unzip(archive_file, output_dir)
    }
}

impl ZipProcess {
    pub fn new(base: ArchiveProcess) -> Self {
        Self { base }
    }

    fn zip(
        &self,
        archive_file: &Path,
        pack_dir: &Path,
        root_path: &Path,
        filter: &Option<Vec<String>>,
    ) -> Result<(), ErrorType> {
        let file = path_helper::open_write(archive_file)?;
        let mut zip = ZipWriter::new(file);
        let files = path_helper::get_all_files(pack_dir);

        self.base.set_count(files.len());

        let options = SimpleFileOptions::default()
            .compression_method(CompressionMethod::Deflated)
            .unix_permissions(0o755);

        for path in files {
            self.base.add_now(&path);

            if let Some(patterns) = filter {
                if archives::should_exclude(&path, patterns) {
                    continue;
                }
            }

            if !path.is_dir() {
                let mut buffer = path_helper::open_read(&path)?;

                let relative_path = path.strip_prefix(root_path).unwrap();
                let tempfile = relative_path.to_string_lossy().to_string();

                zip.start_file(&tempfile, options).map_err(|err| {
                    ErrorType::ArchiveError(ArchiveErrorData {
                        source: path.to_string_lossy().to_string(),
                        target: tempfile.clone(),
                        error: err.to_string(),
                    })
                })?;

                std::io::copy(&mut buffer, &mut zip).map_err(|err| {
                    ErrorType::ArchiveError(ArchiveErrorData {
                        source: path.to_string_lossy().to_string(),
                        target: tempfile.clone(),
                        error: err.to_string(),
                    })
                })?;
            } else {
                let relative_path = path.strip_prefix(root_path).unwrap();
                let tempfile = relative_path.to_string_lossy().to_string();

                zip.add_directory(&tempfile, options).map_err(|err| {
                    ErrorType::ArchiveError(ArchiveErrorData {
                        source: path.to_string_lossy().to_string(),
                        target: tempfile.clone(),
                        error: err.to_string(),
                    })
                })?;
            }
        }

        Ok(())
    }

    fn unzip(&self, archive_file: &Path, output_dir: &Path) -> Result<(), ErrorType> {
        let file = path_helper::open_read(archive_file)?;
        let mut archive = ZipArchive::new(file).map_err(|err| {
            ErrorType::ArchiveOpenError(FileSystemErrorData {
                path: archive_file.to_path_buf(),
                error: err.to_string(),
            })
        })?;
        self.base.set_count(archive.len());

        path_helper::create_dir_all(output_dir)?;
        let output_dir_canonical = output_dir.canonicalize().map_err(|err| {
            ErrorType::FileSystemError(FileSystemErrorData {
                path: output_dir.to_path_buf(),
                error: err.to_string(),
            })
        })?;

        // 收集需要恢复权限的路径（Unix 下需要先设置为可写，最后再改回只读）
        #[cfg(unix)]
        let mut unix_modes = Vec::new();

        for i in 0..archive.len() {
            let mut file = archive.by_index(i).map_err(|err| {
                ErrorType::ArchiveReadError(ErrorData {
                    error: err.to_string(),
                })
            })?;

            // 安全检查：获取安全的输出路径
            let outpath = match file.enclosed_name() {
                Some(path) => output_dir_canonical.join(path),
                None => continue, // 跳过不安全的路径
            };

            self.base.add_now(&outpath);

            if file.is_dir() {
                path_helper::create_dir_all(&outpath)?;
                // Unix 下目录需要保持可写，直到所有子文件提取完成（最后统一恢复权限）
                #[cfg(unix)]
                if let Some(mode) = file.unix_mode() {
                    // 临时设为 0o700 保证可写

                    use crate::archives::set_perms;
                    set_perms(&outpath, 0o700).map_err(|err| {
                        ErrorType::FileSystemError(FileSystemErrorData {
                            path: output_dir.to_path_buf(),
                            error: err.to_string(),
                        })
                    })?;

                    unix_modes.push((outpath, mode));
                }
                continue;
            }

            if file.is_symlink() {
                // 读取链接目标
                let mut target = Vec::new();
                file.read_to_end(&mut target).map_err(|err| {
                    ErrorType::ArchiveReadError(ErrorData {
                        error: err.to_string(),
                    })
                })?;
                let target_str = String::from_utf8(target).map_err(|err| {
                    ErrorType::ArchiveReadError(ErrorData {
                        error: err.to_string(),
                    })
                })?;
                make_symlink(&outpath, &target_str)?;
                continue;
            }

            // 普通文件
            let now = self.base.now.fetch_add(1, Ordering::SeqCst) + 1;

            if let Some(gui) = &self.base.gui {
                let filename = outpath
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                gui.update(Some(filename), now);
            }

            if let Some(parent) = outpath.parent() {
                let res = fs::create_dir_all(parent);
                if let Err(err) = res {
                    return Err(ErrorType::FileSystemError(FileSystemErrorData {
                        path: parent.to_path_buf(),
                        error: err.to_string(),
                    }));
                }
            }
            let mut outfile = path_helper::open_write(&outpath)?;
            io::copy(&mut file, &mut outfile).map_err(|err| {
                ErrorType::ArchiveError(ArchiveErrorData {
                    source: file.name().to_string(),
                    target: outpath.display().to_string(),
                    error: err.to_string(),
                })
            })?;

            // 保留 Unix 权限
            #[cfg(unix)]
            if let Some(mode) = file.unix_mode() {
                use crate::archives::set_perms;

                set_perms(&outpath, mode).map_err(|err| {
                    ErrorType::FileSystemError(FileSystemErrorData {
                        path: outpath.to_path_buf(),
                        error: err.to_string(),
                    })
                })?;
            }

            // 保留修改时间（需要 chrono feature）
            if let Some(last_modified) = file.last_modified() {
                if let Some(system_time) = datetime_to_systemtime(&last_modified) {
                    outfile.set_modified(system_time).map_err(|err| {
                        ErrorType::FileSystemError(FileSystemErrorData {
                            path: outpath.to_path_buf(),
                            error: err.to_string(),
                        })
                    })?;
                }
            }
        }

        // 恢复所有目录的最终权限（Unix 下最后才设为只读）
        #[cfg(unix)]
        for (path, mode) in unix_modes {
            use crate::archives::set_perms;

            set_perms(&path, mode).map_err(|err| {
                ErrorType::FileSystemError(FileSystemErrorData {
                    path: path.clone(),
                    error: err.to_string(),
                })
            })?;
        }

        Ok(())
    }
}

fn generate_chrono_datetime(time: &DateTime) -> Option<chrono::NaiveDateTime> {
    if let Some(chrono_date) =
        chrono::NaiveDate::from_ymd_opt(time.year().into(), time.month().into(), time.day().into())
        && let Some(chrono_datetime) = chrono_date.and_hms_opt(
            time.hour().into(),
            time.minute().into(),
            time.second().into(),
        )
    {
        return Some(chrono_datetime);
    }
    None
}

fn datetime_to_systemtime(time: &DateTime) -> Option<std::time::SystemTime> {
    if let Some(chrono_datetime) = generate_chrono_datetime(time) {
        let time = chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(
            chrono_datetime,
            chrono::Utc,
        );
        return Some(time.into());
    }
    None
}

/// 创建符号链接（跨平台）
#[cfg(unix)]
fn make_symlink(target_path: &Path, link_target: &str) -> io::Result<()> {
    std::os::unix::fs::symlink(link_target, target_path)
}

#[cfg(windows)]
fn make_symlink(target_path: &Path, link_target: &str) -> CoreResult<()> {
    let target = Path::new(link_target);
    if target.is_dir() {
        std::os::windows::fs::symlink_dir(target, target_path)
    } else {
        std::os::windows::fs::symlink_file(target, target_path)
    }
    .map_err(|err| {
        ErrorType::FileSystemError(FileSystemErrorData {
            path: target_path.to_path_buf(),
            error: err.to_string(),
        })
    })
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
