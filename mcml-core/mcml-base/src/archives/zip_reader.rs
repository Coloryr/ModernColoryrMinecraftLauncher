//! Zip 压缩/解压实现（基于 `zip` crate）

use std::{
    fs,
    io::{self, Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    sync::{Arc, atomic::Ordering},
};

use mcml_names::i18_items::error_type::{
    ArchiveErrorData, CoreResult, ErrorData, ErrorType, FileSystemErrorData,
};
use mcml_sys::path_helper;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, DateTime, ZipArchive, ZipWriter};

use crate::archives::{self, ArchiveEntryInfo, ArchiveHandle, ArchiveProcess, IBaseArchiveGui};

/// 保持打开的 Zip 读取句柄（缓存 [`ZipArchive`]，中央目录只解析一次）。
///
/// 同时提供静态的压缩/解压批处理入口。
pub(crate) struct ZipReader {
    /// 读写的文件句柄（追加/重建用）
    file: fs::File,
    /// 缓存的 [`ZipArchive`]（由 `file` 克隆构建）
    zip: ZipArchive<fs::File>,
    /// 压缩包磁盘路径
    path: PathBuf,
}

impl ZipReader {
    /// 压缩目录为 zip 文件。
    pub(crate) fn compress(
        archive_file: &Path,
        pack_dir: &Path,
        root_path: Option<&Path>,
        filter: &Option<Vec<String>>,
        gui: Option<Arc<dyn IBaseArchiveGui>>,
    ) -> CoreResult<()> {
        let process = ArchiveProcess::new(gui);
        let root_path = match root_path {
            Some(path) => path,
            None => pack_dir,
        };
        Self::zip(&process, archive_file, pack_dir, root_path, filter)
    }

    /// 解压 zip 文件到指定目录。
    pub(crate) fn decompress(
        archive_file: &Path,
        output_dir: &Path,
        gui: Option<Arc<dyn IBaseArchiveGui>>,
    ) -> CoreResult<()> {
        let process = ArchiveProcess::new(gui);
        Self::unzip(&process, archive_file, output_dir)
    }

