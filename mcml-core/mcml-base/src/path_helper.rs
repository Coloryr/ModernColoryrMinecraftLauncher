/// 文件与路径处理

#[cfg(not(windows))]
use std::env;
#[cfg(not(windows))]
use std::process::{Command, Stdio};
use std::time::SystemTime;
#[cfg(not(windows))]
use std::time::UNIX_EPOCH;

use std::fs::{self, DirEntry};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use tokio::fs as tfs;

use mcml_names::i18_items::error_type::{CoreResult, ErrorData, ErrorType, FileSystemErrorData};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};

/// 提升权限
#[cfg(any(target_os = "linux", target_os = "macos"))]
pub fn chmod(path: &str) -> io::Result<()> {
    let mut child = Command::new("sh")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    if let Some(mut stdin) = child.stdin.take() {
        use std::io::Write;
        writeln!(stdin, "chmod a+x {}", path)?;
        writeln!(stdin, "exit")?;
    }

    child.wait()?;
    Ok(())
}

/// 提升Java文件夹权限
#[cfg(any(target_os = "linux", target_os = "macos"))]
pub fn per_java_chmod(path: &str) -> io::Result<()> {
    let mut child = Command::new("sh")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let info = Path::new(path);
    if let (Some(parent), Some(grandparent)) =
        (info.parent(), info.parent().and_then(|p| p.parent()))
    {
        if let Some(mut stdin) = child.stdin.take() {
            use std::io::Write;
            writeln!(stdin, "chmod a+x {}/*", parent.display())?;
            writeln!(stdin, "chmod a+x {}/lib/*", grandparent.display())?;
            writeln!(stdin, "exit")?;
        }
    }

    child.wait()?;
    Ok(())
}

/// 获取回收站路径
#[cfg(target_os = "linux")]
fn get_trash_files_path() -> PathBuf {
    let data_home = env::var("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
            PathBuf::from(&home).join(".local/share")
        });

    let trash_path = data_home.join("Trash/files");
    fs::create_dir_all(&trash_path).ok();
    trash_path
}

/// 获取回收站路径
#[cfg(target_os = "linux")]
fn get_trash_info_path() -> PathBuf {
    let data_home = env::var("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
            PathBuf::from(&home).join(".local/share")
        });

    let trash_info_path = data_home.join("Trash/info");
    fs::create_dir_all(&trash_info_path).ok();
    trash_info_path
}

