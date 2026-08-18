//! Tar/TarGz/TarXz 压缩/解压实现（基于 `tar` + `flate2`/`xz2` crate）

use std::{
    fs,
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
    sync::Arc,
};

use flate2::read::GzDecoder;
use flate2::{Compression, write::GzEncoder};
use mcml_names::i18_items::error_type::{
    ArchiveErrorData, CoreResult, ErrorData, ErrorType, FileSystemErrorData,
};
use mcml_sys::path_helper;
use tar::{Archive, Builder};
use xz2::{read::XzDecoder, write::XzEncoder};

use crate::archives::{
    self, ArchiveEntryInfo, ArchiveHandle, ArchiveProcess, ArchiveType, IBaseArchiveGui, TarMode,
};

/// 保持打开文件句柄的 Tar/TarGz/TarXz 读取句柄。
///
/// tar 读取器是流式的、不可复用，每次操作从持有的文件句柄克隆出独立文件描述符再重建读取器。
/// 同时提供静态的压缩/解压批处理入口（`mode=None` 表示纯 tar）。
pub(crate) struct TarReader {
    file: fs::File,
    archive_type: ArchiveType,
    path: PathBuf,
}

impl TarReader {
    pub(crate) fn new(
        file: fs::File,
        archive_type: ArchiveType,
        path: PathBuf,
    ) -> CoreResult<Self> {
        Ok(Self {
            file,
            archive_type,
            path,
        })
    }

    /// 从持有的文件句柄克隆出独立文件描述符，构建对应的 tar 读取器。
    ///
    /// 克隆句柄与原句柄共享文件偏移，读取前先定位到文件开头。
    fn reader(&self) -> CoreResult<Archive<Box<dyn Read>>> {
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
        match self.archive_type {
            ArchiveType::Tar => Ok(Archive::new(Box::new(file))),
            ArchiveType::TarGz => {
                let gz = GzDecoder::new(file);
                Ok(Archive::new(Box::new(gz)))
            }
            ArchiveType::TarXz => {
                let xz = XzDecoder::new(file);
                Ok(Archive::new(Box::new(xz)))
            }
            _ => unreachable!(),
        }
    }

    /// 压缩目录为 tar/tar.gz/tar.xz 文件。
    pub(crate) fn compress(
        mode: Option<TarMode>,
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
        Self::tar(&process, archive_file, pack_dir, root_path, filter, mode)
    }

    /// 解压 tar/tar.gz/tar.xz 文件到指定目录。
    pub(crate) fn decompress(
        mode: Option<TarMode>,
        archive_file: &Path,
        output_dir: &Path,
        gui: Option<Arc<dyn IBaseArchiveGui>>,
    ) -> CoreResult<()> {
        let process = ArchiveProcess::new(gui);
        Self::un_tar(&process, archive_file, output_dir, mode)
    }

    /// 压缩实现（带进度）。
    fn tar(
        process: &ArchiveProcess,
        archive_file: &Path,
        pack_dir: &Path,
        root_path: &Path,
        filter: &Option<Vec<String>>,
        mode: Option<TarMode>,
    ) -> CoreResult<()> {
        let file = path_helper::open_write(archive_file)?;

        let files = path_helper::get_all_files(pack_dir);

        let mut tar_builder: Builder<Box<dyn std::io::Write>> = match mode {
            Some(TarMode::Gz) => {
                let gz_encoder = GzEncoder::new(file, Compression::default());
                Builder::new(Box::new(gz_encoder))
            }
            Some(TarMode::Xz) => {
                let xz_encoder = XzEncoder::new(file, 6);
                Builder::new(Box::new(xz_encoder))
            }
            None => Builder::new(Box::new(file) as Box<dyn std::io::Write>),
        };

        for path in files {
            process.add_now(&path);

            if let Some(patterns) = filter {
                if archives::should_exclude(&path, patterns) {
                    continue;
                }
            }

            let relative_path = path.strip_prefix(root_path).unwrap();
            let archive_path = archives::normalize_path(relative_path);

            if path.is_file() {
                let mut file_reader = path_helper::open_read(&path)?;

                tar_builder
                    .append_file(&archive_path, &mut file_reader)
                    .map_err(|err| {
                        ErrorType::ArchiveError(ArchiveErrorData {
                            source: path.to_string_lossy().to_string(),
                            target: archive_path.clone(),
                            error: err.to_string(),
                        })
                    })?;
            } else if path.is_dir() {
                tar_builder
                    .append_dir(&archive_path, &path)
                    .map_err(|err| {
                        ErrorType::ArchiveError(ArchiveErrorData {
                            source: path.to_string_lossy().to_string(),
                            target: archive_path.clone(),
                            error: err.to_string(),
                        })
                    })?;
            }
        }

        tar_builder.finish().map_err(|err| {
            ErrorType::ArchiveWriteError(ErrorData {
                error: err.to_string(),
            })
        })?;

        Ok(())
    }

