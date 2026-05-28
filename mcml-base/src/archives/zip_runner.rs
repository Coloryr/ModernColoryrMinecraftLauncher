use mcml_names::i18_items::error_type::{
    ArchiveErrorData, ErrorData, ErrorType, FileSystemErrorData,
};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, DateTime, ZipArchive, ZipWriter};

use std::fs::{self, File};
use std::io::{self, Read};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::archives::{IArchive, IArchiveGui, make_symlink, should_exclude};
use crate::path_helper;

pub struct ZipProcess {
    gui: Option<Box<dyn IArchiveGui + Send + Sync>>,
    size: AtomicUsize,
    now: AtomicUsize,
}

impl IArchive for ZipProcess {
    fn compress(
        &self,
        zip_file: &PathBuf,
        pack_dir: &PathBuf,
        root_path: Option<&PathBuf>,
        filter: &Option<Vec<String>>,
    ) -> Result<(), ErrorType> {
        let root_path = match root_path {
            Some(path) => path,
            None => &pack_dir.clone(),
        };
        self.zip(zip_file, pack_dir, root_path, filter)
    }

    fn decompress(&self, archive_file: &PathBuf, output_dir: &PathBuf) -> Result<(), ErrorType> {
        self.unzip(archive_file, output_dir)
    }
}

impl ZipProcess {
    pub fn new(gui: Option<Box<dyn IArchiveGui + Send + Sync>>) -> Self {
        Self {
            gui,
            size: AtomicUsize::new(0),
            now: AtomicUsize::new(0),
        }
    }

    pub fn zip(
        &self,
        zip_file: &PathBuf,
        pack_dir: &PathBuf,
        root_path: &PathBuf,
        filter: &Option<Vec<String>>,
    ) -> Result<(), ErrorType> {
        let zip = path_helper::open_write(zip_file);
        match zip {
            Err(err) => Err(ErrorType::FileSystemError(FileSystemErrorData {
                path: zip_file.clone(),
                error: err.to_string(),
            })),
            Ok(zip) => {
                if let Some(gui) = &self.gui {
                    let size = path_helper::get_all_files(pack_dir).len();
                    self.size.store(size, Ordering::SeqCst);
                    gui.start(size);
                }

                let mut zip = ZipWriter::new(zip);
                self.zip_inner(pack_dir, &mut zip, root_path, filter)
            }
        }
    }

    fn zip_inner(
        &self,
        pack_dir: &PathBuf,
        zip: &mut ZipWriter<File>,
        root_path: &PathBuf,
        filter: &Option<Vec<String>>,
    ) -> Result<(), ErrorType> {
        let mut entries = Vec::new();
        let items = std::fs::read_dir(&pack_dir);
        if let Err(err) = items {
            return Err(ErrorType::FileSystemError(FileSystemErrorData {
                path: pack_dir.clone(),
                error: err.to_string(),
            }));
        }

        for entry in items.unwrap() {
            if let Ok(entry) = entry {
                entries.push(entry.path());
            }
        }

        let options = SimpleFileOptions::default()
            .compression_method(CompressionMethod::Deflated)
            .unix_permissions(0o755);

        for path in entries {
            if let Some(patterns) = filter {
                if should_exclude(&path, patterns) {
                    continue;
                }
            }

            if path.is_dir() {
                if let Err(err) = self.zip_inner(&path, zip, root_path, filter) {
                    return Err(err);
                }
            } else {
                let now = self.now.fetch_add(1, Ordering::SeqCst) + 1;

                if let Some(gui) = &self.gui {
                    let filename = path
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default();
                    gui.zip_update(Some(filename), now);
                }

                let buffer = path_helper::open_read(&path);
                if let Err(err) = buffer {
                    return Err(ErrorType::FileSystemError(FileSystemErrorData {
                        path: path.clone(),
                        error: err.to_string(),
                    }));
                }

                let mut buffer = buffer.unwrap();

                let relative_path = path.strip_prefix(root_path).unwrap();
                let tempfile = relative_path.to_string_lossy().to_string();

                let res = zip.start_file(&tempfile, options);
                if let Err(err) = res {
                    return Err(ErrorType::ArchiveError(ArchiveErrorData {
                        source: path.to_string_lossy().to_string(),
                        target: tempfile.clone(),
                        error: err.to_string(),
                    }));
                }

                let res = std::io::copy(&mut buffer, zip);
                if let Err(err) = res {
                    return Err(ErrorType::ArchiveError(ArchiveErrorData {
                        source: path.to_string_lossy().to_string(),
                        target: tempfile.clone(),
                        error: err.to_string(),
                    }));
                }
            }
        }

        if let Some(gui) = &self.gui {
            gui.done();
        }

        Ok(())
    }

