use std::{
    fs, io::{Read, Seek, SeekFrom, Write}, path::{Path, PathBuf}, sync::Arc,
};

use flate2::read::GzDecoder;
use mcml_names::{
    i18_items::error_type::{
        ArchiveErrorData, CoreResult, ErrorData, ErrorType, FileSystemErrorData,
    },
    names,
};
use sevenz_rust2::{ArchiveReader, Password};
use tar::Archive;
use uuid::Uuid;
use xz2::read::XzDecoder;
use zip::ZipArchive;

use crate::{
    archives::{compress, decompress, ArchiveGui, ArchiveType},
    path_helper,
};

/// Information about an entry in an archive.
#[derive(Debug, Clone)]
pub struct ArchiveEntryInfo {
    /// Entry name / path within the archive.
    pub name: String,
    /// Whether this entry is a directory.
    pub is_dir: bool,
    /// Uncompressed size in bytes. `0` for directories.
    pub size: u64,
}

impl ArchiveType {
    /// Auto-detect archive type from a file path extension.
    ///
    /// Returns `None` if the extension is not a supported archive format.
    pub fn try_from_path(path: &Path) -> Option<Self> {
        let file_name = path.file_name()?.to_string_lossy().to_lowercase();

        if file_name.ends_with(names::ZIP_DOT_EXT) {
            Some(ArchiveType::Zip)
        } else if file_name.ends_with(names::R7Z_DOT_EXT) {
            Some(ArchiveType::R7Z)
        } else if file_name.ends_with(names::TAR_GZ_DOT_EXT) || file_name.ends_with(names::TGZ_DOT_EXT) {
            Some(ArchiveType::TarGz)
        } else if file_name.ends_with(names::TAR_XZ_DOT_EXT) || file_name.ends_with(names::TXZ_DOT_EXT) {
            Some(ArchiveType::TarXz)
        } else if file_name.ends_with(names::TAR_EXT) {
            Some(ArchiveType::Tar)
        } else {
            None
        }
    }
}

/// A unified archive handler that auto-detects the archive type and provides
/// read/write access to archive contents.
///
/// # Examples
///
/// ```ignore
/// use mcml_base::archives::BaseArchive;
///
/// // Open an archive (type auto-detected from extension)
/// let archive = BaseArchive::open("path/to/file.zip").unwrap();
///
/// // Iterate over entries
/// for entry in archive.entries() {
///     println!("{} ({} bytes)", entry.name, entry.size);
/// }
///
/// // Extract a single file
/// archive.extract_file("readme.txt", "output/readme.txt", None).unwrap();
///
/// // Extract everything
/// archive.extract_all("output_dir/", None).unwrap();
/// ```
pub struct BaseArchive {
    path: PathBuf,
    archive_type: ArchiveType,
    entries: Vec<ArchiveEntryInfo>,
}

impl BaseArchive {
    /// Open an archive file. The archive type is auto-detected from the file
    /// extension.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be opened, the format is
    /// unsupported, or the archive is malformed.
    pub fn open<P: AsRef<Path>>(path: P) -> CoreResult<Self> {
        let path = path.as_ref().to_path_buf();
        let archive_type = ArchiveType::try_from_path(&path).ok_or_else(|| {
            ErrorType::ArchiveOpenError(FileSystemErrorData {
                path: path.clone(),
                error: format!("Unsupported archive format: {}", path.display()),
            })
        })?;

        let entries = Self::read_entries(&path, archive_type)?;

        Ok(Self {
            path,
            archive_type,
            entries,
        })
    }

    /// Returns the detected archive type.
    pub fn archive_type(&self) -> ArchiveType {
        self.archive_type
    }

    /// Returns the path to the archive file on disk.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns all entries in the archive.
    ///
    /// Use this to iterate over the archive contents without extracting.
    pub fn entries(&self) -> &[ArchiveEntryInfo] {
        &self.entries
    }

    /// Checks whether an entry with the given name exists in the archive.
    pub fn contains(&self, name: &str) -> bool {
        self.entries.iter().any(|e| e.name == name)
    }

    /// If all entries share a single top-level directory, returns its name.
    ///
    /// For example, when every entry starts with `"MyWorld/"`, this returns
    /// `Some("MyWorld")`. Returns `None` when entries live at the archive
    /// root or have multiple top-level directories.
    pub fn single_top_dir(&self) -> Option<&str> {
        let mut firsts: Vec<&str> = self
            .entries
            .iter()
            .filter_map(|e| {
                let trimmed = e.name.trim_end_matches(['/', '\\']);
                trimmed.split(['/', '\\']).next()
            })
            .collect();
        firsts.sort_unstable();
        firsts.dedup();
        firsts.retain(|s| !s.is_empty());
        if firsts.len() == 1 {
            Some(firsts[0])
        } else {
            None
        }
    }