/// 将文件夹挪到回收站
pub fn move_to_trash<P: AsRef<Path>>(dir: P) -> CoreResult<()> {
    // Check if the path exists
    if !dir.as_ref().exists() {
        return Ok(());
    }

    #[cfg(target_os = "windows")]
    {
        move_to_trash_windows(dir)
    }

    #[cfg(target_os = "linux")]
    {
        move_to_trash_linux(dir)
    }

    #[cfg(target_os = "macos")]
    {
        move_to_trash_macos(dir)
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    {
        Ok(false)
    }
}

/// 将文件夹挪到回收站
#[cfg(target_os = "linux")]
fn move_to_trash_linux(dir: &str) -> io::Result<bool> {
    use std::time::SystemTime;

    let trash_files_path = get_trash_files_path();
    let trash_info_path = get_trash_info_path();

    let file_name = Path::new(dir).file_name().unwrap_or_default();
    let mut dest_path = trash_files_path.join(file_name);
    let mut trash_info_file =
        trash_info_path.join(format!("{}.trashinfo", file_name.to_string_lossy()));

    let mut counter = 1;
    while dest_path.exists() {
        let name = Path::new(file_name);
        let name_without_ext = name.file_stem().unwrap_or_default();
        let ext = name.extension().unwrap_or_default();

        let new_name = if ext.is_empty() {
            format!("{}_{}", name_without_ext.to_string_lossy(), counter)
        } else {
            format!(
                "{}_{}.{}",
                name_without_ext.to_string_lossy(),
                counter,
                ext.to_string_lossy()
            )
        };

        dest_path = trash_files_path.join(&new_name);
        trash_info_file = trash_info_path.join(format!("{}.trashinfo", new_name));
        counter += 1;
    }

    // Generate trash info content
    let deletion_date = SystemTime::now();
    let datetime_str = format!(
        "{:?}",
        deletion_date.duration_since(UNIX_EPOCH).unwrap_or_default()
    );
    let trash_info_content = format!(
        "[Trash Info]\nPath={}\nDeletionDate={}\n",
        dir, datetime_str
    );

    fs::write(&trash_info_file, trash_info_content)?;

    // Move file or directory
    let path = Path::new(dir);
    if path.is_file() {
        fs::rename(path, &dest_path)?;
    } else if path.is_dir() {
        fs::rename(path, &dest_path)?;
    }

    Ok(true)
}

/// 将文件夹挪到回收站
#[cfg(target_os = "macos")]
fn move_to_trash_macos(dir: &str) -> io::Result<bool> {
    let escaped_dir = dir
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t");

    let apple_script = format!(
        "tell application \"Finder\" to delete POSIX file \"{}\"",
        escaped_dir
    );

    let output = Command::new("osascript")
        .args(["-e", &apple_script])
        .output()?;

    Ok(output.status.success())
}

/// 将文件夹挪到回收站
#[cfg(target_os = "windows")]
fn move_to_trash_windows<P: AsRef<Path>>(dir: P) -> CoreResult<()> {
    use std::iter::once;
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Foundation::HWND;
    use windows_sys::Win32::UI::Shell::FO_DELETE;
    use windows_sys::Win32::UI::Shell::FOF_ALLOWUNDO;
    use windows_sys::Win32::UI::Shell::FOF_NOCONFIRMATION;
    use windows_sys::Win32::UI::Shell::FOF_NOERRORUI;
    use windows_sys::Win32::UI::Shell::FOF_SILENT;
    use windows_sys::Win32::UI::Shell::SHFILEOPSTRUCTW;
    use windows_sys::Win32::UI::Shell::SHFileOperationW;

    // Convert string to Windows wide string (double null terminated)
    let wide_path: Vec<u16> = dir
        .as_ref()
        .as_os_str()
        .encode_wide()
        .chain(once(0))
        .chain(once(0))
        .collect();

    let mut operation = SHFILEOPSTRUCTW {
        hwnd: HWND::default(),
        wFunc: FO_DELETE,
        pFrom: wide_path.as_ptr(),
        pTo: std::ptr::null(),
        fFlags: (FOF_ALLOWUNDO | FOF_NOCONFIRMATION | FOF_SILENT | FOF_NOERRORUI) as u16,
        fAnyOperationsAborted: 0,
        hNameMappings: std::ptr::null_mut(),
        lpszProgressTitle: std::ptr::null(),
    };

    let result = unsafe { SHFileOperationW(&mut operation) };

    // Result codes:
    // 0 = Success
    // 0x71 = No error but user aborted (we treat as success since nothing was moved)
    // Other values indicate errors
    match result {
        0 => Ok(()),
        0x71 => Err(ErrorType::TaskCancel), // User cancelled
        _ => Err(ErrorType::FileSystemError(FileSystemErrorData {
            path: dir.as_ref().to_path_buf(),
            error: format!("SHFileOperationW failed with error code: 0x{:X}", result),
        })),
    }
}

/// 检查非法名字
/// - `name`: 需要检查的名字
pub fn file_has_invalid_chars(name: &str) -> bool {
    if name.is_empty() || name.chars().all(|c| c == '.') {
        return true;
    }

    if name.len() > 80 {
        return true;
    }

    name.contains(&['<', '>', ':', '"', '/', '\\', '|', '?', '*'][..])
}

/// 获取所有文件
/// - `path`: 需要计算的路径
pub fn get_all_files<P: AsRef<Path>>(local: P) -> Vec<PathBuf> {
    let mut files = Vec::new();

    if let Ok(entries) = fs::read_dir(local) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                files.push(path);
            } else if path.is_dir() {
                files.extend(get_all_files(&path));
            }
        }
    }

    files
}