    /// 解压实现（带进度）。
    fn un_tar(
        process: &ArchiveProcess,
        archive_file: &Path,
        output_dir: &Path,
        mode: Option<TarMode>,
    ) -> CoreResult<()> {
        path_helper::create_dir_all(output_dir)?;

        {
            let file = path_helper::open_read(archive_file)?;
            let mut archive: Archive<Box<dyn Read>> = match mode {
                Some(TarMode::Gz) => {
                    let gz = GzDecoder::new(file);
                    Archive::new(Box::new(gz))
                }
                Some(TarMode::Xz) => {
                    let xz = XzDecoder::new(file);
                    Archive::new(Box::new(xz))
                }
                None => Archive::new(Box::new(file) as Box<dyn Read>),
            };
            let count = archive
                .entries()
                .map_err(|err| {
                    ErrorType::ArchiveReadError(ErrorData {
                        error: err.to_string(),
                    })
                })?
                .count();
            process.set_count(count);
        }

        let file = path_helper::open_read(archive_file)?;
        let mut archive: Archive<Box<dyn Read>> = match mode {
            Some(TarMode::Gz) => {
                let gz = GzDecoder::new(file);
                Archive::new(Box::new(gz))
            }
            Some(TarMode::Xz) => {
                let xz = XzDecoder::new(file);
                Archive::new(Box::new(xz))
            }
            None => Archive::new(Box::new(file) as Box<dyn Read>),
        };

        let items = archive.entries().map_err(|err| {
            ErrorType::ArchiveReadError(ErrorData {
                error: err.to_string(),
            })
        })?;

        for entry in items {
            let mut entry = entry.map_err(|err| {
                ErrorType::ArchiveReadError(ErrorData {
                    error: err.to_string(),
                })
            })?;
            let path = entry
                .path()
                .map_err(|err| {
                    ErrorType::ArchiveReadError(ErrorData {
                        error: err.to_string(),
                    })
                })?
                .to_path_buf();
            // 文件名非法时询问 GUI 是否替换
            let name = process.check_name(&path.to_string_lossy())?;
            let name_path = Path::new(&name).to_path_buf();
            process.add_now(&name_path);
            if name_path != path {
                let dest = output_dir.join(name_path);
                if let Some(parent) = dest.parent() {
                    std::fs::create_dir_all(parent).map_err(|err| {
                        ErrorType::FileSystemError(FileSystemErrorData {
                            path: parent.to_path_buf(),
                            error: err.to_string(),
                        })
                    })?;
                }
                entry.unpack(dest).map_err(|err| {
                    ErrorType::ArchiveError(ArchiveErrorData {
                        source: path.display().to_string(),
                        target: output_dir.display().to_string(),
                        error: err.to_string(),
                    })
                })?;
            } else {
                entry.unpack_in(output_dir).map_err(|err| {
                    ErrorType::ArchiveError(ArchiveErrorData {
                        source: path.display().to_string(),
                        target: output_dir.display().to_string(),
                        error: err.to_string(),
                    })
                })?;
            }
        }

        process.done();

        Ok(())
    }
}

impl ArchiveHandle for TarReader {
    fn read_entries(&mut self) -> CoreResult<Vec<ArchiveEntryInfo>> {
        let mut archive = self.reader()?;

        let mut entries = Vec::new();
        for entry in archive.entries().map_err(|err| {
            ErrorType::ArchiveReadError(ErrorData {
                error: err.to_string(),
            })
        })? {
            let entry = entry.map_err(|err| {
                ErrorType::ArchiveReadError(ErrorData {
                    error: err.to_string(),
                })
            })?;
            let header = entry.header();
            entries.push(ArchiveEntryInfo {
                name: entry
                    .path()
                    .map_err(|err| {
                        ErrorType::ArchiveReadError(ErrorData {
                            error: err.to_string(),
                        })
                    })?
                    .to_string_lossy()
                    .to_string(),
                is_dir: header.entry_type() == tar::EntryType::Directory,
                size: header.size().map_err(|err| {
                    ErrorType::ArchiveReadError(ErrorData {
                        error: err.to_string(),
                    })
                })?,
            });
        }
        Ok(entries)
    }

