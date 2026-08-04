//! 压缩包处理核心逻辑
//!
//! 实现压缩/解压的统一入口，根据 [`ArchiveType`] 分派到对应的 runner。

use std::{
    fs,
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
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
    archives::{
        ArchiveProcess, ArchiveRun, ArchiveType, IBaseArchiveGui, TarMode,
        r7z_runner::R7zProcess, replace_invalid_name, tar_runner::TarProcess,
        zip_runner::ZipProcess,
    },
    path_helper,
};

/// 压缩包条目信息
#[derive(Debug, Clone)]
pub struct ArchiveEntryInfo {
    /// 条目名称 / 压缩包内路径
    pub name: String,
    /// 是否为目录
    pub is_dir: bool,
    /// 未压缩大小（字节），目录为 `0`
    pub size: u64,
}

impl ArchiveType {
    /// 根据文件路径后缀自动检测压缩包类型。
    ///
    /// 后缀不受支持时返回 `None`。
    pub fn try_from_path(path: &Path) -> Option<Self> {
        let file_name = path.file_name()?.to_string_lossy().to_lowercase();

        if file_name.ends_with(names::ZIP_DOT_EXT) {
            Some(ArchiveType::Zip)
        } else if file_name.ends_with(names::R7Z_DOT_EXT) {
            Some(ArchiveType::R7Z)
        } else if file_name.ends_with(names::TAR_GZ_DOT_EXT)
            || file_name.ends_with(names::TGZ_DOT_EXT)
        {
            Some(ArchiveType::TarGz)
        } else if file_name.ends_with(names::TAR_XZ_DOT_EXT)
            || file_name.ends_with(names::TXZ_DOT_EXT)
        {
            Some(ArchiveType::TarXz)
        } else if file_name.ends_with(names::TAR_EXT) {
            Some(ArchiveType::Tar)
        } else {
            None
        }
    }
}

/// 统一的压缩包处理器，自动检测压缩包格式并提供读写功能。
///
/// # 示例
///
/// ```ignore
/// use mcml_base::archives::BaseArchive;
///
/// // 打开压缩包（根据后缀自动检测类型）
/// let archive = BaseArchive::open("path/to/file.zip").unwrap();
///
/// // 遍历条目
/// for entry in archive.entries() {
///     println!("{} ({} bytes)", entry.name, entry.size);
/// }
///
/// // 提取单个文件
/// archive.extract_file("readme.txt", "output/readme.txt", None).unwrap();
///
/// // 提取全部文件
/// archive.extract_all("output_dir/", None, None).unwrap();
/// ```
pub struct BaseArchive {
    /// 压缩包磁盘路径
    path: PathBuf,
    /// 压缩包类型
    archive_type: ArchiveType,
    /// 条目列表缓存
    entries: Vec<ArchiveEntryInfo>,
    /// 始终保持打开的文件句柄。Zip 格式下此句柄独立于缓存的
    /// [`ZipArchive`]；其他格式下每次读取操作通过克隆此句柄获得
    /// 独立的文件描述符，避免反复打开文件。
    file: fs::File,
    /// 缓存的 [`ZipArchive`]，中央目录仅解析一次。
    /// 非 Zip 格式为 `None`。
    zip: Mutex<Option<ZipArchive<fs::File>>>,
}