/// 获取当前目录所有文件
/// - `path`: 需要获取的路径
pub fn get_files<P: AsRef<Path>>(path: P) -> Vec<PathBuf> {
    let mut files = Vec::new();

    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            if entry.path().is_file() {
                files.push(entry.path());
            }
        }
    }

    files
}

/// 获取文件夹下面最后写入的文件
/// - `path`: 获取的目录
pub fn get_last_written_file<P: AsRef<Path>>(path: P) -> CoreResult<Option<PathBuf>> {
    let entries = fs::read_dir(&path).map_err(|err| {
        ErrorType::FileSystemError(FileSystemErrorData {
            path: path.as_ref().to_path_buf(),
            error: err.to_string(),
        })
    })?;

    let mut files: Vec<(PathBuf, SystemTime)> = Vec::new();

    for entry in entries {
        let entry = entry.map_err(|err| {
            ErrorType::FileSystemError(FileSystemErrorData {
                path: path.as_ref().to_path_buf(),
                error: err.to_string(),
            })
        })?;
        let path = entry.path();

        if path.is_file() {
            let metadata = fs::metadata(&path).map_err(|err| {
                ErrorType::FileSystemError(FileSystemErrorData {
                    path: path.clone(),
                    error: err.to_string(),
                })
            })?;
            if let Ok(modified) = metadata.modified() {
                files.push((path, modified));
            }
        }
    }

    files.sort_by(|a, b| b.1.cmp(&a.1));

    Ok(files.first().map(|(path, _)| path.clone()))
}

/// 获取目录占用大小
/// - `path`: 需要获取的路径
pub fn get_folder_size<P: AsRef<Path>>(path: P) -> u64 {
    let mut size = 0;

    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                size += entry.metadata().map(|m| m.len()).unwrap_or(0);
            } else if path.is_dir() {
                size += get_folder_size(&path);
            }
        }
    }

    size
}

/// 获取当前目录所有目录
/// - `path`: 需要获取的路径
pub fn get_dirs<P: AsRef<Path>>(path: P) -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                dirs.push(path);
            }
        }
    }

    dirs
}

/// 复制文件
/// - `input`: 目标文件
/// - `output`: 输出文件
pub fn copy_file<P: AsRef<Path>>(input: P, output: P) -> CoreResult<()> {
    fs::copy(&input, output).map_err(|err| {
        ErrorType::FileSystemError(FileSystemErrorData {
            path: input.as_ref().to_path_buf(),
            error: err.to_string(),
        })
    })?;
    Ok(())
}

/// 异步复制文件
/// - `input`: 目标文件
/// - `output`: 输出文件
pub async fn copy_file_async<P: AsRef<Path>>(input: P, output: P) -> CoreResult<()> {
    tfs::copy(&input, output).await.map_err(|err| {
        ErrorType::FileSystemError(FileSystemErrorData {
            path: input.as_ref().to_path_buf(),
            error: err.to_string(),
        })
    })?;
    Ok(())
}

/// 搬运文件
/// - `input`: 目标文件
/// - `output`: 输出文件
pub fn move_file<P: AsRef<Path>>(input: P, output: P) -> CoreResult<()> {
    if let Some(parent) = output.as_ref().parent() {
        create_dir_all(parent)?;
    }

    match fs::rename(&input, &output) {
        Ok(_) => return Ok(()),
        Err(err) => {
            if err.kind() == io::ErrorKind::CrossesDevices {
                copy_file(&input, &output)?;
                delete(&input)?;
                return Ok(());
            }
            return Err(ErrorType::FileSystemError(FileSystemErrorData {
                path: input.as_ref().to_path_buf(),
                error: err.to_string(),
            }));
        }
    }
}

