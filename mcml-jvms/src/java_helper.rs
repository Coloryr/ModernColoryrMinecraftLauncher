use std::{collections::HashSet, path::{Path, PathBuf}, process::Stdio};

use crate::java_info_obj::{ArchEnum, JavaInfoObj};

use mcml_base::{Os, SystemInfo, get_system_info};
use tokio::process::Command;

/// 获取主版本号
fn get_major_version(version: &str) -> i32 {
    // Java 版本格式处理
    // 传统格式: 1.8.0_201 -> 8
    // 新格式: 11.0.2 -> 11, 17.0.1 -> 17

    if version.starts_with("1.") {
        // 传统版本: 1.8.0 -> 8
        version
            .split('.')
            .nth(1)
            .and_then(|v| v.parse::<i32>().ok())
            .unwrap_or(0)
    } else {
        // 新版本: 11.0.2 -> 11
        version
            .split('.')
            .next()
            .and_then(|v| v.parse::<i32>().ok())
            .unwrap_or(0)
    }
}

/// 异步版本的获取 Java 信息
pub async fn get_java_info(file: &PathBuf) -> Option<JavaInfoObj> {
    let path = file.clone();

    if !path.exists() || !path.is_file() {
        return None;
    }

    let working_dir = path
        .parent()
        .and_then(|parent| parent.parent())
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));

    let output = match Command::new(&path)
        .arg("-version")
        .current_dir(working_dir)
        .stderr(Stdio::piped())
        .stdout(Stdio::piped())
        .output()
        .await
    {
        Ok(output) => output,
        Err(_) => return None,
    };

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let combined = format!("{}\n{}", stderr, stdout);

    for line in combined.lines() {
        let line = line.trim();
        if line.contains(" version ") || line.contains("\"") {
            let parts: Vec<&str> = line.split_whitespace().collect();

            if parts.len() >= 3 {
                let java_type = parts[0].to_string();
                let version = parts[2].trim_matches('"').to_string();
                let is64 = combined.contains("64-Bit") || combined.contains("64-bit");

                let arch = if cfg!(target_arch = "aarch64") {
                    if is64 {
                        ArchEnum::Aarch64
                    } else {
                        ArchEnum::Arm
                    }
                } else {
                    if is64 {
                        ArchEnum::X86_64
                    } else {
                        ArchEnum::X86
                    }
                };

                let major_version = get_major_version(&version);

                return Some(JavaInfoObj {
                    name: String::new(),
                    path: path.to_string_lossy().to_string(),
                    version,
                    arch,
                    java_type,
                    major_version,
                });
            }
        }
    }

    None
}

pub async fn find_java() -> Option<Vec<JavaInfoObj>> {
    let system_info = get_system_info();
    let mut java_paths = HashSet::new();

    if system_info.os == Os::Windows {
        find_java_on_windows(&mut java_paths).await;
    } else if system_info.os == Os::Linux {
        find_java_on_linux(&system_info, &mut java_paths).await;
    } else if system_info.os == Os::MacOS {
        find_java_on_macos(&mut java_paths).await;
    }

    if java_paths.is_empty() {
        return None;
    }

    // 获取详细信息
    let mut java_list = Vec::new();
    for path in java_paths {
        if let Some(info) = get_java_info(&path).await {
            java_list.push(info);
        }
    }

    // 去重（基于路径）
    java_list.sort_by(|a, b| a.path.cmp(&b.path));
    java_list.dedup_by(|a, b| a.path == b.path);

    if java_list.is_empty() {
        None
    } else {
        Some(java_list)
    }
}

/// 执行命令并返回输出行列表
async fn get_list<I, S>(command: &str, args: I) -> Result<Vec<String>, std::io::Error>
where
    I: IntoIterator<Item = S>,
    S: AsRef<std::ffi::OsStr>,
{
    let output = Command::new(command).args(args).output().await?;

    if !output.status.success() {
        return Ok(Vec::new());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout
        .lines()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect())
}

/// Windows 注册表读取（需要 winreg crate）
#[cfg(target_os = "windows")]
fn get_java_from_registry(key_path: &str) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    use winreg::{RegKey, enums::HKEY_LOCAL_MACHINE};

    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let subkey = hklm.open_subkey(key_path)?;
    let mut paths = Vec::new();

    for name in subkey.enum_keys() {
        let key_name = name?;
        let key = subkey.open_subkey(&key_name)?;
        if let Ok(java_home) = key.get_value::<String, _>("JavaHome") {
            paths.push(PathBuf::from(java_home));
        }
    }

    Ok(paths)
}

#[cfg(not(target_os = "windows"))]
fn get_java_from_registry(_key_path: &str) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    Ok(Vec::new())
}