impl BaseArchive {
    /// 打开压缩包文件，根据文件扩展名自动检测类型。
    ///
    /// # 错误
    ///
    /// 文件无法打开、格式不支持或压缩包损坏时返回错误。
    pub fn open<P: AsRef<Path>>(path: P) -> CoreResult<Self> {
        let path = path.as_ref().to_path_buf();
        let archive_type = ArchiveType::try_from_path(&path).ok_or_else(|| {
            ErrorType::ArchiveOpenError(FileSystemErrorData {
                path: path.clone(),
                error: String::new(),
            })
        })?;

        let file = path_helper::open_read(&path)?;

        let (entries, zip) = match archive_type {
            ArchiveType::Zip => {
                // 克隆句柄，让 ZipArchive 拥有独立的文件描述符
                let zip_file = file.try_clone().map_err(|err| {
                    ErrorType::FileSystemError(FileSystemErrorData {
                        path: path.clone(),
                        error: err.to_string(),
                    })
                })?;
                let mut zip_archive = ZipArchive::new(zip_file).map_err(|err| {
                    ErrorType::ArchiveOpenError(FileSystemErrorData {
                        path: path.clone(),
                        error: err.to_string(),
                    })
                })?;
                let entries = Self::read_entries_zip_archive(&mut zip_archive)?;
                (entries, Mutex::new(Some(zip_archive)))
            }
            _ => {
                // 从克隆句柄读取条目，保留原句柄供后续使用
                let clone = file.try_clone().map_err(|err| {
                    ErrorType::FileSystemError(FileSystemErrorData {
                        path: path.clone(),
                        error: err.to_string(),
                    })
                })?;
                let entries = Self::read_entries(clone, archive_type)?;
                (entries, Mutex::new(None))
            }
        };

        Ok(Self {
            path,
            archive_type,
            entries,
            file,
            zip,
        })
    }

    /// 从目录创建新的压缩包并返回已打开的状态。
    ///
    /// 创建成功后可通过 [`add_files`](Self::add_files) /
    /// [`add_data`](Self::add_data) 继续添加内容。
    ///
    /// * `output_file` — 压缩包输出路径。
    /// * `source_dir` — 需要打包的源目录。
    /// * `root_path` — 相对根路径，设置后文件在包内的路径相对于此。
    /// * `filter` — 可选的排除子串列表，匹配的文件将被跳过。
    pub fn compress<P: AsRef<Path>>(
        archive_type: ArchiveType,
        output_file: P,
        source_dir: P,
        root_path: Option<P>,
        filter: &Option<Vec<String>>,
        gui: Option<Arc<dyn IBaseArchiveGui>>,
    ) -> CoreResult<Self> {
        Self::compress_inner(
            archive_type,
            output_file.as_ref(),
            source_dir.as_ref(),
            root_path.as_ref().map(|p| p.as_ref()),
            filter,
            gui,
        )?;
        Self::open(output_file)
    }

    /// 创建一个空的压缩包（无条目）并返回已打开的状态。
    ///
    /// 创建后通过 [`add_files`](Self::add_files) /
    /// [`add_data`](Self::add_data) 填充内容。
    ///
    /// * `archive_type` — 压缩包格式。
    /// * `output_file` — 压缩包输出路径。
    pub fn create_empty<P: AsRef<Path>>(
        archive_type: ArchiveType,
        output_file: P,
    ) -> CoreResult<Self> {
        let output_file = output_file.as_ref();
        match archive_type {
            ArchiveType::Zip => Self::create_empty_zip(output_file)?,
            ArchiveType::Tar => Self::create_empty_tar(output_file)?,
            // 7z / TarGz / TarXz：压缩空目录后再打开
            _ => {
                let temp_dir = std::env::temp_dir().join(format!("mcml_empty_{}", Uuid::new_v4()));
                path_helper::create_dir_all(&temp_dir)?;
                let res =
                    Self::compress_inner(archive_type, output_file, &temp_dir, None, &None, None);
                let _ = fs::remove_dir_all(&temp_dir);
                res?;
            }
        }
        Self::open(output_file)
    }

    /// 解压压缩包到指定目录，无需构造 [`BaseArchive`] 实例。
    pub fn decompress<P: AsRef<Path>>(
        archive_type: ArchiveType,
        archive_file: P,
        output_dir: P,
        gui: Option<Arc<dyn IBaseArchiveGui>>,
    ) -> CoreResult<()> {
        Self::make_runner(archive_type, gui)?.decompress(archive_file.as_ref(), output_dir.as_ref())
    }