/// 异步搬运文件
/// - `input`: 目标文件
/// - `output`: 输出文件
pub async fn move_file_async<P: AsRef<Path>>(input: P, output: P) -> CoreResult<()> {
    if let Some(parent) = output.as_ref().parent() {
        create_dir_all(parent)?;
    }

    match tfs::rename(&input, &output).await {
        Ok(_) => return Ok(()),
        Err(err) => {
            if err.kind() == io::ErrorKind::CrossesDevices {
                copy_file(&input, &output)?;
                delete(&input)?;
                return Ok(());
            }
            return Err(ErrorType::FileSystemError(FileSystemErrorData {
                path: input.as_ref().to_path_buf(),
                error: err.to_string(),
            }));
        }
    }
}

/// 复制文件夹
/// - `input`: 目标目录
/// - `output`: 输出目录
pub fn copy_dir<P: AsRef<Path>>(input: P, output: P) -> CoreResult<()> {
    create_dir_all(&output)?;

    for entry in fs::read_dir(&input).map_err(|err| {
        ErrorType::FileSystemError(FileSystemErrorData {
            path: input.as_ref().to_path_buf(),
            error: err.to_string(),
        })
    })? {
        let entry = entry.map_err(|err| {
            ErrorType::FileReadError(ErrorData {
                error: err.to_string(),
            })
        })?;
        let path = entry.path();
        let dest_path = output.as_ref().join(entry.file_name());

        if path.is_dir() {
            copy_dir(&path, &dest_path)?;
        } else {
            copy_file(&path, &dest_path)?;
        }
    }

    Ok(())
}

/// 异步复制文件夹
/// - `input`: 目标目录
/// - `output`: 输出目录
pub async fn copy_dir_async<P: AsRef<Path>>(from: P, to: P) -> CoreResult<()> {
    create_dir_all(&to)?;

    let mut dir = tfs::read_dir(&from).await.map_err(|err| {
        ErrorType::FileSystemError(FileSystemErrorData {
            path: from.as_ref().to_path_buf(),
            error: err.to_string(),
        })
    })?;

    loop {
        let item = dir.next_entry().await.map_err(|err| {
            ErrorType::FileReadError(ErrorData {
                error: err.to_string(),
            })
        })?;

        if item.is_none() {
            break;
        }

        let entry = item.unwrap();
        let path = entry.path();
        let dest_path = to.as_ref().join(entry.file_name());

        if path.is_dir() {
            copy_dir_async(&path, &dest_path).await?;
        } else {
            copy_file_async(&path, &dest_path).await?;
        }
    }

    Ok(())
}

/// 查找文件
/// - `path`: 查找的目录
/// - `name`: 查找的文件名
pub fn search_file<P: AsRef<Path>>(path: P, name: &str) -> Option<PathBuf> {
    let files = get_all_files(path);
    files.into_iter().find(|f| f.file_name().unwrap() == name)
}

/// 读文件
/// - `file`: 文件路径
pub fn open_read<P: AsRef<Path>>(file: P) -> CoreResult<fs::File> {
    match fs::File::open(&file) {
        Ok(ok) => Ok(ok),
        Err(err) => Err(ErrorType::FileSystemError(FileSystemErrorData {
            path: file.as_ref().to_path_buf(),
            error: err.to_string(),
        })),
    }
}

/// 异步读文件
/// - `file`: 文件路径
pub async fn open_read_async<P: AsRef<Path>>(file: P) -> CoreResult<tfs::File> {
    match tfs::File::open(&file).await {
        Ok(ok) => Ok(ok),
        Err(err) => Err(ErrorType::FileSystemError(FileSystemErrorData {
            path: file.as_ref().to_path_buf(),
            error: err.to_string(),
        })),
    }
}

/// 写文件
/// - `file`: 文件路径
pub fn open_write<P: AsRef<Path>>(file: P) -> CoreResult<fs::File> {
    if let Some(parent) = file.as_ref().parent() {
        if let Err(err) = fs::create_dir_all(parent) {
            return Err(ErrorType::FileSystemError(FileSystemErrorData {
                path: file.as_ref().to_path_buf(),
                error: err.to_string(),
            }));
        }
    }

    Ok(fs::OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .open(&file)
        .map_err(|err| {
            ErrorType::FileSystemError(FileSystemErrorData {
                path: file.as_ref().to_path_buf(),
                error: err.to_string(),
            })
        })?)
}

