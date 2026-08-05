//! 压缩包处理核心逻辑
//!
//! 实现压缩/解压的统一入口，根据 [`ArchiveType`] 分派到对应格式的读取器。

use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};

use mcml_names::{
    i18_items::error_type::{CoreResult, ErrorData, ErrorType, FileSystemErrorData},
    names,
};
use uuid::Uuid;

use crate::{
    archives::{
        ArchiveHandle, ArchiveType, IBaseArchiveGui, TarMode, r7z_reader::R7zReader,
        replace_invalid_name, tar_reader::TarReader, zip_reader::ZipReader,
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
    /// 统一的已打开压缩包读取句柄，各格式实现见对应 runner 的读取器。
    handle: Mutex<Box<dyn ArchiveHandle>>,
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

        // 以读写方式打开：读取句柄持有该文件，zip/tar 的就地追加复用同一句柄，无需重新打开
        let file = path_helper::open_read_write(&path)?;
        let mut handle = Self::make_handle(archive_type, &path, file)?;
        let entries = handle.read_entries()?;

        Ok(Self {
            path,
            archive_type,
            entries,
            handle: Mutex::new(handle),
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
        let archive_file = archive_file.as_ref();
        let output_dir = output_dir.as_ref();
        match archive_type {
            ArchiveType::Zip => ZipReader::decompress(archive_file, output_dir, gui),
            ArchiveType::R7Z => R7zReader::decompress(archive_file, output_dir, gui),
            ArchiveType::Tar => TarReader::decompress(None, archive_file, output_dir, gui),
            ArchiveType::TarGz => {
                TarReader::decompress(Some(TarMode::Gz), archive_file, output_dir, gui)
            }
            ArchiveType::TarXz => {
                TarReader::decompress(Some(TarMode::Xz), archive_file, output_dir, gui)
            }
        }
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
        match archive_type {
            ArchiveType::Zip => {
                ZipReader::compress(output_file, source_dir, root_path, filter, gui)
            }
            ArchiveType::R7Z => {
                R7zReader::compress(output_file, source_dir, root_path, filter, gui)
            }
            ArchiveType::Tar => {
                TarReader::compress(None, output_file, source_dir, root_path, filter, gui)
            }
            ArchiveType::TarGz => TarReader::compress(
                Some(TarMode::Gz),
                output_file,
                source_dir,
                root_path,
                filter,
                gui,
            ),
            ArchiveType::TarXz => TarReader::compress(
                Some(TarMode::Xz),
                output_file,
                source_dir,
                root_path,
                filter,
                gui,
            ),
        }
    }

    /// 根据压缩包类型创建对应的读取句柄。
    fn make_handle(
        archive_type: ArchiveType,
        path: &Path,
        file: fs::File,
    ) -> CoreResult<Box<dyn ArchiveHandle>> {
        Ok(match archive_type {
            ArchiveType::Zip => Box::new(ZipReader::new(file, path.to_path_buf())?),
            ArchiveType::R7Z => Box::new(R7zReader::new(file, path.to_path_buf())?),
            ArchiveType::Tar | ArchiveType::TarGz | ArchiveType::TarXz => {
                Box::new(TarReader::new(file, archive_type, path.to_path_buf())?)
            }
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
        self.handle.lock().unwrap().read(name)
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
        self.handle.lock().unwrap().read_stream(name)
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
        let output_path =
            self.resolve_output_path(name, output_path.as_ref().to_path_buf(), gui)?;
        self.extract_resolved(name, &output_path, gui)
    }

    /// 解析最终输出路径：文件名非法时替换非法字符为 `_`，并询问 GUI 是否同意。
    ///
    /// 仅替换目标路径的最后一个名称段；GUI 不同意替换时返回 `TaskCancel`。
    fn resolve_output_path(
        &self,
        name: &str,
        output: PathBuf,
        gui: Option<&dyn IBaseArchiveGui>,
    ) -> CoreResult<PathBuf> {
        if name
            .split(['/', '\\'])
            .filter(|seg| !seg.is_empty() && *seg != "." && *seg != "..")
            .any(|seg| path_helper::file_has_invalid_chars(seg))
        {
            let safe_name = replace_invalid_name(name);
            let new_name = match gui {
                Some(gui) => {
                    if gui.file_rename(name) {
                        safe_name
                    } else {
                        return Err(ErrorType::TaskCancel);
                    }
                }
                None => safe_name,
            };
            let new_file = Path::new(&new_name)
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| name.to_string());
            Ok(output.with_file_name(new_file))
        } else {
            Ok(output)
        }
    }

    /// 提取单个条目到已解析的目标路径（创建父目录）。
    fn extract_resolved(
        &self,
        name: &str,
        output_path: &Path,
        gui: Option<&dyn IBaseArchiveGui>,
    ) -> CoreResult<()> {
        if let Some(parent) = output_path.parent() {
            path_helper::create_dir_all(parent)?;
        }
        self.handle
            .lock()
            .unwrap()
            .extract_file(name, output_path, gui)
    }

    /// 顺序提取一组条目。
    fn extract_tasks_sequential(
        &self,
        tasks: &[(String, PathBuf)],
        gui: Option<&dyn IBaseArchiveGui>,
    ) -> CoreResult<()> {
        let mut current = 0usize;
        for (name, output) in tasks {
            self.extract_resolved(name, output, None)?;
            current += 1;
            if let Some(gui) = gui {
                gui.update(Some(name.clone()), current);
            }
        }
        Ok(())
    }

    /// 并行提取一组条目（仅 Zip：每个线程持有独立的读取句柄随机访问）。
    fn extract_tasks_parallel(
        &self,
        tasks: &[(String, PathBuf)],
        gui: Option<&dyn IBaseArchiveGui>,
    ) -> CoreResult<()> {
        let thread_count = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4)
            .min(tasks.len());
        if thread_count <= 1 {
            return self.extract_tasks_sequential(tasks, gui);
        }

        let current = Arc::new(AtomicUsize::new(0));
        let error: Arc<Mutex<Option<ErrorType>>> = Arc::new(Mutex::new(None));

        std::thread::scope(|scope| {
            for chunk in tasks.chunks(tasks.len().div_ceil(thread_count)) {
                let chunk = chunk.to_vec();
                let path = self.path.clone();
                let gui = gui;
                let current = current.clone();
                let error = error.clone();
                scope.spawn(move || {
                    // 任一线程出错则尽早退出
                    if error.lock().unwrap().is_some() {
                        return;
                    }
                    let file = match path_helper::open_read(&path) {
                        Ok(file) => file,
                        Err(err) => {
                            *error.lock().unwrap() = Some(err);
                            return;
                        }
                    };
                    let mut handle = match Self::make_handle(ArchiveType::Zip, &path, file) {
                        Ok(handle) => handle,
                        Err(err) => {
                            *error.lock().unwrap() = Some(err);
                            return;
                        }
                    };
                    for (name, output) in chunk {
                        if error.lock().unwrap().is_some() {
                            return;
                        }
                        if let Some(parent) = output.parent() {
                            if let Err(err) = path_helper::create_dir_all(parent) {
                                *error.lock().unwrap() = Some(err);
                                return;
                            }
                        }
                        if let Err(err) = handle.extract_file(&name, &output, None) {
                            *error.lock().unwrap() = Some(err);
                            return;
                        }
                        let now = current.fetch_add(1, Ordering::SeqCst) + 1;
                        if let Some(gui) = gui {
                            gui.update(Some(name.clone()), now);
                        }
                    }
                });
            }
        });

        if let Some(err) = error.lock().unwrap().take() {
            return Err(err);
        }
        Ok(())
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
    /// Zip 格式下并行解压（每个线程持有独立读取句柄随机访问），其余格式流式解压保持顺序。
    /// 文件名非法时的 GUI 询问发生在任务收集阶段，保持串行。
    ///
    /// * `map` — 将每个条目映射为可选输出路径的闭包。
    /// * `gui` — 可选的进度回调（按每个已提取文件触发）。
    pub fn extract_where<F: FnMut(&ArchiveEntryInfo) -> Option<PathBuf>>(
        &self,
        mut map: F,
        gui: Option<&dyn IBaseArchiveGui>,
    ) -> CoreResult<()> {
        // 顺序阶段：计算目标路径，文件名非法时的 GUI 询问保持串行
        let mut tasks: Vec<(String, PathBuf)> = Vec::new();
        for entry in &self.entries {
            if entry.is_dir {
                continue;
            }
            if let Some(base) = map(entry) {
                let output = self.resolve_output_path(&entry.name, base, gui)?;
                tasks.push((entry.name.clone(), output));
            }
        }

        if let Some(gui) = gui {
            gui.start(tasks.len());
        }

        let result = if self.archive_type == ArchiveType::Zip {
            self.extract_tasks_parallel(&tasks, gui)
        } else {
            self.extract_tasks_sequential(&tasks, gui)
        };

        if let Some(gui) = gui {
            gui.done();
        }
        result
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
        // Zip / Tar 支持就地追加（复用已打开的文件句柄，不重新打开）；其余解压后重压缩
        match self.archive_type {
            ArchiveType::Zip | ArchiveType::Tar => {
                let paths: Vec<(PathBuf, PathBuf)> = files
                    .iter()
                    .map(|(src, dest)| (src.as_ref().to_path_buf(), dest.as_ref().to_path_buf()))
                    .collect();
                let mut handle = self.handle.lock().unwrap();
                handle.add_files(&paths)?;
                self.entries = handle.read_entries()?;
                Ok(())
            }
            _ => self.add_files_extract_recompress(files, gui),
        }
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
        self.refresh_after_modify()?;

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
        // Zip / Tar 支持就地追加；其余解压后重压缩
        match self.archive_type {
            ArchiveType::Zip | ArchiveType::Tar => {
                let mut handle = self.handle.lock().unwrap();
                handle.add_data(name, data)?;
                self.entries = handle.read_entries()?;
                Ok(())
            }
            _ => self.add_data_extract_recompress(name, data, gui),
        }
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
        self.refresh_after_modify()?;

        Ok(())
    }

    /// 在磁盘上的压缩包被**替换**后重新打开读取句柄并刷新条目。
    fn refresh_after_modify(&mut self) -> CoreResult<()> {
        let file = path_helper::open_read_write(&self.path)?;
        let mut handle = Self::make_handle(self.archive_type, &self.path, file)?;
        self.entries = handle.read_entries()?;
        *self.handle.lock().unwrap() = handle;
        Ok(())
    }
}