    /// 内部辅助：仅执行压缩写入磁盘，不打开结果文件。
    fn compress_inner(
        archive_type: ArchiveType,
        output_file: &Path,
        source_dir: &Path,
        root_path: Option<&Path>,
        filter: &Option<Vec<String>>,
        gui: Option<Arc<dyn IBaseArchiveGui>>,
    ) -> CoreResult<()> {
        Self::make_runner(archive_type, gui)?.compress(output_file, source_dir, root_path, filter)
    }

    /// 根据压缩包类型创建对应的执行器。
    fn make_runner(
        archive_type: ArchiveType,
        gui: Option<Arc<dyn IBaseArchiveGui>>,
    ) -> CoreResult<Box<dyn ArchiveRun + Send + Sync>> {
        let process = ArchiveProcess::new(gui);
        Ok(match archive_type {
            ArchiveType::Zip => Box::new(ZipProcess::new(process)),
            ArchiveType::R7Z => Box::new(R7zProcess::new(process)),
            ArchiveType::Tar => Box::new(TarProcess::new(process, None)),
            ArchiveType::TarGz => Box::new(TarProcess::new(process, Some(TarMode::Gz))),
            ArchiveType::TarXz => Box::new(TarProcess::new(process, Some(TarMode::Xz))),
        })
    }

    /// 在磁盘上创建一个空的 zip 文件（仅含中央目录，零条目）。
    fn create_empty_zip(path: &Path) -> CoreResult<()> {
        use zip::ZipWriter;

        let file = fs::File::create(path).map_err(|err| {
            ErrorType::FileSystemError(FileSystemErrorData {
                path: path.to_path_buf(),
                error: err.to_string(),
            })
        })?;

        let zip = ZipWriter::new(file);
        zip.finish().map_err(|err| {
            ErrorType::ArchiveWriteError(ErrorData {
                error: err.to_string(),
            })
        })?;

        Ok(())
    }

    /// 在磁盘上创建一个空的 tar 文件（头 + 两个零块 EOF 标记）。
    fn create_empty_tar(path: &Path) -> CoreResult<()> {
        use tar::Builder;

        let file = fs::File::create(path).map_err(|err| {
            ErrorType::FileSystemError(FileSystemErrorData {
                path: path.to_path_buf(),
                error: err.to_string(),
            })
        })?;

        let mut builder = Builder::new(file);
        builder.finish().map_err(|err| {
            ErrorType::ArchiveWriteError(ErrorData {
                error: err.to_string(),
            })
        })?;

        Ok(())
    }

    /// 返回检测到的压缩包类型。
    pub fn archive_type(&self) -> ArchiveType {
        self.archive_type
    }

    /// 返回压缩包在磁盘上的路径。
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// 返回压缩包内所有条目。
    ///
    /// 用于在不解压的情况下遍历压缩包内容。
    pub fn entries(&self) -> &[ArchiveEntryInfo] {
        &self.entries
    }

    /// 检查压缩包中是否存在指定名称的条目。
    pub fn contains(&self, name: &str) -> bool {
        self.entries.iter().any(|e| e.name == name)
    }

    /// 如果所有条目共享同一个顶层目录，返回该目录名。
    ///
    /// 例如所有条目都以 `"MyWorld/"` 开头时返回
    /// `Some("MyWorld")`。条目位于压缩包根目录或有多个
    /// 顶层目录时返回 `None`。
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

    /// 读取压缩包内单个条目的内容到内存。
    ///
    /// * `name` — 条目在压缩包内的名称/路径（如 `"subdir/readme.txt"`）。
    ///
    /// 返回条目内容的原始字节。
    ///
    /// # 错误
    ///
    /// 条目不存在、是目录或读取失败时返回错误。
    pub fn read(&self, name: &str) -> CoreResult<Vec<u8>> {
        match self.archive_type {
            ArchiveType::Zip => self.read_zip(name),
            ArchiveType::R7Z => self.read_7z(name),
            ArchiveType::Tar | ArchiveType::TarGz | ArchiveType::TarXz => self.read_tar(name),
        }
    }