/// 异步写文件
/// - `file`: 文件路径
pub async fn open_write_async<P: AsRef<Path>>(file: P) -> CoreResult<tfs::File> {
    if let Some(parent) = file.as_ref().parent() {
        create_dir_all(parent)?;
    }

    Ok(tfs::OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .open(&file)
        .await
        .map_err(|err| {
            ErrorType::FileSystemError(FileSystemErrorData {
                path: file.as_ref().to_path_buf(),
                error: err.to_string(),
            })
        })?)
}

/// 创建所有目录
/// - `path`: 目录
pub fn create_dir_all<P: AsRef<Path>>(path: P) -> CoreResult<()> {
    match fs::create_dir_all(&path) {
        Ok(_) => Ok(()),
        Err(err) => Err(ErrorType::FileSystemError(FileSystemErrorData {
            path: path.as_ref().to_path_buf(),
            error: err.to_string(),
        })),
    }
}

/// 继续写文件
/// - `file`: 文件路径
pub fn open_append<P: AsRef<Path>>(file: P) -> CoreResult<fs::File> {
    if let Some(parent) = file.as_ref().parent() {
        create_dir_all(parent)?;
    }
    Ok(fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&file)
        .map_err(|err| {
            ErrorType::FileSystemError(FileSystemErrorData {
                path: file.as_ref().to_path_buf(),
                error: err.to_string(),
            })
        })?)
}

/// 写文本
/// - `file`: 文件路径
/// - `text`: 文本内容
pub fn write_text<P: AsRef<Path>>(file: P, text: &str) -> CoreResult<()> {
    let mut stream = open_write(&file)?;
    stream.write_all(text.as_bytes()).map_err(|err| {
        ErrorType::FileSystemError(FileSystemErrorData {
            path: file.as_ref().to_path_buf(),
            error: err.to_string(),
        })
    })?;

    Ok(())
}

/// 异步写文本
/// - `file`: 文件路径
/// - `text`: 文本内容
pub async fn write_text_async<P: AsRef<Path>>(file: P, text: String) -> CoreResult<()> {
    let mut stream = open_write_async(&file).await?;

    stream.write_all(text.as_bytes()).await.map_err(|err| {
        ErrorType::FileSystemError(FileSystemErrorData {
            path: file.as_ref().to_path_buf(),
            error: err.to_string(),
        })
    })?;

    Ok(())
}

/// 读文本
/// - `file`: 文件路径
pub fn read_text<P: AsRef<Path>>(file: P) -> CoreResult<String> {
    let mut stream = open_read(&file)?;
    let mut content = String::new();
    stream.read_to_string(&mut content).map_err(|err| {
        ErrorType::FileSystemError(FileSystemErrorData {
            path: file.as_ref().to_path_buf(),
            error: err.to_string(),
        })
    })?;
    Ok(content)
}

/// 异步读文件
/// - `file`: 文件路径
pub async fn read_text_async<P: AsRef<Path>>(file: P) -> CoreResult<String> {
    let mut stream = open_read_async(&file).await?;
    let mut content = String::new();
    stream.read_to_string(&mut content).await.map_err(|err| {
        ErrorType::FileSystemError(FileSystemErrorData {
            path: file.as_ref().to_path_buf(),
            error: err.to_string(),
        })
    })?;
    Ok(content)
}

/// 读取byte数据
/// - `file`: 文件路径
pub fn read_byte<P: AsRef<Path>>(file: P) -> CoreResult<Vec<u8>> {
    let mut stream = open_read(&file)?;
    let mut buffer = Vec::new();
    stream.read_to_end(&mut buffer).map_err(|err| {
        ErrorType::FileSystemError(FileSystemErrorData {
            path: file.as_ref().to_path_buf(),
            error: err.to_string(),
        })
    })?;
    Ok(buffer)
}