    /// 压缩实现（带进度）。
    fn zip(
        process: &ArchiveProcess,
        archive_file: &Path,
        pack_dir: &Path,
        root_path: &Path,
        filter: &Option<Vec<String>>,
    ) -> CoreResult<()> {
        let file = path_helper::open_write(archive_file)?;
        let mut zip = ZipWriter::new(file);
        let files = path_helper::get_all_files(pack_dir);

        process.set_count(files.len());

        let options = SimpleFileOptions::default()
            .compression_method(CompressionMethod::Deflated)
            .unix_permissions(0o755);

        for path in files {
            process.add_now(&path);

            if let Some(patterns) = filter {
                if archives::should_exclude(&path, patterns) {
                    continue;
                }
            }

            if !path.is_dir() {
                let mut buffer = path_helper::open_read(&path)?;

                let relative_path = path.strip_prefix(root_path).unwrap();
                let tempfile = archives::normalize_path(relative_path);

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
                let tempfile = archives::normalize_path(relative_path);

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

    /// 解压实现（带进度）。
    fn unzip(process: &ArchiveProcess, archive_file: &Path, output_dir: &Path) -> CoreResult<()> {
        let file = path_helper::open_read(archive_file)?;
        let mut archive = ZipArchive::new(file).map_err(|err| {
            ErrorType::ArchiveOpenError(FileSystemErrorData {
                path: archive_file.to_path_buf(),
                error: err.to_string(),
            })
        })?;
        process.set_count(archive.len());

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
                Some(path) => path,
                None => continue, // 跳过不安全的路径
            };

            // 文件名非法时询问 GUI 是否替换
            let outpath =
                output_dir_canonical.join(process.check_name(&outpath.to_string_lossy())?);

            process.add_now(&outpath);

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
            let now = process.now.fetch_add(1, Ordering::SeqCst) + 1;

            if let Some(gui) = &process.gui {
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

        process.done();

        Ok(())
    }

    pub(crate) fn new(file: fs::File, path: PathBuf) -> CoreResult<Self> {
        let zip = ZipArchive::new(file.try_clone().map_err(|err| {
            ErrorType::FileSystemError(FileSystemErrorData {
                path: path.clone(),
                error: err.to_string(),
            })
        })?)
        .map_err(|err| {
            ErrorType::ArchiveOpenError(FileSystemErrorData {
                path: path.clone(),
                error: err.to_string(),
            })
        })?;
        Ok(Self { file, zip, path })
    }

    /// 追加后用持有的读写句柄重新解析中央目录。
    fn reload(&mut self) -> CoreResult<()> {
        let mut file = self.file.try_clone().map_err(|err| {
            ErrorType::FileSystemError(FileSystemErrorData {
                path: self.path.clone(),
                error: err.to_string(),
            })
        })?;
        file.seek(SeekFrom::Start(0)).map_err(|err| {
            ErrorType::FileSystemError(FileSystemErrorData {
                path: self.path.clone(),
                error: err.to_string(),
            })
        })?;
        self.zip = ZipArchive::new(file).map_err(|err| {
            ErrorType::ArchiveOpenError(FileSystemErrorData {
                path: self.path.clone(),
                error: err.to_string(),
            })
        })?;
        Ok(())
    }
}

impl ArchiveHandle for ZipReader {
    fn read_entries(&mut self) -> CoreResult<Vec<ArchiveEntryInfo>> {
        let mut entries = Vec::with_capacity(self.zip.len());
        for i in 0..self.zip.len() {
            let entry = self.zip.by_index(i).map_err(|err| {
                ErrorType::ArchiveReadError(ErrorData {
                    error: err.to_string(),
                })
            })?;
            entries.push(ArchiveEntryInfo {
                name: entry.name().to_string(),
                is_dir: entry.is_dir(),
                size: entry.size(),
            });
        }
        Ok(entries)
    }

    fn read(&mut self, name: &str) -> CoreResult<Vec<u8>> {
        let mut entry = self.zip.by_name(name).map_err(|err| {
            ErrorType::ArchiveReadError(ErrorData {
                error: err.to_string(),
            })
        })?;

        if entry.is_dir() {
            return Err(ErrorType::ArchiveReadError(ErrorData {
                error: name.to_string(),
            }));
        }

        let mut buf = Vec::with_capacity(entry.size() as usize);
        entry.read_to_end(&mut buf).map_err(|err| {
            ErrorType::ArchiveReadError(ErrorData {
                error: err.to_string(),
            })
        })?;
        Ok(buf)
    }

    fn read_stream(&mut self, name: &str) -> CoreResult<Box<dyn Read>> {
        let data = self.read(name)?;
        Ok(Box::new(std::io::Cursor::new(data)))
    }

    fn extract_file(
        &mut self,
        name: &str,
        output_path: &Path,
        gui: Option<&dyn IBaseArchiveGui>,
    ) -> CoreResult<()> {
        if let Some(gui) = gui {
            gui.start(1);
        }

        let mut entry = self.zip.by_name(name).map_err(|err| {
            ErrorType::ArchiveReadError(ErrorData {
                error: err.to_string(),
            })
        })?;

        let mut outfile = path_helper::open_write(output_path)?;
        std::io::copy(&mut entry, &mut outfile).map_err(|err| {
            ErrorType::ArchiveError(ArchiveErrorData {
                source: name.to_string(),
                target: output_path.display().to_string(),
                error: err.to_string(),
            })
        })?;

        if let Some(gui) = gui {
            gui.update(Some(name.to_string()), 1);
        }

        Ok(())
    }

    fn add_files(&mut self, files: &[(PathBuf, PathBuf)]) -> CoreResult<()> {
        use zip::{CompressionMethod, ZipWriter, write::SimpleFileOptions};

        let file = self.file.try_clone().map_err(|err| {
            ErrorType::FileSystemError(FileSystemErrorData {
                path: self.path.clone(),
                error: err.to_string(),
            })
        })?;
        let mut zip = ZipWriter::new_append(file).map_err(|err| {
            ErrorType::ArchiveOpenError(FileSystemErrorData {
                path: self.path.clone(),
                error: err.to_string(),
            })
        })?;

        let options = SimpleFileOptions::default()
            .compression_method(CompressionMethod::Deflated)
            .unix_permissions(0o644);

        for (src, dest) in files {
            let internal_path = dest.to_string_lossy();
            let mut reader = path_helper::open_read(src)?;

            zip.start_file(internal_path.as_ref(), options)
                .map_err(|err| {
                    ErrorType::ArchiveWriteError(ErrorData {
                        error: err.to_string(),
                    })
                })?;

            std::io::copy(&mut reader, &mut zip).map_err(|err| {
                ErrorType::ArchiveWriteError(ErrorData {
                    error: err.to_string(),
                })
            })?;
        }

        zip.finish().map_err(|err| {
            ErrorType::ArchiveWriteError(ErrorData {
                error: err.to_string(),
            })
        })?;

        self.reload()
    }

    fn add_data(&mut self, name: &str, data: &[u8]) -> CoreResult<()> {
        use zip::{CompressionMethod, ZipWriter, write::SimpleFileOptions};

        let file = self.file.try_clone().map_err(|err| {
            ErrorType::FileSystemError(FileSystemErrorData {
                path: self.path.clone(),
                error: err.to_string(),
            })
        })?;
        let mut zip = ZipWriter::new_append(file).map_err(|err| {
            ErrorType::ArchiveOpenError(FileSystemErrorData {
                path: self.path.clone(),
                error: err.to_string(),
            })
        })?;

        let options = SimpleFileOptions::default()
            .compression_method(CompressionMethod::Deflated)
            .unix_permissions(0o644);

        zip.start_file(name, options).map_err(|err| {
            ErrorType::ArchiveWriteError(ErrorData {
                error: err.to_string(),
            })
        })?;

        zip.write_all(data).map_err(|err| {
            ErrorType::ArchiveWriteError(ErrorData {
                error: err.to_string(),
            })
        })?;

        zip.finish().map_err(|err| {
            ErrorType::ArchiveWriteError(ErrorData {
                error: err.to_string(),
            })
        })?;

        self.reload()
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