    /// 以流式 [`Read`] 接口打开压缩包内的单个文件条目。
    ///
    /// 与返回完整 `Vec<u8>` 的 [`read`](Self::read) 不同，此方法返回
    /// `Box<dyn Read>`，可直接传递给任何接受 [`Read`] trait 的函数。
    ///
    /// * `name` — 条目在压缩包内的名称/路径（如 `"subdir/readme.txt"`）。
    ///
    /// # 错误
    ///
    /// 条目不存在、是目录或读取失败时返回错误。
    pub fn read_stream(&self, name: &str) -> CoreResult<Box<dyn Read>> {
        match self.archive_type {
            ArchiveType::Zip => self.read_zip_stream(name),
            ArchiveType::R7Z => self.read_7z_stream(name),
            ArchiveType::Tar | ArchiveType::TarGz | ArchiveType::TarXz => {
                self.read_tar_stream(name)
            }
        }
    }

    /// 将压缩包中单个文件提取到指定输出路径。
    ///
    /// * `name` — 条目在压缩包内的名称/路径（如 `"subdir/readme.txt"`）。
    /// * `output_path` — 目标磁盘路径，父目录会自动创建。
    /// * `gui` — 可选的进度回调。
    ///
    /// # 错误
    ///
    /// 条目不存在或提取失败时返回错误。
    pub fn extract_file<P: AsRef<Path>>(
        &self,
        name: &str,
        output_path: P,
        gui: Option<&dyn IBaseArchiveGui>,
    ) -> CoreResult<()> {
        let output_path = output_path.as_ref();

        // 文件名非法时替换非法字符为 `_`，GUI 可返回自定义名字覆盖（仅替换目标路径的最后一个名称段）
        let output_path = if name
            .split(['/', '\\'])
            .filter(|seg| !seg.is_empty() && *seg != "." && *seg != "..")
            .any(|seg| path_helper::file_has_invalid_chars(seg))
        {
            let safe_name = replace_invalid_name(name);
            let new_name = match gui {
                Some(gui) => gui
                    .file_rename(name)
                    .filter(|n| !n.is_empty())
                    .unwrap_or(safe_name),
                None => safe_name,
            };
            let new_file = Path::new(&new_name)
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| name.to_string());
            output_path.with_file_name(new_file)
        } else {
            output_path.to_path_buf()
        };

        if let Some(parent) = output_path.parent() {
            path_helper::create_dir_all(parent)?;
        }