/// 异步读取byte数据
/// - `file`: 文件路径
pub async fn read_byte_async<P: AsRef<Path>>(file: P) -> CoreResult<Vec<u8>> {
    let mut stream = open_read(&file)?;
    let mut buffer = Vec::new();
    stream.read_to_end(&mut buffer).map_err(|err| {
        ErrorType::FileSystemError(FileSystemErrorData {
            path: file.as_ref().to_path_buf(),
            error: err.to_string(),
        })
    })?;
    Ok(buffer)
}

/// 删除文件
/// - `file`: 文件路径
pub fn delete<P: AsRef<Path>>(file: P) -> CoreResult<()> {
    if file.as_ref().is_file() {
        fs::remove_file(&file).map_err(|err| {
            ErrorType::FileSystemError(FileSystemErrorData {
                path: file.as_ref().to_path_buf(),
                error: err.to_string(),
            })
        })?;
    }
    Ok(())
}

/// 异步删除文件
/// - `file`: 文件路径
pub async fn delete_async<P: AsRef<Path>>(file: P) -> CoreResult<()> {
    if file.as_ref().is_file() {
        tfs::remove_file(&file).await.map_err(|err| {
            ErrorType::FileSystemError(FileSystemErrorData {
                path: file.as_ref().to_path_buf(),
                error: err.to_string(),
            })
        })?;
    }
    Ok(())
}

/// 写文件
/// - `file`: 文件路径
/// - `data`: 数据
pub fn write_bytes<P: AsRef<Path>>(file: P, data: &[u8]) -> CoreResult<()> {
    let mut stream = open_write(&file)?;
    stream.write_all(data).map_err(|err| {
        ErrorType::FileSystemError(FileSystemErrorData {
            path: file.as_ref().to_path_buf(),
            error: err.to_string(),
        })
    })
}

/// 异步写文件
/// - `file`: 文件路径
/// - `data`: 数据
pub async fn write_bytes_async<P: AsRef<Path>>(file: P, data: &[u8]) -> CoreResult<()> {
    let mut stream = open_write_async(&file).await?;
    stream.write_all(data).await.map_err(|err| {
        ErrorType::FileSystemError(FileSystemErrorData {
            path: file.as_ref().to_path_buf(),
            error: err.to_string(),
        })
    })
}

/// 写文件
/// - `file`: 文件路径
/// - `reader`: 数据流
pub fn write_stream<P: AsRef<Path>, R: Read>(file: P, mut reader: R) -> CoreResult<()> {
    let mut stream = open_write(&file)?;
    io::copy(&mut reader, &mut stream).map_err(|err| {
        ErrorType::FileSystemError(FileSystemErrorData {
            path: file.as_ref().to_path_buf(),
            error: err.to_string(),
        })
    })?;
    Ok(())
}

/// 写文件
/// - `file`: 文件路径
/// - `reader`: 数据流
pub async fn write_stream_async<P: AsRef<Path>, R: AsyncRead + Unpin>(
    path: P,
    mut reader: R,
) -> CoreResult<()> {
    let mut stream = open_write_async(&path).await?;
    tokio::io::copy(&mut reader, &mut stream)
        .await
        .map_err(|err| {
            ErrorType::FileSystemError(FileSystemErrorData {
                path: path.as_ref().to_path_buf(),
                error: err.to_string(),
            })
        })?;
    Ok(())
}

/// 替换文件名非法字符
pub fn replace_file_name(name: &str) -> String {
    name.replace(|c: char| "<>:\"/\\|?*\0".contains(c), "_")
}

/// 替换文件名非法字符
pub fn replace_path_name(name: &str) -> String {
    #[cfg(not(windows))]
    let invalid_chars: Vec<char> = vec!['\0'];

    #[cfg(windows)]
    let invalid_chars: Vec<char> = vec!['<', '>', ':', '"', '/', '\\', '|', '?', '*', '\0'];

    name.chars()
        .map(|c| if invalid_chars.contains(&c) { '_' } else { c })
        .collect()
}
