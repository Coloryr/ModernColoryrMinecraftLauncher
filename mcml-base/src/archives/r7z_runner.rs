use std::path::Path;

use mcml_names::i18_items::error_type::{
    ArchiveErrorData, ErrorData,
    ErrorType::{self},
    FileSystemErrorData,
};
use sevenz_rust2::{ArchiveEntry, ArchiveReader, ArchiveWriter, Password};

use crate::{
    archives::{ArchiveProcess, IArchive, should_exclude},
    path_helper,
};

pub(crate) struct R7zProcess {
    base: ArchiveProcess,
}

impl IArchive for R7zProcess {
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

        self.r7z_compress(archive_file, pack_dir, root_path, filter)
    }

    fn decompress(&self, archive_file: &Path, output_dir: &Path) -> Result<(), ErrorType> {
        self.r7z_decompress(archive_file, output_dir)
    }
}

impl R7zProcess {
    pub fn new(base: ArchiveProcess) -> Self {
        Self { base }
    }

    /// 将整个目录压缩为 7z 文件
    fn r7z_compress(
        &self,
        archive_file: &Path,
        pack_dir: &Path,
        root_path: &Path,
        filter: &Option<Vec<String>>,
    ) -> Result<(), ErrorType> {
        let file = path_helper::open_write(archive_file)?;
        let mut archive = ArchiveWriter::new(file).map_err(|err| {
            ErrorType::ArchiveOpenError(FileSystemErrorData {
                path: archive_file.to_path_buf(),
                error: err.to_string(),
            })
        })?;

        let entries = path_helper::get_all_files(pack_dir);
        self.base.set_count(entries.len());

        for path in entries {
            self.base.add_now(&path);

            if let Some(patterns) = filter {
                if should_exclude(&path, patterns) {
                    continue;
                }
            }

            let buffer = path_helper::open_read(&path)?;

            let relative_path = path.strip_prefix(root_path).unwrap();
            let tempfile = relative_path.to_string_lossy().to_string();

            archive
                .push_archive_entry(
                    ArchiveEntry::from_path(&path, tempfile.clone()),
                    Some(buffer),
                )
                .map_err(|err| {
                    ErrorType::ArchiveError(ArchiveErrorData {
                        source: path.display().to_string(),
                        target: tempfile,
                        error: err.to_string(),
                    })
                })?;
        }

        match archive.finish() {
            Ok(_) => Ok(()),
            Err(err) => Err(ErrorType::ArchiveWriteError(ErrorData {
                error: err.to_string(),
            })),
        }
    }

    /// 解压 7z 文件到指定目录
    fn r7z_decompress(
        &self,
        archive_file: &Path,
        output_dir: &Path,
    ) -> Result<(), ErrorType> {
        let file = path_helper::open_read(archive_file)?;
        path_helper::create_dir_all(output_dir)?;

        let mut seven = ArchiveReader::new(file, Password::empty()).map_err(|err| {
            ErrorType::ArchiveOpenError(FileSystemErrorData {
                path: archive_file.to_path_buf(),
                error: err.to_string(),
            })
        })?;

        self.base.set_count(seven.archive().files.len());

        match seven.for_each_entries(|entry, reader| {
            let dest_path = output_dir.join(entry.name());
            self.base.add_now(&dest_path);
            sevenz_rust2::default_entry_extract_fn(entry, reader, &dest_path)
        }) {
            Ok(_) => Ok(()),
            Err(err) => Err(ErrorType::ArchiveReadError(ErrorData {
                error: err.to_string(),
            })),
        }
    }
}
