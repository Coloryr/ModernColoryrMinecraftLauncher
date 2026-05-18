use mcml_names::i18_items::error_type::{ErrorType, FileSystemErrorData, ZipErrorData};
use std::fs::File;
use std::io::{Seek, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use zip::CompressionMethod;
use zip::write::{SimpleFileOptions, ZipWriter};

use crate::path_helper;

// 假设的GUI trait
pub trait IZipGui: Send + Sync {
    fn zip_update(&self, filename: String, current: usize, total: usize);
    fn file_rename(&self, old_name: &str) -> bool;
    fn done(&self);
}

pub struct ZipProcess {
    gui: Option<Box<dyn IZipGui + Send + Sync>>,
    size: AtomicUsize,
    now: AtomicUsize,
}

impl ZipProcess {
    pub fn new(gui: Option<Box<dyn IZipGui + Send + Sync>>) -> Self {
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
        root_path: &String,
        filter: &Option<Vec<String>>,
    ) -> Result<(), ErrorType> {
        let zip = path_helper::open_write(zip_file);
        match zip {
            Err(err) => Err(ErrorType::FileSystemError(FileSystemErrorData {
                dir: zip_file.to_string_lossy().to_string(),
                error: err.to_string(),
            })),
            Ok(zip) => {
                let mut zip = ZipWriter::new(zip);
                self.zip_inner(pack_dir, &mut zip, root_path, filter)
            }
        }
    }

    fn zip_inner(
        &self,
        pack_dir: &PathBuf,
        zip: &mut ZipWriter<File>,
        root_path: &String,
        filter: &Option<Vec<String>>,
    ) -> Result<(), ErrorType> {
        let mut entries = Vec::new();
        let items = std::fs::read_dir(&pack_dir);
        if let Err(err) = items {
            return Err(ErrorType::FileSystemError(FileSystemErrorData {
                dir: pack_dir.to_string_lossy().to_string(),
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

        for file_path in entries {
            if let Some(filter) = filter {
                if filter.contains(&file_path.to_string_lossy().to_string()) {
                    continue;
                }
            }

            if file_path.is_dir() {
                if let Err(err) = self.zip_inner(&file_path, zip, root_path, filter) {
                    return Err(err);
                }
            } else {
                let now = self.now.fetch_add(1, Ordering::SeqCst) + 1;

                if let Some(gui) = &self.gui {
                    let filename = file_path
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default();
                    gui.zip_update(filename, now, self.size.load(Ordering::SeqCst));
                }

                let buffer = path_helper::open_read(&file_path);
                if buffer.is_none() {
                    continue;
                }

                let mut buffer = buffer.unwrap();

                let relative_path = file_path.strip_prefix(root_path).unwrap();
                let tempfile = relative_path.to_string_lossy().to_string();

                let res = zip.start_file(&tempfile, options);
                if let Err(err) = res {
                    return Err(ErrorType::ZipError(ZipErrorData {
                        source: file_path.to_string_lossy().to_string(),
                        target: tempfile.clone(),
                        error: err.to_string(),
                    }));
                }

                let res = std::io::copy(&mut buffer, zip);
                if let Err(err) = res {
                    return Err(ErrorType::ZipError(ZipErrorData {
                        source: file_path.to_string_lossy().to_string(),
                        target: tempfile.clone(),
                        error: err.to_string(),
                    }));
                }
            }
        }

        Ok(())
    }

    fn unzip(zip_file: &String, dir: &String) {
        
    }
}