    fn read(&mut self, name: &str) -> CoreResult<Vec<u8>> {
        let mut archive = self.reader()?;

        for entry in archive.entries().map_err(|err| {
            ErrorType::ArchiveReadError(ErrorData {
                error: err.to_string(),
            })
        })? {
            let mut entry = entry.map_err(|err| {
                ErrorType::ArchiveReadError(ErrorData {
                    error: err.to_string(),
                })
            })?;
            let entry_path = entry.path().map_err(|err| {
                ErrorType::ArchiveReadError(ErrorData {
                    error: err.to_string(),
                })
            })?;
            if entry_path.to_string_lossy() == name {
                let header = entry.header();
                if header.entry_type() == tar::EntryType::Directory {
                    return Err(ErrorType::ArchiveReadError(ErrorData {
                        error: name.to_string(),
                    }));
                }
                let mut buf = Vec::with_capacity(header.size().unwrap_or(0) as usize);
                entry.read_to_end(&mut buf).map_err(|err| {
                    ErrorType::ArchiveReadError(ErrorData {
                        error: err.to_string(),
                    })
                })?;
                return Ok(buf);
            }
        }

        Err(ErrorType::ArchiveReadError(ErrorData {
            error: name.to_string(),
        }))
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

        let mut archive = self.reader()?;

        let mut found = false;
        for entry in archive.entries().map_err(|err| {
            ErrorType::ArchiveReadError(ErrorData {
                error: err.to_string(),
            })
        })? {
            let mut entry = entry.map_err(|err| {
                ErrorType::ArchiveReadError(ErrorData {
                    error: err.to_string(),
                })
            })?;
            let entry_path = entry.path().map_err(|err| {
                ErrorType::ArchiveReadError(ErrorData {
                    error: err.to_string(),
                })
            })?;
            if entry_path.to_string_lossy() == name {
                found = true;
                let mut outfile = path_helper::open_write(output_path)?;
                std::io::copy(&mut entry, &mut outfile).map_err(|err| {
                    ErrorType::ArchiveError(ArchiveErrorData {
                        source: name.to_string(),
                        target: output_path.display().to_string(),
                        error: err.to_string(),
                    })
                })?;
                break;
            }
        }

        if !found {
            return Err(ErrorType::ArchiveReadError(ErrorData {
                error: name.to_string(),
            }));
        }

        if let Some(gui) = gui {
            gui.update(Some(name.to_string()), 1);
        }

        Ok(())
    }

    fn add_files(&mut self, files: &[(PathBuf, PathBuf)]) -> CoreResult<()> {
        use tar::Builder;

        let mut file = self.file.try_clone().map_err(|err| {
            ErrorType::FileSystemError(FileSystemErrorData {
                path: self.path.clone(),
                error: err.to_string(),
            })
        })?;

        // 跳过 tar 的 EOF 标记（两个 512 字节的零块），由 Builder
        // 用新条目 + 新 EOF 覆盖
        let len = file
            .metadata()
            .map_err(|err| {
                ErrorType::FileSystemError(FileSystemErrorData {
                    path: self.path.clone(),
                    error: err.to_string(),
                })
            })?
            .len();
        if len >= 1024 {
            file.seek(SeekFrom::Start(len - 1024)).map_err(|err| {
                ErrorType::FileSystemError(FileSystemErrorData {
                    path: self.path.clone(),
                    error: err.to_string(),
                })
            })?;
        }

        let mut builder = Builder::new(file);
        for (src, dest) in files {
            let internal_path = dest.to_string_lossy();
            let mut reader = path_helper::open_read(src)?;

            builder
                .append_file(internal_path.as_ref(), &mut reader)
                .map_err(|err| {
                    ErrorType::ArchiveWriteError(ErrorData {
                        error: err.to_string(),
                    })
                })?;
        }

        builder.finish().map_err(|err| {
            ErrorType::ArchiveWriteError(ErrorData {
                error: err.to_string(),
            })
        })?;
        Ok(())
    }

    fn add_data(&mut self, name: &str, data: &[u8]) -> CoreResult<()> {
        use tar::{Builder, Header};

        let mut file = self.file.try_clone().map_err(|err| {
            ErrorType::FileSystemError(FileSystemErrorData {
                path: self.path.clone(),
                error: err.to_string(),
            })
        })?;

        // 跳过 tar 的 EOF 标记（两个 512 字节的零块），由 Builder
        // 用新条目 + 新 EOF 覆盖
        let len = file
            .metadata()
            .map_err(|err| {
                ErrorType::FileSystemError(FileSystemErrorData {
                    path: self.path.clone(),
                    error: err.to_string(),
                })
            })?
            .len();
        if len >= 1024 {
            file.seek(SeekFrom::Start(len - 1024)).map_err(|err| {
                ErrorType::FileSystemError(FileSystemErrorData {
                    path: self.path.clone(),
                    error: err.to_string(),
                })
            })?;
        }

        let mut builder = Builder::new(file);
        let mut header = Header::new_gnu();
        header.set_size(data.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();

        builder
            .append_data(&mut header, name, data)
            .map_err(|err| {
                ErrorType::ArchiveWriteError(ErrorData {
                    error: err.to_string(),
                })
            })?;

        builder.finish().map_err(|err| {
            ErrorType::ArchiveWriteError(ErrorData {
                error: err.to_string(),
            })
        })?;
        Ok(())
    }
}
