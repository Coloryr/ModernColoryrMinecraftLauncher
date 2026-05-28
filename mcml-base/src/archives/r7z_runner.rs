use std::{
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};

use mcml_names::i18_items::error_type::{
    ArchiveErrorData, ErrorData,
    ErrorType::{self},
    FileSystemErrorData,
};
use sevenz_rust2::{ArchiveEntry, ArchiveReader, ArchiveWriter, Password};

use crate::{
    archives::{IArchive, IArchiveGui, should_exclude},
    path_helper,
};

pub struct R7zProcess {
    gui: Option<Box<dyn IArchiveGui + Send + Sync>>,
    size: AtomicUsize,
    now: AtomicUsize,
}

impl IArchive for R7zProcess {
    fn compress(
        &self,
        archive_file: &PathBuf,
        pack_dir: &PathBuf,
        root_path: Option<&PathBuf>,
        filter: &Option<Vec<String>>,
    ) -> Result<(), ErrorType> {
        let root_path = match root_path {
            Some(path) => path,
            None => &pack_dir.clone(),
        };

        self.r7z_compress(archive_file, pack_dir, root_path, filter)
    }

    fn decompress(&self, archive_file: &PathBuf, output_dir: &PathBuf) -> Result<(), ErrorType> {
        self.r7z_decompress(archive_file, output_dir)
    }
}

impl R7zProcess {
    pub fn new(gui: Option<Box<dyn IArchiveGui + Send + Sync>>) -> Self {
        Self {
            gui,
            size: AtomicUsize::new(0),
            now: AtomicUsize::new(0),
        }
    }

    /// 将整个目录压缩为 7z 文件
    pub fn r7z_compress(
        &self,
        archive_file: &PathBuf,
        pack_dir: &PathBuf,
        root_path: &PathBuf,
        filter: &Option<Vec<String>>,
    ) -> Result<(), ErrorType> {
        let file = path_helper::open_write(archive_file);
        if let Err(err) = file {
            return Err(ErrorType::FileSystemError(FileSystemErrorData {
                path: archive_file.clone(),
                error: err.to_string(),
            }));
        }
        let file = file.unwrap();
        let archive = ArchiveWriter::new(file);
        if let Err(err) = archive {
            return Err(ErrorType::ArchiveOpenError(FileSystemErrorData {
                path: archive_file.clone(),
                error: err.to_string(),
            }));
        }

        let mut archive = archive.unwrap();
        let entries = path_helper::get_all_files(pack_dir);

        if let Some(gui) = &self.gui {
            let size = path_helper::get_all_files(pack_dir).len();
            self.size.store(size, Ordering::SeqCst);
            gui.start(size);
        }

        for path in entries {
            if let Some(patterns) = filter {
                if should_exclude(&path, patterns) {
                    continue;
                }
            }

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

            let buffer = buffer.unwrap();

            let relative_path = path.strip_prefix(root_path).unwrap();
            let tempfile = relative_path.to_string_lossy().to_string();

            let res = archive.push_archive_entry(
                ArchiveEntry::from_path(&path, tempfile.clone()),
                Some(buffer),
            );
            if let Err(err) = res {
                return Err(ErrorType::ArchiveError(ArchiveErrorData {
                    source: path.display().to_string(),
                    target: tempfile,
                    error: err.to_string(),
                }));
            }
        }

        match archive.finish() {
            Ok(_) => Ok(()),
            Err(err) => Err(ErrorType::ArchiveWriteError(ErrorData {
                error: err.to_string(),
            })),
        }
    }

    /// 解压 7z 文件到指定目录
    pub fn r7z_decompress(
        &self,
        archive_file: &PathBuf,
        output_dir: &PathBuf,
    ) -> Result<(), ErrorType> {
        let file = path_helper::open_read(archive_file);
        if let Err(err) = file {
            return Err(ErrorType::FileSystemError(FileSystemErrorData {
                path: archive_file.clone(),
                error: err.to_string(),
            }));
        }
        let file = file.unwrap();
        let seven = ArchiveReader::new(file, Password::empty());
        if let Err(err) = seven {
            return Err(ErrorType::ArchiveOpenError(FileSystemErrorData {
                path: archive_file.clone(),
                error: err.to_string(),
            }));
        }

        let mut seven = seven.unwrap();

        if let Some(gui) = &self.gui {
            let size = seven.archive().files.len();
            self.size.store(size, Ordering::SeqCst);
            gui.start(size);
        }

        match seven.for_each_entries(|entry, reader| {
            let dest_path = output_dir.join(entry.name());

            let now = self.now.fetch_add(1, Ordering::SeqCst) + 1;

            if let Some(gui) = &self.gui {
                let filename = entry.name.to_string();
                gui.zip_update(Some(filename), now);
            }

            sevenz_rust2::default_entry_extract_fn(entry, reader, &dest_path)
        }) {
            Ok(_) => Ok(()),
            Err(err) => Err(ErrorType::ArchiveReadError(ErrorData {
                error: err.to_string(),
            })),
        }
    }
}
