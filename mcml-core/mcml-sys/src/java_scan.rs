use std::{collections::HashSet, path::PathBuf, process::Command};

/// 执行命令并返回输出行列表
fn get_list<I, S>(command: &str, args: I) -> Result<Vec<String>, std::io::Error>
where
    I: IntoIterator<Item = S>,
    S: AsRef<std::ffi::OsStr>,
{
    let output = Command::new(command).args(args).output()?;

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

/// 搜索Java
///
/// - `java_paths`: 搜索的结果
#[cfg(target_os = "windows")]
pub fn find_java_inner(java_paths: &mut HashSet<PathBuf>) {
    use mcml_names::i18_items::error_type::CoreResult;
    use mcml_names::i18_items::error_type::DataNotFoundData;
    use mcml_names::i18_items::error_type::ErrorType;
    use winreg::{RegKey, enums::HKEY_LOCAL_MACHINE};

    /// Windows 注册表读取
    fn get_oracle_java_from_registry(key_path: &str) -> CoreResult<Vec<PathBuf>> {
        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
        let subkey = hklm.open_subkey(key_path).map_err(|_| {
            ErrorType::DataNotFound(DataNotFoundData::RegistryKey(key_path.to_string()))
        })?;
        let mut paths = Vec::new();

        for name in subkey.enum_keys() {
            let key_name = name.map_err(|_| {
                ErrorType::DataNotFound(DataNotFoundData::RegistryKey(key_path.to_string()))
            })?;
            let key = subkey
                .open_subkey(&key_name)
                .map_err(|_| ErrorType::DataNotFound(DataNotFoundData::RegistryKey(key_name)))?;
            if let Ok(java_home) = key.get_value::<String, _>("JavaHome") {
                paths.push(PathBuf::from(java_home));
            }
        }

        Ok(paths)
    }

    fn get_adoptium_java_from_registry(key_path: &str) -> CoreResult<Vec<PathBuf>> {
        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
        let subkey = hklm.open_subkey(key_path).map_err(|_| {
            ErrorType::DataNotFound(DataNotFoundData::RegistryKey(key_path.to_string()))
        })?;
        let mut paths = Vec::new();

        for name in subkey.enum_keys() {
            let key_name = name.map_err(|_| {
                ErrorType::DataNotFound(DataNotFoundData::RegistryKey(key_path.to_string()))
            })? + r"\hotspot\MSI";
            let key = subkey
                .open_subkey(&key_name)
                .map_err(|_| ErrorType::DataNotFound(DataNotFoundData::RegistryKey(key_name)))?;
            if let Ok(java_home) = key.get_value::<String, _>("Path") {
                paths.push(PathBuf::from(java_home));
            }
        }

        Ok(paths)
    }

    fn get_zulu_java_from_registry(key_path: &str) -> CoreResult<Vec<PathBuf>> {
        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
        let subkey = hklm.open_subkey(key_path).map_err(|_| {
            ErrorType::DataNotFound(DataNotFoundData::RegistryKey(key_path.to_string()))
        })?;
        let mut paths = Vec::new();

        for name in subkey.enum_keys() {
            let key_name = name.map_err(|_| {
                ErrorType::DataNotFound(DataNotFoundData::RegistryKey(key_path.to_string()))
            })?;
            let key = subkey
                .open_subkey(&key_name)
                .map_err(|_| ErrorType::DataNotFound(DataNotFoundData::RegistryKey(key_name)))?;
            if let Ok(java_home) = key.get_value::<String, _>("InstallationPath") {
                paths.push(PathBuf::from(java_home));
            }
        }

        Ok(paths)
    }

    if let Ok(paths) = get_list("where", &["javaw.exe"]) {
        for path in paths {
            java_paths.insert(PathBuf::from(path));
        }
    }

    if let Ok(install_paths) =
        get_oracle_java_from_registry(r"SOFTWARE\JavaSoft\Java Runtime Environment\")
    {
        for install_path in install_paths {
            let java_exe = install_path.join("bin").join("javaw.exe");
            if java_exe.exists() {
                java_paths.insert(java_exe);
            }
        }
    }
    if let Ok(install_paths) = get_oracle_java_from_registry(r"SOFTWARE\JavaSoft\JDK\") {
        for install_path in install_paths {
            let java_exe = install_path.join("bin").join("javaw.exe");
            if java_exe.exists() {
                java_paths.insert(java_exe);
            }
        }
    }
    if let Ok(install_paths) = get_adoptium_java_from_registry(r"SOFTWARE\Eclipse Adoptium\JDK\") {
        for install_path in install_paths {
            let java_exe = install_path.join("bin").join("javaw.exe");
            if java_exe.exists() {
                java_paths.insert(java_exe);
            }
        }
    }
    if let Ok(install_paths) = get_zulu_java_from_registry(r"SOFTWARE\Azul Systems\Zulu\") {
        for install_path in install_paths {
            let java_exe = install_path.join("bin").join("javaw.exe");
            if java_exe.exists() {
                java_paths.insert(java_exe);
            }
        }
    }

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

/// 搜索Java
///
/// - `java_paths`: 搜索的结果
#[cfg(target_os = "linux")]
pub(crate) fn find_java_inner(java_paths: &mut HashSet<PathBuf>) {
    fn resolve_symlink(path: &str) -> Result<PathBuf, std::io::Error> {
        let path = Path::new(path);
        if path.is_symlink() {
            std::fs::read_link(path)
        } else {
            Ok(path.to_path_buf())
        }
    }

    /// Arch Linux 查找 Java
    fn find_java_on_arch(java_paths: &mut HashSet<PathBuf>) {
        // 查询已安装的 JDK/JRE 包
        if let Ok(packages) = get_list("pacman", &["-Qs", "jre|jdk"]) {
            for package_line in packages {
                // 提取包名
                let parts: Vec<&str> = package_line.split_whitespace().collect();
                if parts.is_empty() {
                    continue;
                }
                let package_name = parts[0];

                // 查询包文件列表
                if let Ok(files) = get_list("pacman", &["-Ql", package_name]) {
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
    fn scan_common_java_dirs(java_paths: &mut HashSet<PathBuf>) {
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

    if let Ok(paths) = get_list("which", &["java"]) {
        for path in paths {
            if let Ok(resolved) = resolve_symlink(&path) {
                java_paths.insert(resolved);
            }
        }
    }

    let system_info = get_system_info();

    match system_info.distribution.as_str() {
        "ubuntu" | "debian" => {
            if let Ok(paths) = get_list("update-alternatives", &["--list", "java"]) {
                for path in paths {
                    if let Ok(resolved) = resolve_symlink(&path) {
                        java_paths.insert(resolved);
                    }
                }
            }
        }
        "arch" | "manjaro" => {
            find_java_on_arch(java_paths);
        }
        _ => {
            scan_common_java_dirs(java_paths);
        }
    }

    scan_common_java_dirs(java_paths);
}

/// 搜索Java
///
/// - `java_paths`: 搜索的结果
#[cfg(target_os = "macos")]
pub(crate) fn find_java(java_paths: &mut HashSet<PathBuf>) {
    // 使用 /usr/libexec/java_home
    if let Ok(output) = Command::new("/usr/libexec/java_home").arg("-V").output() {
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