    /// Extract a single file from the archive to the given output path.
    ///
    /// * `name` — The entry name/path inside the archive (e.g.
    ///   `"subdir/readme.txt"`).
    /// * `output_path` — The destination file path on disk. Parent directories
    ///   are created automatically.
    /// * `gui` — Optional progress callback.
    ///
    /// # Errors
    ///
    /// Returns an error if the entry is not found or extraction fails.
    pub fn extract_file<P: AsRef<Path>>(
        &self,
        name: &str,
        output_path: P,
        gui: Option<&dyn ArchiveGui>,
    ) -> CoreResult<()> {
        let output_path = output_path.as_ref();

        if let Some(parent) = output_path.parent() {
            path_helper::create_dir_all(parent)?;
        }

        match self.archive_type {
            ArchiveType::Zip => self.extract_file_zip(name, output_path, gui),
            ArchiveType::R7Z => self.extract_file_7z(name, output_path, gui),
            ArchiveType::Tar | ArchiveType::TarGz | ArchiveType::TarXz => {
                self.extract_file_tar(name, output_path, gui)
            }
        }
    }

    /// Extract all files from the archive to the given output directory.
    ///
    /// * `output_dir` — The destination directory.
    /// * `gui` — Optional progress callback.
    pub fn extract_all<P: AsRef<Path>>(
        &self,
        output_dir: P,
        gui: Option<Arc<dyn ArchiveGui>>,
    ) -> CoreResult<()> {
        decompress(
            self.archive_type,
            self.path.as_path(),
            output_dir.as_ref(),
            gui,
        )
    }

    /// Add files to the archive and save in-place.
    ///
    /// For Zip and Tar archives files are appended directly without
    /// extracting or re-compressing existing entries. For 7z and compressed
    /// tar formats the archive is extracted, patched, and re-compressed.
    /// Existing entries with the same internal path are overwritten.
    ///
    /// * `files` — Pairs of `(source_disk_path, internal_archive_path)`.
    /// * `gui` — Optional progress callback (used during re-compression when
    ///   applicable).
    ///
    /// After a successful call the internal entry list is refreshed.
    pub fn add_files<P: AsRef<Path>>(
        &mut self,
        files: &[(P, P)],
        gui: Option<Arc<dyn ArchiveGui>>,
    ) -> CoreResult<()> {
        if files.is_empty() {
            return Ok(());
        }
        match self.archive_type {
            ArchiveType::Zip => self.add_files_zip(files),
            ArchiveType::Tar => self.add_files_tar(files),
            _ => self.add_files_extract_recompress(files, gui),
        }
    }

    /// Zip fast path: append files from disk directly without touching
    /// existing entries.
    fn add_files_zip<P: AsRef<Path>>(&mut self, files: &[(P, P)]) -> CoreResult<()> {
        use zip::{write::SimpleFileOptions, CompressionMethod, ZipWriter};

        let file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&self.path)
            .map_err(|err| {
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
            let internal_path = dest.as_ref().to_string_lossy();

            let mut reader = path_helper::open_read(src.as_ref()).map_err(|err| {
                ErrorType::FileSystemError(FileSystemErrorData {
                    path: src.as_ref().to_path_buf(),
                    error: err.to_string(),
                })
            })?;

            zip.start_file(internal_path.as_ref(), options)
                .map_err(|err| {
                    ErrorType::ArchiveWriteError(ErrorData {
                        error: format!("Cannot start entry '{}': {}", internal_path, err),
                    })
                })?;

            std::io::copy(&mut reader, &mut zip).map_err(|err| {
                ErrorType::ArchiveWriteError(ErrorData {
                    error: format!("Cannot write data for '{}': {}", internal_path, err),
                })
            })?;
        }

        zip.finish().map_err(|err| {
            ErrorType::ArchiveWriteError(ErrorData {
                error: format!("Cannot finalize archive: {}", err),
            })
        })?;

        self.entries = Self::read_entries(&self.path, self.archive_type)?;
        Ok(())
    }

    /// Tar fast path: append files from disk by seeking past the EOF marker
    /// and writing new tar entries directly.
    fn add_files_tar<P: AsRef<Path>>(&mut self, files: &[(P, P)]) -> CoreResult<()> {
        use tar::Builder;

        let mut file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&self.path)
            .map_err(|err| {
                ErrorType::FileSystemError(FileSystemErrorData {
                    path: self.path.clone(),
                    error: err.to_string(),
                })
            })?;