        match self.archive_type {
            ArchiveType::Zip => self.extract_file_zip(name, &output_path, gui),
            ArchiveType::R7Z => self.extract_file_7z(name, &output_path, gui),
            ArchiveType::Tar | ArchiveType::TarGz | ArchiveType::TarXz => {
                self.extract_file_tar(name, &output_path, gui)
            }
        }
    }

    /// 将压缩包中所有文件提取到指定输出目录。
    ///
    /// * `output_dir` — 目标目录。
    /// * `unselect` — 可选的排除条目名列表，名字完全匹配的条目将被跳过。
    /// * `gui` — 可选的进度回调。
    pub fn extract_all<P: AsRef<Path>>(
        &self,
        output_dir: P,
        unselect: Option<Vec<String>>,
        gui: Option<Arc<dyn IBaseArchiveGui>>,
    ) -> CoreResult<()> {
        let Some(patterns) = unselect else {
            // 无排除项时走 runner 的批量解压路径
            return Self::decompress(
                self.archive_type,
                self.path.as_path(),
                output_dir.as_ref(),
                gui,
            );
        };

        // 有排除项时逐个提取，跳过名字完全匹配的条目
        let output_dir = output_dir.as_ref();
        self.extract_where(
            |entry| {
                if patterns.iter().any(|u| u == &entry.name) {
                    None
                } else {
                    Some(output_dir.join(&entry.name))
                }
            },
            gui.as_deref(),
        )
    }

    /// 按条件提取选中条目到由闭包计算的输出路径。
    ///
    /// 闭包接收每个文件 [`ArchiveEntryInfo`]，返回 `Some(输出路径)` 则提取，
    /// 返回 `None` 则跳过。目录条目始终自动跳过。
    ///
    /// 此方法比反复调用 [`extract_file`](Self::extract_file) 更高效，
    /// 因为缓存的压缩包读取器（zip）或文件句柄（其他格式）在所有提取过程中复用。
    ///
    /// * `map` — 将每个条目映射为可选输出路径的闭包。
    /// * `gui` — 可选的进度回调（按每个已提取文件触发）。
    pub fn extract_where<F: FnMut(&ArchiveEntryInfo) -> Option<PathBuf>>(
        &self,
        mut map: F,
        gui: Option<&dyn IBaseArchiveGui>,
    ) -> CoreResult<()> {
        for entry in &self.entries {
            if entry.is_dir {
                continue;
            }
            if let Some(output) = map(entry) {
                self.extract_file(&entry.name, &output, gui)?;
            }
        }
        if let Some(gui) = gui {
            gui.done();
        }
        Ok(())
    }

    /// 向压缩包添加文件并原地保存。
    ///
    /// Zip 和 Tar 格式下文件直接追加，无需解压或重压缩已有条目。
    /// 7z 和压缩 tar 格式则先解压到临时目录，合并文件后重压缩。
    /// 已有同路径条目会被覆盖。
    ///
    /// * `files` — `(磁盘源路径, 压缩包内路径)` 对。
    /// * `gui` — 可选的进度回调（仅在重压缩时使用）。
    ///
    /// 调用成功后内部条目列表会自动刷新。
    pub fn add_files<P: AsRef<Path>>(
        &mut self,
        files: &[(P, P)],
        gui: Option<Arc<dyn IBaseArchiveGui>>,
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

    /// Zip 快速路径：直接从磁盘追加文件，不触碰已有条目。
    fn add_files_zip<P: AsRef<Path>>(&mut self, files: &[(P, P)]) -> CoreResult<()> {
        use zip::{CompressionMethod, ZipWriter, write::SimpleFileOptions};

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

        self.refresh_after_modify()?;
        Ok(())
    }

    /// Tar 快速路径：通过跳过 EOF 标记后直接写入新 tar 条目来追加文件。
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
                        error: err.to_string(),
                    })
                })?;
        }

        builder.finish().map_err(|err| {
            ErrorType::ArchiveWriteError(ErrorData {
                error: err.to_string(),
            })
        })?;

        self.refresh_after_modify()?;
        Ok(())
    }

    /// 7z / tar.gz / tar.xz 的后备路径：解压到临时目录，写入文件后重压缩。
    fn add_files_extract_recompress<P: AsRef<Path>>(
        &mut self,
        files: &[(P, P)],
        gui: Option<Arc<dyn IBaseArchiveGui>>,
    ) -> CoreResult<()> {
        // 创建临时目录用于解压和新文件
        let temp_dir = std::env::temp_dir().join(format!("mcml_archive_{}", Uuid::new_v4()));
        path_helper::create_dir_all(&temp_dir)?;

        // 将已有压缩包解压到临时目录（空压缩包跳过）
        if !self.entries.is_empty() {
            if let Err(err) = Self::decompress(self.archive_type, &self.path, &temp_dir, None) {
                let _ = fs::remove_dir_all(&temp_dir);
                return Err(err);
            }
        }

        // 将新文件复制到临时目录
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

        // 先压缩到临时文件，再原子替换
        let temp_archive =
            std::env::temp_dir().join(format!("mcml_archive_{}.tmp", Uuid::new_v4()));

        let compress_result = Self::compress_inner(
            self.archive_type,
            temp_archive.as_path(),
            temp_dir.as_path(),
            None::<&Path>,
            &None,
            gui,
        );

        // 无论成功与否都清理临时目录
        let _ = fs::remove_dir_all(&temp_dir);

        if let Err(err) = compress_result {
            let _ = fs::remove_file(&temp_archive);
            return Err(err);
        }

        // 用新压缩包替换原有压缩包
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

        // 刷新内部条目列表
        self.reopen_after_replace()?;

        Ok(())
    }

    /// 将内存中的文件添加到压缩包并原地保存。
    ///
    /// Zip 格式下直接追加，无需解压或重压缩已有条目。
    /// 7z 和 tar 系列格式则先解压到临时目录，写入数据后重压缩。
    /// 已有同路径条目会被覆盖。
    ///
    /// * `name` — 条目在压缩包内的路径（如 `"subdir/readme.txt"`）。
    /// * `data` — 文件内容的原始字节。
    /// * `gui` — 可选的进度回调（仅在重压缩时使用）。
    ///
    /// 调用成功后内部条目列表会自动刷新。
    pub fn add_data(
        &mut self,
        name: &str,
        data: &[u8],
        gui: Option<Arc<dyn IBaseArchiveGui>>,
    ) -> CoreResult<()> {
        match self.archive_type {
            ArchiveType::Zip => self.add_data_zip(name, data),
            ArchiveType::Tar => self.add_data_tar(name, data),
            _ => self.add_data_extract_recompress(name, data, gui),
        }
    }

    /// Zip 快速路径：直接追加内存数据，不触碰已有条目。
    fn add_data_zip(&mut self, name: &str, data: &[u8]) -> CoreResult<()> {
        use zip::{CompressionMethod, ZipWriter, write::SimpleFileOptions};

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

        // 刷新内部条目列表
        self.refresh_after_modify()?;

        Ok(())
    }

    /// Tar 快速路径：通过跳过 EOF 标记后直接写入新 tar 条目来追加内存数据。
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

        // 刷新内部条目列表
        self.refresh_after_modify()?;

        Ok(())
    }

    /// 7z / tar.gz / tar.xz 的后备路径：解压到临时目录，写入数据后重压缩。
    fn add_data_extract_recompress(
        &mut self,
        name: &str,
        data: &[u8],
        gui: Option<Arc<dyn IBaseArchiveGui>>,
    ) -> CoreResult<()> {
        // 创建临时目录用于解压和新文件
        let temp_dir = std::env::temp_dir().join(format!("mcml_archive_{}", Uuid::new_v4()));
        path_helper::create_dir_all(&temp_dir)?;

        // 将已有压缩包解压到临时目录（空压缩包跳过）
        if !self.entries.is_empty() {
            if let Err(err) = Self::decompress(self.archive_type, &self.path, &temp_dir, None) {
                let _ = fs::remove_dir_all(&temp_dir);
                return Err(err);
            }
        }

        // 将内存数据写入临时目录中的目标路径
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

        // 先压缩到临时文件，再原子替换
        let temp_archive =
            std::env::temp_dir().join(format!("mcml_archive_{}.tmp", Uuid::new_v4()));

        let compress_result = Self::compress_inner(
            self.archive_type,
            temp_archive.as_path(),
            temp_dir.as_path(),
            None::<&Path>,
            &None,
            gui,
        );

        // 无论成功与否都清理临时目录
        let _ = fs::remove_dir_all(&temp_dir);

        if let Err(err) = compress_result {
            let _ = fs::remove_file(&temp_archive);
            return Err(err);
        }

        // 用新压缩包替换原有压缩包
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

        // 刷新内部条目列表
        self.reopen_after_replace()?;

        Ok(())
    }

    /// 克隆存储的文件句柄，供需要获取所有权的读取操作使用。
    fn clone_file(&self) -> CoreResult<fs::File> {
        self.file.try_clone().map_err(|err| {
            ErrorType::FileSystemError(FileSystemErrorData {
                path: self.path.clone(),
                error: err.to_string(),
            })
        })
    }

    /// 在磁盘上的压缩包被**原地修改**（追加/覆盖）后刷新条目元数据。
    /// Zip 格式下同时重建缓存的 [`ZipArchive`]。
    fn refresh_after_modify(&mut self) -> CoreResult<()> {
        if self.archive_type == ArchiveType::Zip {
            *self.zip.lock().unwrap() = None;
            let clone = self.clone_file()?;
            let mut new_zip = ZipArchive::new(clone).map_err(|err| {
                ErrorType::ArchiveOpenError(FileSystemErrorData {
                    path: self.path.clone(),
                    error: err.to_string(),
                })
            })?;
            self.entries = Self::read_entries_zip_archive(&mut new_zip)?;
            *self.zip.lock().unwrap() = Some(new_zip);
        } else {
            let clone = self.clone_file()?;
            self.entries = Self::read_entries(clone, self.archive_type)?;
        }
        Ok(())
    }

    /// 在磁盘上的压缩包被**替换**（旧文件删除/重命名，新文件在
    /// `self.path`）后重新打开文件句柄并刷新条目。
    fn reopen_after_replace(&mut self) -> CoreResult<()> {
        self.file = path_helper::open_read(&self.path)?;
        self.refresh_after_modify()
    }

    /// 读取指定类型压缩包的所有条目，消费文件句柄。
    fn read_entries(
        file: fs::File,
        archive_type: ArchiveType,
    ) -> CoreResult<Vec<ArchiveEntryInfo>> {
        match archive_type {
            ArchiveType::Zip => {
                // 仅应在 `open()` 路径下到达此分支，Zip 已在外部处理。
                // 此处作为兜底保护。
                let mut zip = ZipArchive::new(file).map_err(|err| {
                    ErrorType::ArchiveOpenError(FileSystemErrorData {
                        path: PathBuf::new(),
                        error: err.to_string(),
                    })
                })?;
                Self::read_entries_zip_archive(&mut zip)
            }
            ArchiveType::R7Z => Self::read_entries_7z(file),
            ArchiveType::Tar | ArchiveType::TarGz | ArchiveType::TarXz => {
                Self::read_entries_tar(file, archive_type)
            }
        }
    }

    /// 从已打开的 [`ZipArchive`] 读取条目元数据。
    fn read_entries_zip_archive(
        archive: &mut ZipArchive<fs::File>,
    ) -> CoreResult<Vec<ArchiveEntryInfo>> {
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

    /// 读取 7z 压缩包的所有条目。
    fn read_entries_7z(file: fs::File) -> CoreResult<Vec<ArchiveEntryInfo>> {
        let archive = ArchiveReader::new(file, Password::empty()).map_err(|err| {
            ErrorType::ArchiveOpenError(FileSystemErrorData {
                path: PathBuf::new(),
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

    /// 读取 tar（plain / Gz / Xz）压缩包的所有条目。
    fn read_entries_tar(
        file: fs::File,
        archive_type: ArchiveType,
    ) -> CoreResult<Vec<ArchiveEntryInfo>> {
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

    /// 根据压缩包类型（plain / Gz / Xz）创建 tar 读取器。
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

    /// Zip：从缓存的 ZipArchive 中查找并读取条目。
    fn read_zip(&self, name: &str) -> CoreResult<Vec<u8>> {
        let mut zip = self.zip.lock().unwrap();
        let zip = zip.as_mut().ok_or_else(|| {
            ErrorType::ArchiveOpenError(FileSystemErrorData {
                path: self.path.clone(),
                error: String::new(),
            })
        })?;

        let mut entry = zip.by_name(name).map_err(|err| {
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

    /// 7z：克隆文件句柄，创建 ArchiveReader 后遍历查找条目。
    fn read_7z(&self, name: &str) -> CoreResult<Vec<u8>> {
        let file = self.clone_file()?;
        let mut archive = ArchiveReader::new(file, Password::empty()).map_err(|err| {
            ErrorType::ArchiveOpenError(FileSystemErrorData {
                path: self.path.clone(),
                error: err.to_string(),
            })
        })?;

        let mut result: Option<Vec<u8>> = None;
        let mut found = false;
        archive
            .for_each_entries(|entry, reader| {
                if entry.name() == name {
                    found = true;
                    if entry.is_directory() {
                        result = None;
                        return Err(sevenz_rust2::Error::Other("is directory".into()));
                    }
                    let mut buf = Vec::new();
                    std::io::Read::read_to_end(reader, &mut buf)?;
                    result = Some(buf);
                    Ok(false) // 停止遍历
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
                error: name.to_string(),
            }));
        }

        result.ok_or_else(|| {
            ErrorType::ArchiveReadError(ErrorData {
                error: name.to_string(),
            })
        })
    }

    /// Tar：克隆文件句柄，打开 tar 读取器后扫描查找条目。
    fn read_tar(&self, name: &str) -> CoreResult<Vec<u8>> {
        let file = self.clone_file()?;
        let mut archive = Self::open_tar_reader(file, self.archive_type)?;

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

    /// Zip 流式读取：先读取到内存，再包装为 Cursor。
    fn read_zip_stream(&self, name: &str) -> CoreResult<Box<dyn Read>> {
        let data = self.read_zip(name)?;
        Ok(Box::new(std::io::Cursor::new(data)))
    }

    /// 7z 流式读取：先读取到内存，再包装为 Cursor。
    fn read_7z_stream(&self, name: &str) -> CoreResult<Box<dyn Read>> {
        let data = self.read_7z(name)?;
        Ok(Box::new(std::io::Cursor::new(data)))
    }

    /// Tar 流式读取：先读取到内存，再包装为 Cursor。
    fn read_tar_stream(&self, name: &str) -> CoreResult<Box<dyn Read>> {
        let data = self.read_tar(name)?;
        Ok(Box::new(std::io::Cursor::new(data)))
    }

    /// Zip：从缓存读取条目，流式写入目标文件。
    fn extract_file_zip(
        &self,
        name: &str,
        output_path: &Path,
        gui: Option<&dyn IBaseArchiveGui>,
    ) -> CoreResult<()> {
        if let Some(gui) = gui {
            gui.start(1);
        }

        let mut zip = self.zip.lock().unwrap();
        let zip = zip.as_mut().ok_or_else(|| {
            ErrorType::ArchiveOpenError(FileSystemErrorData {
                path: self.path.clone(),
                error: String::new(),
            })
        })?;

        let mut entry = zip.by_name(name).map_err(|err| {
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

    /// 7z：克隆文件句柄，通过 default_entry_extract_fn 提取条目。
    fn extract_file_7z(
        &self,
        name: &str,
        output_path: &Path,
        gui: Option<&dyn IBaseArchiveGui>,
    ) -> CoreResult<()> {
        if let Some(gui) = gui {
            gui.start(1);
        }

        let file = self.clone_file()?;
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
                error: name.to_string(),
            }));
        }

        if let Some(gui) = gui {
            gui.update(Some(name.to_string()), 1);
        }

        Ok(())
    }

    /// Tar：克隆文件句柄，扫描条目后流式写入目标文件。
    fn extract_file_tar(
        &self,
        name: &str,
        output_path: &Path,
        gui: Option<&dyn IBaseArchiveGui>,
    ) -> CoreResult<()> {
        if let Some(gui) = gui {
            gui.start(1);
        }

        let file = self.clone_file()?;
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
                error: name.to_string(),
            }));
        }

        if let Some(gui) = gui {
            gui.update(Some(name.to_string()), 1);
        }

        Ok(())
    }
}