/// Windows 平台查找 Java
async fn find_java_on_windows(java_paths: &mut HashSet<PathBuf>) {
    // 1. 从 PATH 中查找
    if let Ok(paths) = get_list("where", &["javaw.exe"]).await {
        for path in paths {
            java_paths.insert(PathBuf::from(path));
        }
    }

    // 2. 从注册表查找
    let registry_paths = [
        (
            r"SOFTWARE\JavaSoft\Java Runtime Environment",
            "Oracle Java JRE",
        ),
        (r"SOFTWARE\JavaSoft\JDK", "Oracle Java JDK"),
        (r"SOFTWARE\Eclipse Adoptium\JDK", "Eclipse Adoptium"),
        (r"SOFTWARE\Azul Systems\Zulu", "Azul Zulu"),
    ];

    for (reg_path, _vendor) in registry_paths {
        if let Ok(install_paths) = get_java_from_registry(reg_path) {
            for install_path in install_paths {
                let java_exe = install_path.join("bin").join("javaw.exe");
                if java_exe.exists() {
                    java_paths.insert(java_exe);
                }
            }
        }
    }

    // 3. 常见安装路径
    let common_paths = [
        r"C:\Program Files\Java",
        r"C:\Program Files (x86)\Java",
        r"C:\Program Files\Eclipse Adoptium",
        r"C:\Program Files\Azul\Zulu",
    ];

    for base_path in common_paths {
        if let Ok(entries) = std::fs::read_dir(base_path) {
            for entry in entries.flatten() {
                let java_exe = entry.path().join("bin").join("javaw.exe");
                if java_exe.exists() {
                    java_paths.insert(java_exe);
                }
            }
        }
    }
}

/// 解析符号链接
fn resolve_symlink(path: &str) -> Result<PathBuf, std::io::Error> {
    let path = Path::new(path);
    if path.is_symlink() {
        std::fs::read_link(path)
    } else {
        Ok(path.to_path_buf())
    }
}

/// Linux 平台查找 Java
async fn find_java_on_linux(system_info: &SystemInfo, java_paths: &mut HashSet<PathBuf>) {
    if let Ok(paths) = get_list("which", &["java"]).await {
        for path in paths {
            if let Ok(resolved) = resolve_symlink(&path) {
                java_paths.insert(resolved);
            }
        }
    }

    match system_info.distribution.as_str() {
        "ubuntu" | "debian" => {
            if let Ok(paths) = get_list("update-alternatives", &["--list", "java"]).await {
                for path in paths {
                    if let Ok(resolved) = resolve_symlink(&path) {
                        java_paths.insert(resolved);
                    }
                }
            }
        }
        "arch" | "manjaro" => {
            find_java_on_arch(java_paths).await;
        }
        _ => {
            scan_common_java_dirs(java_paths).await;
        }
    }

    scan_common_java_dirs(java_paths).await;
}

/// Arch Linux 查找 Java
async fn find_java_on_arch(java_paths: &mut HashSet<PathBuf>) {
    // 查询已安装的 JDK/JRE 包
    if let Ok(packages) = get_list("pacman", &["-Qs", "jre|jdk"]).await {
        for package_line in packages {
            // 提取包名
            let parts: Vec<&str> = package_line.split_whitespace().collect();
            if parts.is_empty() {
                continue;
            }
            let package_name = parts[0];

            // 查询包文件列表
            if let Ok(files) = get_list("pacman", &["-Ql", package_name]).await {
                for file in files {
                    if file.ends_with("/bin/java") {
                        let parts: Vec<&str> = file.split_whitespace().collect();
                        if parts.len() >= 2 {
                            if let Ok(resolved) = resolve_symlink(parts[1]) {
                                java_paths.insert(resolved);
                            }
                        }
                    }
                }
            }
        }
    }
}

/// 扫描常见 Java 安装目录
async fn scan_common_java_dirs(java_paths: &mut HashSet<PathBuf>) {
    let common_dirs = ["/usr/lib/jvm", "/usr/java", "/opt/java", "/opt/jdk"];

    for dir in common_dirs {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let java_exe = entry.path().join("bin").join("java");
                if java_exe.exists() {
                    java_paths.insert(java_exe);
                }
            }
        }
    }
}

/// macOS 平台查找 Java
async fn find_java_on_macos(java_paths: &mut HashSet<PathBuf>) {
    // 使用 /usr/libexec/java_home
    if let Ok(output) = Command::new("/usr/libexec/java_home").arg("-V").output().await {
        let stderr = String::from_utf8_lossy(&output.stderr);
        for line in stderr.lines() {
            if line.contains("Java SE") || line.contains("JDK") {
                // 提取路径
                if let Some(path_start) = line.find('/') {
                    let path = &line[path_start..];
                    if let Some(path_end) = path.find(".jdk") {
                        let java_home = &path[..path_end + 4];
                        let java_exe = Path::new(java_home).join("bin").join("java");
                        if java_exe.exists() {
                            java_paths.insert(java_exe);
                        }
                    }
                }
            }
        }
    }

    // 扫描 /Library/Java/JavaVirtualMachines
    if let Ok(entries) = std::fs::read_dir("/Library/Java/JavaVirtualMachines") {
        for entry in entries.flatten() {
            let java_exe = entry
                .path()
                .join("Contents")
                .join("Home")
                .join("bin")
                .join("java");
            if java_exe.exists() {
                java_paths.insert(java_exe);
            }
        }
    }
}
