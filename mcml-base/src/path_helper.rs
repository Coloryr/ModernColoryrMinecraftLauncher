/// 文件与路径处理
use std::env;
use std::fs;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::UNIX_EPOCH;

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
pub fn move_to_trash(dir: &str) -> io::Result<bool> {
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
fn move_to_trash_windows(dir: &str) -> io::Result<bool> {
    use std::ffi::OsStr;
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

    // Check if the path exists
    let path = std::path::Path::new(dir);
    if !path.exists() {
        return Ok(false);
    }

    // Convert string to Windows wide string (double null terminated)
    let wide_path: Vec<u16> = OsStr::new(dir)
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
        0 => Ok(true),
        0x71 => Ok(false), // User cancelled
        _ => {
            eprintln!("SHFileOperationW failed with error code: 0x{:X}", result);
            Ok(false)
        }
    }
}

/// 检查非法名字
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
pub fn get_all_files(local: &PathBuf) -> Vec<PathBuf> {
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
pub fn get_files(local: &PathBuf) -> Vec<PathBuf> {
    let mut files = Vec::new();

    if let Ok(entries) = fs::read_dir(local) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                files.push(path);
            }
        }
    }

    files
}

/// 获取目录占用大小
pub fn get_folder_size(folder_path: &PathBuf) -> u64 {
    let mut size = 0;

    if let Ok(entries) = fs::read_dir(folder_path) {
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
pub fn get_dirs(local: &PathBuf) -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    if let Ok(entries) = fs::read_dir(local) {
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
pub fn copy_file(input: &PathBuf, output: &PathBuf) -> io::Result<()> {
    fs::copy(input, output)?;
    Ok(())
}

/// 搬运文件
pub fn move_file(input: &PathBuf, output: &PathBuf) -> io::Result<()> {
    copy_file(input, output)?;
    delete(input)?;
    Ok(())
}

/// 复制文件夹
fn copy_dir_recursive(from: &Path, to: &Path) -> io::Result<()> {
    fs::create_dir_all(to)?;

    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let path = entry.path();
        let dest_path = to.join(entry.file_name());

        if path.is_dir() {
            copy_dir_recursive(&path, &dest_path)?;
        } else {
            fs::copy(&path, &dest_path)?;
        }
    }

    Ok(())
}

/// 复制文件夹
pub async fn copy_dir_async(dir: &PathBuf, dir1: &PathBuf) -> io::Result<()> {
    let dir = Path::new(dir).to_path_buf();
    let dir1 = Path::new(dir1).to_path_buf();
    tokio::task::spawn_blocking(move || copy_dir_recursive(&dir, &dir1))
        .await
        .unwrap()
}

/// 查找文件
pub fn get_file(local: &PathBuf, name: &str) -> Option<PathBuf> {
    let files = get_all_files(local);
    files.into_iter().find(|f| f.file_name().unwrap() == name)
}

/// 读文件
pub fn open_read(local: &PathBuf) -> Option<fs::File> {
    let path = Path::new(local);
    if path.exists() {
        fs::File::open(path).ok()
    } else {
        None
    }
}

/// 写文件
pub fn open_write(local: &PathBuf) -> io::Result<fs::File> {
    let path = Path::new(local);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .open(path)
}

/// 继续写文件
pub fn open_append(local: &PathBuf) -> io::Result<fs::File> {
    let path = Path::new(local);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = fs::OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .open(path)?;
    file.seek(SeekFrom::End(0))?;
    Ok(file)
}

/// 写文本
pub fn write_text(local: &PathBuf, text: &str) -> io::Result<()> {
    let path = Path::new(local);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, text)
}

/// 异步写文本
pub async fn write_text_async(local: &PathBuf, text: String) -> io::Result<()> {
    let local = local.clone();
    tokio::task::spawn_blocking(move || write_text(&local, &text))
        .await
        .unwrap()
}

/// 读文本
pub fn read_text(local: &PathBuf) -> Option<String> {
    let mut file = open_read(local)?;
    let mut content = String::new();
    file.read_to_string(&mut content).ok()?;
    Some(content)
}

/// 删除文件
pub fn delete(local: &PathBuf) -> io::Result<()> {
    let path = Path::new(local);
    if path.is_file() {
        fs::remove_file(path)?;
    }
    Ok(())
}

/// 写文件
pub fn write_bytes(local: &PathBuf, data: &[u8]) -> io::Result<()> {
    let mut file = open_write(local)?;
    file.write_all(data)
}

/// 写文件
pub fn write_bytes_from_stream(local: &PathBuf, mut data: impl Read) -> io::Result<()> {
    let mut file = open_write(local)?;
    io::copy(&mut data, &mut file)?;
    Ok(())
}

/// 写文件
pub async fn write_bytes_async(local: &PathBuf, data: Vec<u8>) -> io::Result<()> {
    let local = local.clone();
    tokio::task::spawn_blocking(move || write_bytes(&local, &data))
        .await
        .unwrap()
}

/// 替换文件名非法字符
pub fn replace_file_name(name: Option<&str>) -> String {
    let name = name.unwrap_or_default();
    let invalid_chars: Vec<char> = vec!['<', '>', ':', '"', '/', '\\', '|', '?', '*', '\0'];

    name.chars()
        .map(|c| if invalid_chars.contains(&c) { '_' } else { c })
        .collect()
}

/// 替换文件名非法字符
pub fn replace_path_name(name: Option<&str>) -> String {
    let name = name.unwrap_or_default();
    let invalid_chars: Vec<char> = vec!['\0'];

    #[cfg(windows)]
    let invalid_chars: Vec<char> = vec!['<', '>', ':', '"', '/', '\\', '|', '?', '*', '\0'];

    name.chars()
        .map(|c| if invalid_chars.contains(&c) { '_' } else { c })
        .collect()
}

/// 读取byte数据
pub fn read_byte(local: &PathBuf) -> Option<Vec<u8>> {
    let mut file = open_read(local)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).ok()?;
    Some(buffer)
}
