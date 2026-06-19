use std::{io::Read, path::Path};

use flate2::read::GzDecoder;
use flate2::{Compression, write::GzEncoder};
use mcml_names::i18_items::error_type::{ArchiveErrorData, ErrorData, ErrorType};
use tar::{Archive, Builder};
use xz2::{read::XzDecoder, write::XzEncoder};

use crate::archives::TarMode;
use crate::{
    archives::{ArchiveProcess, ArchiveRun, should_exclude},
    path_helper,
};

pub(crate) struct TarProcess {
    base: ArchiveProcess,
    mode: Option<TarMode>,
}

impl ArchiveRun for TarProcess {
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

        let mode = self
            .mode
            .unwrap_or(TarMode::try_from_path(archive_file).unwrap_or(TarMode::Gz));

        self.tar(archive_file, pack_dir, root_path, filter, mode)
    }

    fn decompress(&self, archive_file: &Path, output_dir: &Path) -> Result<(), ErrorType> {
        let mode = self
            .mode
            .unwrap_or(TarMode::try_from_path(archive_file).unwrap_or(TarMode::Gz));

        self.un_tar(archive_file, output_dir, mode)
    }
}

impl TarProcess {
    pub fn new(base: ArchiveProcess, mode: Option<TarMode>) -> Self {
        Self { base, mode }
    }

    fn tar(
        &self,
        archive_file: &Path,
        pack_dir: &Path,
        root_path: &Path,
        filter: &Option<Vec<String>>,
        mode: TarMode,
    ) -> Result<(), ErrorType> {
        let file = path_helper::open_write(archive_file)?;

        let files = path_helper::get_all_files(pack_dir);

        let mut tar_builder: Builder<Box<dyn std::io::Write>> = match mode {
            TarMode::Gz => {
                let gz_encoder = GzEncoder::new(file, Compression::default());
                Builder::new(Box::new(gz_encoder))
            }
            TarMode::Xz => {
                let xz_encoder = XzEncoder::new(file, 6);
                Builder::new(Box::new(xz_encoder))
            }
        };

        for path in files {
            self.base.add_now(&path);

            if let Some(patterns) = filter {
                if should_exclude(&path, patterns) {
                    continue;
                }
            }

            let relative_path = path.strip_prefix(root_path).unwrap();
            let archive_path = relative_path.to_string_lossy().to_string();

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

    fn un_tar(
        &self,
        archive_file: &Path,
        output_dir: &Path,
        mode: TarMode,
    ) -> Result<(), ErrorType> {
        path_helper::create_dir_all(output_dir)?;

        {
            let file = path_helper::open_read(archive_file)?;
            let mut archive: Archive<Box<dyn Read>> = match mode {
                TarMode::Gz => {
                    let gz = GzDecoder::new(file);
                    Archive::new(Box::new(gz))
                }
                TarMode::Xz => {
                    let xz = XzDecoder::new(file);
                    Archive::new(Box::new(xz))
                }
            };
            let count = archive
                .entries()
                .map_err(|err| {
                    ErrorType::ArchiveReadError(ErrorData {
                        error: err.to_string(),
                    })
                })?
                .count();
            self.base.set_count(count);
        }

        let file = path_helper::open_read(archive_file)?;
        let mut archive: Archive<Box<dyn Read>> = match mode {
            TarMode::Gz => {
                let gz = GzDecoder::new(file);
                Archive::new(Box::new(gz))
            }
            TarMode::Xz => {
                let xz = XzDecoder::new(file);
                Archive::new(Box::new(xz))
            }
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
            self.base.add_now(&path);
            entry.unpack_in(output_dir).map_err(|err| {
                ErrorType::ArchiveError(ArchiveErrorData {
                    source: path.display().to_string(),
                    target: output_dir.display().to_string(),
                    error: err.to_string(),
                })
            })?;
        }

        Ok(())
    }
}