        let len = file.metadata().map_err(|err| {
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
            let internal_path = dest.as_ref().to_string_lossy();
            let mut reader = path_helper::open_read(src.as_ref()).map_err(|err| {
                ErrorType::FileSystemError(FileSystemErrorData {
                    path: src.as_ref().to_path_buf(),
                    error: err.to_string(),
                })
            })?;

            builder
                .append_file(internal_path.as_ref(), &mut reader)
                .map_err(|err| {
                    ErrorType::ArchiveWriteError(ErrorData {
                        error: format!("Cannot append '{}' to tar: {}", internal_path, err),
                    })
                })?;
        }

        builder.finish().map_err(|err| {
            ErrorType::ArchiveWriteError(ErrorData {
                error: format!("Cannot finalize tar: {}", err),
            })
        })?;

        self.entries = Self::read_entries(&self.path, self.archive_type)?;
        Ok(())
    }

    /// Fallback for 7z / tar.gz / tar.xz: extract to temp dir, write data,
    /// re-compress.
    fn add_files_extract_recompress<P: AsRef<Path>>(
        &mut self,
        files: &[(P, P)],
        gui: Option<Arc<dyn ArchiveGui>>,
    ) -> CoreResult<()> {
        // Create a temp directory for the extraction + new files
        let temp_dir = std::env::temp_dir().join(format!("mcml_archive_{}", Uuid::new_v4()));
        path_helper::create_dir_all(&temp_dir)?;

        // Extract existing archive to temp dir (skip if the archive is empty)
        if !self.entries.is_empty() {
            if let Err(err) = decompress(self.archive_type, &self.path, &temp_dir, None) {
                let _ = fs::remove_dir_all(&temp_dir);
                return Err(err);
            }
        }

        // Copy new files into the temp directory
        for (src, dest) in files {
            let dest_path = temp_dir.join(dest.as_ref());
            if let Some(parent) = dest_path.parent() {
                path_helper::create_dir_all(parent)?;
            }
            if let Err(err) = path_helper::copy_file(src.as_ref(), &dest_path) {
                let _ = fs::remove_dir_all(&temp_dir);
                return Err(err);
            }
        }

        // Compress to a temporary archive file first, then atomically replace
        let temp_archive =
            std::env::temp_dir().join(format!("mcml_archive_{}.tmp", Uuid::new_v4()));

        let compress_result = compress(
            self.archive_type,
            temp_archive.as_path(),
            temp_dir.as_path(),
            None::<&Path>,
            &None,
            gui,
        );

        // Clean up temp dir regardless of outcome
        let _ = fs::remove_dir_all(&temp_dir);

        if let Err(err) = compress_result {
            let _ = fs::remove_file(&temp_archive);
            return Err(err);
        }

        // Replace the original archive with the new one
        fs::remove_file(&self.path).map_err(|err| {
            ErrorType::FileSystemError(FileSystemErrorData {
                path: self.path.clone(),
                error: err.to_string(),
            })
        })?;
        fs::rename(&temp_archive, &self.path).map_err(|err| {
            ErrorType::FileSystemError(FileSystemErrorData {
                path: self.path.clone(),
                error: err.to_string(),
            })
        })?;

        // Refresh the internal entry list
        self.entries = Self::read_entries(&self.path, self.archive_type)?;

        Ok(())
    }

    /// Add an in-memory file to the archive and save in-place.
    ///
    /// For Zip archives this appends directly without extracting or
    /// re-compressing existing entries. For 7z and tar-based formats the
    /// archive is extracted, patched, and re-compressed.
    ///
    /// An existing entry with the same internal path is overwritten.
    ///
    /// * `name` — The entry path inside the archive (e.g. `"subdir/readme.txt"`).
    /// * `data` — The file content as raw bytes.
    /// * `gui` — Optional progress callback (used during re-compression when
    ///   applicable).
    ///
    /// After a successful call the internal entry list is refreshed.
    pub fn add_data(
        &mut self,
        name: &str,
        data: &[u8],
        gui: Option<Arc<dyn ArchiveGui>>,
    ) -> CoreResult<()> {
        match self.archive_type {
            ArchiveType::Zip => self.add_data_zip(name, data),
            ArchiveType::Tar => self.add_data_tar(name, data),
            _ => self.add_data_extract_recompress(name, data, gui),
        }
    }