    pub fn unzip(&self, archive_file: &PathBuf, output_dir: &PathBuf) -> Result<(), ErrorType> {
        let file = path_helper::open_read(archive_file);
        if let Err(err) = file {
            return Err(ErrorType::FileSystemError(FileSystemErrorData {
                path: archive_file.clone(),
                error: err.to_string(),
            }));
        }
        let file = file.unwrap();
        let archive = ZipArchive::new(file);
        if let Err(err) = archive {
            return Err(ErrorType::ArchiveOpenError(FileSystemErrorData {
                path: archive_file.clone(),
                error: err.to_string(),
            }));
        }
        let mut archive = archive.unwrap();

        self.size.store(archive.len(), Ordering::SeqCst);

        if let Some(gui) = &self.gui {
            gui.start(self.size.load(Ordering::SeqCst));
        }

        let res = fs::create_dir_all(output_dir);
        if let Err(err) = res {
            return Err(ErrorType::FileSystemError(FileSystemErrorData {
                path: output_dir.clone(),
                error: err.to_string(),
            }));
        }
        let output_dir_canonical = output_dir.canonicalize();
        if let Err(err) = output_dir_canonical {
            return Err(ErrorType::FileSystemError(FileSystemErrorData {
                path: output_dir.clone(),
                error: err.to_string(),
            }));
        }

        let output_dir_canonical = output_dir_canonical.unwrap();

        // 收集需要恢复权限的路径（Unix 下需要先设置为可写，最后再改回只读）
        #[cfg(unix)]
        let mut unix_modes = Vec::new();

        for i in 0..archive.len() {
            let file = archive.by_index(i);
            if let Err(err) = file {
                return Err(ErrorType::FileSystemError(FileSystemErrorData {
                    path: output_dir.clone(),
                    error: err.to_string(),
                }));
            }
            let mut file = file.unwrap();

            // 安全检查：获取安全的输出路径
            let outpath = match file.enclosed_name() {
                Some(path) => output_dir_canonical.join(path),
                None => continue, // 跳过不安全的路径
            };

            if file.is_dir() {
                let res = fs::create_dir_all(&outpath);
                if let Err(err) = res {
                    return Err(ErrorType::FileSystemError(FileSystemErrorData {
                        path: output_dir.clone(),
                        error: err.to_string(),
                    }));
                }
                // Unix 下目录需要保持可写，直到所有子文件提取完成（最后统一恢复权限）
                #[cfg(unix)]
                if let Some(mode) = file.unix_mode() {
                    // 临时设为 0o700 保证可写

                    use crate::archives::set_perms;
                    let res = set_perms(&outpath, 0o700);
                    if let Err(err) = res {
                        return Err(ErrorType::FileSystemError(FileSystemErrorData {
                            path: output_dir.clone(),
                            error: err.to_string(),
                        }));
                    }

                    unix_modes.push((outpath, mode));
                }
                continue;
            }

            if file.is_symlink() {
                // 读取链接目标
                let mut target = Vec::new();
                let res = file.read_to_end(&mut target);
                if let Err(err) = res {
                    return Err(ErrorType::ArchiveReadError(ErrorData {
                        error: err.to_string(),
                    }));
                }
                let target_str = String::from_utf8(target);
                if let Err(err) = target_str {
                    return Err(ErrorType::ArchiveReadError(ErrorData {
                        error: err.to_string(),
                    }));
                }
                let target_str = target_str.unwrap();
                let res = make_symlink(&outpath, &target_str);
                if let Err(err) = res {
                    return Err(ErrorType::FileSystemError(FileSystemErrorData {
                        path: output_dir.clone(),
                        error: err.to_string(),
                    }));
                }
                continue;
            }

            // 普通文件
            let now = self.now.fetch_add(1, Ordering::SeqCst) + 1;

            if let Some(gui) = &self.gui {
                let filename = outpath
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                gui.zip_update(Some(filename), now);
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
            let outfile = path_helper::open_write(&outpath);
            if let Err(err) = outfile {
                return Err(ErrorType::FileSystemError(FileSystemErrorData {
                    path: outpath.to_path_buf(),
                    error: err.to_string(),
                }));
            }
            let mut outfile = outfile.unwrap();
            let res = io::copy(&mut file, &mut outfile);
            if let Err(err) = res {
                return Err(ErrorType::ArchiveError(ArchiveErrorData {
                    source: file.name().to_string(),
                    target: outpath.display().to_string(),
                    error: err.to_string(),
                }));
            }

            // 保留 Unix 权限
            #[cfg(unix)]
            if let Some(mode) = file.unix_mode() {
                use crate::archives::set_perms;

                let res = set_perms(&outpath, mode);
                if let Err(err) = res {
                    return Err(ErrorType::FileSystemError(FileSystemErrorData {
                        path: outpath.to_path_buf(),
                        error: err.to_string(),
                    }));
                }
            }

            // 保留修改时间（需要 chrono feature）
            if let Some(last_modified) = file.last_modified() {
                if let Some(system_time) = datetime_to_systemtime(&last_modified) {
                    let res = outfile.set_modified(system_time);
                    if let Err(err) = res {
                        return Err(ErrorType::FileSystemError(FileSystemErrorData {
                            path: outpath.to_path_buf(),
                            error: err.to_string(),
                        }));
                    }
                }
            }
        }

        // 恢复所有目录的最终权限（Unix 下最后才设为只读）
        #[cfg(unix)]
        for (path, mode) in unix_modes {
            use crate::archives::set_perms;

            let res = set_perms(&path, mode);
            if let Err(err) = res {
                return Err(ErrorType::FileSystemError(FileSystemErrorData {
                    path: path.clone(),
                    error: err.to_string(),
                }));
            }
        }

        if let Some(gui) = &self.gui {
            gui.done();
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