    /// Zip fast path: append in-memory data directly without touching existing
    /// entries.
    fn add_data_zip(&mut self, name: &str, data: &[u8]) -> CoreResult<()> {
        use zip::{write::SimpleFileOptions, CompressionMethod, ZipWriter};

        let file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&self.path)
            .map_err(|err| {
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
                error: format!("Cannot start entry '{}': {}", name, err),
            })
        })?;

        zip.write_all(data).map_err(|err| {
            ErrorType::ArchiveWriteError(ErrorData {
                error: format!("Cannot write data for '{}': {}", name, err),
            })
        })?;

        zip.finish().map_err(|err| {
            ErrorType::ArchiveWriteError(ErrorData {
                error: format!("Cannot finalize archive: {}", err),
            })
        })?;

        // Refresh the internal entry list
        self.entries = Self::read_entries(&self.path, self.archive_type)?;

        Ok(())
    }

    /// Tar fast path: append in-memory data by seeking past the EOF marker and
    /// writing a new tar entry directly.
    fn add_data_tar(&mut self, name: &str, data: &[u8]) -> CoreResult<()> {
        use tar::{Builder, Header};

        let mut file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&self.path)
            .map_err(|err| {
                ErrorType::FileSystemError(FileSystemErrorData {
                    path: self.path.clone(),
                    error: err.to_string(),
                })
            })?;

        // Seek past the tar EOF marker (two 512-byte zero blocks) so the
        // builder overwrites them with the new entry + a fresh EOF marker.
        let len = file.metadata().map_err(|err| {
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
                    error: format!("Cannot append '{}' to tar: {}", name, err),
                })
            })?;

        builder.finish().map_err(|err| {
            ErrorType::ArchiveWriteError(ErrorData {
                error: format!("Cannot finalize tar: {}", err),
            })
        })?;

        // Refresh the internal entry list
        self.entries = Self::read_entries(&self.path, self.archive_type)?;

        Ok(())
    }

    /// Fallback for 7z / tar.gz / tar.xz: extract to temp dir, write data, re-compress.
    fn add_data_extract_recompress(
        &mut self,
        name: &str,
        data: &[u8],
        gui: Option<Arc<dyn ArchiveGui>>,
    ) -> CoreResult<()> {
        // Create a temp directory for the extraction + new file
        let temp_dir = std::env::temp_dir().join(format!("mcml_archive_{}", Uuid::new_v4()));
        path_helper::create_dir_all(&temp_dir)?;

        // Extract existing archive to temp dir (skip if the archive is empty)
        if !self.entries.is_empty() {
            if let Err(err) = decompress(self.archive_type, &self.path, &temp_dir, None) {
                let _ = fs::remove_dir_all(&temp_dir);
                return Err(err);
            }
        }

        // Write the in-memory data to the target path inside the temp directory
        let dest_path = temp_dir.join(name);
        if let Some(parent) = dest_path.parent() {
            path_helper::create_dir_all(parent)?;
        }
        let mut file = path_helper::open_write(&dest_path)?;
        file.write_all(data).map_err(|err| {
            ErrorType::FileSystemError(FileSystemErrorData {
                path: dest_path.clone(),
                error: err.to_string(),
            })
        })?;

        // Compress to a temporary archive file first, then atomically replace
        let temp_archive =
            std::env::temp_dir().join(format!("mcml_archive_{}.tmp", Uuid::new_v4()));

        let compress_result = compress(
            self.archive_type,
            temp_archive.as_path(),
            temp_dir.as_path(),
            None::<&Path>,
            &None,
            gui,
        );

        // Clean up temp dir regardless of outcome
        let _ = fs::remove_dir_all(&temp_dir);

        if let Err(err) = compress_result {
            let _ = fs::remove_file(&temp_archive);
            return Err(err);
        }

        // Replace the original archive with the new one
        fs::remove_file(&self.path).map_err(|err| {
            ErrorType::FileSystemError(FileSystemErrorData {
                path: self.path.clone(),
                error: err.to_string(),
            })
        })?;
        fs::rename(&temp_archive, &self.path).map_err(|err| {
            ErrorType::FileSystemError(FileSystemErrorData {
                path: self.path.clone(),
                error: err.to_string(),
            })
        })?;

        // Refresh the internal entry list
        self.entries = Self::read_entries(&self.path, self.archive_type)?;

        Ok(())
    }

    // ------------------------------------------------------------------
    // Private helpers
    // ------------------------------------------------------------------

    /// Read all entries from an archive of the given type.
    fn read_entries(path: &Path, archive_type: ArchiveType) -> CoreResult<Vec<ArchiveEntryInfo>> {
        match archive_type {
            ArchiveType::Zip => Self::read_entries_zip(path),
            ArchiveType::R7Z => Self::read_entries_7z(path),
            ArchiveType::Tar | ArchiveType::TarGz | ArchiveType::TarXz => {
                Self::read_entries_tar(path, archive_type)
            }
        }
    }

    fn read_entries_zip(path: &Path) -> CoreResult<Vec<ArchiveEntryInfo>> {
        let file = path_helper::open_read(path)?;
        let mut archive = ZipArchive::new(file).map_err(|err| {
            ErrorType::ArchiveOpenError(FileSystemErrorData {
                path: path.to_path_buf(),
                error: err.to_string(),
            })
        })?;

        let mut entries = Vec::with_capacity(archive.len());
        for i in 0..archive.len() {
            let entry = archive.by_index(i).map_err(|err| {
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

    fn read_entries_7z(path: &Path) -> CoreResult<Vec<ArchiveEntryInfo>> {
        let file = path_helper::open_read(path)?;
        let archive = ArchiveReader::new(file, Password::empty()).map_err(|err| {
            ErrorType::ArchiveOpenError(FileSystemErrorData {
                path: path.to_path_buf(),
                error: err.to_string(),
            })
        })?;

        let entries = archive
            .archive()
            .files
            .iter()
            .map(|f| ArchiveEntryInfo {
                name: f.name().to_string(),
                is_dir: f.is_directory(),
                size: f.size(),
            })
            .collect();
        Ok(entries)
    }

    fn read_entries_tar(
        path: &Path,
        archive_type: ArchiveType,
    ) -> CoreResult<Vec<ArchiveEntryInfo>> {
        let file = path_helper::open_read(path)?;
        let mut archive = Self::open_tar_reader(file, archive_type)?;

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
                    .map_err(|err| ErrorType::ArchiveReadError(ErrorData {
                        error: err.to_string(),
                    }))?
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

    /// Open a tar reader based on the archive type (plain / Gz / Xz).
    fn open_tar_reader(
        file: fs::File,
        archive_type: ArchiveType,
    ) -> CoreResult<Archive<Box<dyn Read>>> {
        match archive_type {
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

    // --- Per-format single-file extractors ---

    fn extract_file_zip(
        &self,
        name: &str,
        output_path: &Path,
        gui: Option<&dyn ArchiveGui>,
    ) -> CoreResult<()> {
        if let Some(gui) = gui {
            gui.start(1);
        }

        let file = path_helper::open_read(&self.path)?;
        let mut archive = ZipArchive::new(file).map_err(|err| {
            ErrorType::ArchiveOpenError(FileSystemErrorData {
                path: self.path.clone(),
                error: err.to_string(),
            })
        })?;

        let mut entry = archive.by_name(name).map_err(|err| {
            ErrorType::ArchiveReadError(ErrorData {
                error: format!("Entry '{}' not found: {}", name, err),
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

    fn extract_file_7z(
        &self,
        name: &str,
        output_path: &Path,
        gui: Option<&dyn ArchiveGui>,
    ) -> CoreResult<()> {
        if let Some(gui) = gui {
            gui.start(1);
        }

        let file = path_helper::open_read(&self.path)?;
        let mut archive = ArchiveReader::new(file, Password::empty()).map_err(|err| {
            ErrorType::ArchiveOpenError(FileSystemErrorData {
                path: self.path.clone(),
                error: err.to_string(),
            })
        })?;

        let mut found = false;
        let output_pb = output_path.to_path_buf();
        archive
            .for_each_entries(|entry, reader| {
                if entry.name() == name {
                    found = true;
                    if let Some(parent) = output_pb.parent() {
                        fs::create_dir_all(parent).ok();
                    }
                    sevenz_rust2::default_entry_extract_fn(entry, reader, &output_pb)
                } else {
                    Ok(true)
                }
            })
            .map_err(|err| {
                ErrorType::ArchiveReadError(ErrorData {
                    error: err.to_string(),
                })
            })?;

        if !found {
            return Err(ErrorType::ArchiveReadError(ErrorData {
                error: format!("Entry '{}' not found in archive", name),
            }));
        }

        if let Some(gui) = gui {
            gui.update(Some(name.to_string()), 1);
        }

        Ok(())
    }

    fn extract_file_tar(
        &self,
        name: &str,
        output_path: &Path,
        gui: Option<&dyn ArchiveGui>,
    ) -> CoreResult<()> {
        if let Some(gui) = gui {
            gui.start(1);
        }

        let file = path_helper::open_read(&self.path)?;
        let mut archive = Self::open_tar_reader(file, self.archive_type)?;

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
                error: format!("Entry '{}' not found in archive", name),
            }));
        }

        if let Some(gui) = gui {
            gui.update(Some(name.to_string()), 1);
        }

        Ok(())
    }
}
