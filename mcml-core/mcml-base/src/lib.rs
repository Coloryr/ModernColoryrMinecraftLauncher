//! 启动器基础库模块
//!
//! 本模块是启动器最底层的公共库，提供了所有其他模块共享的基础设施：
//!
//! # 核心功能
//!
//! - **系统信息检测** — 操作系统类型、CPU 架构、Linux 发行版识别
//! - **文件系统操作** — 文件读写、复制移动、权限提升、回收站操作
//! - **序列化工具** — JSON/TOML 的解析和序列化，自定义反序列化器
//! - **哈希计算** — MD5/SHA1/SHA256/SHA512 及 Base64 编解码
//! - **压缩包处理** — Zip/7z/Tar/TarGz/TarXz 的压缩和解压
//! - **事件系统** — 全局事件发布订阅（带参数/无参数）
//! - **进程管理** — 子进程启动（普通/管理员权限）、输出流捕获
//! - **字符串校验** — 数字格式、英文数字格式的正则校验
//!
//! # 子模块
//!
//! | 模块 | 用途 |
//! |------|------|
//! | [`path_helper`] | 文件和目录操作 |
//! | [`serialize_tools`] | JSON/TOML 序列化 |
//! | [`hash_helper`] | 哈希和 Base64 |
//! | [`archives`] | 压缩包处理 |
//! | [`events`] | 事件发布订阅 |
//! | [`process_utils`] | 进程管理 |
//! | [`inner_path`] | 内部数据存储路径 |
//! | [`file_item`] | 文件下载项定义 |

pub mod archives;
pub mod tools;
pub mod events;
pub mod file_item;
pub mod hash_helper;
pub mod inner_path;
pub mod path_helper;
pub mod process_utils;
pub mod serialize_tools;

use serde_repr::{Deserialize_repr, Serialize_repr};
use std::{
    fmt,
    path::{Path, PathBuf},
    sync::{LazyLock, OnceLock},
};

/// 操作系统类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Os {
    /// 未知操作系统
    None,
    /// Microsoft Windows
    Windows,
    /// Linux（通用发行版）
    Linux,
    /// Apple macOS
    MacOS,
    /// Alpine Linux（使用 musl libc）
    AlpineLinux,
    /// IBM AIX
    AIX,
    /// Oracle Solaris
    Solaris,
}

impl fmt::Display for Os {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Os::Windows => write!(f, "Windows"),
            Os::Linux => write!(f, "Linux"),
            Os::MacOS => write!(f, "MacOS"),
            Os::None => write!(f, "Unknown"),
            Os::AlpineLinux => write!(f, "Alpine Linux"),
            Os::AIX => write!(f, "AIX"),
            Os::Solaris => write!(f, "Solaris"),
        }
    }
}

/// CPU 架构枚举
///
/// 使用 `#[repr(u8)]` 支持高效序列化。
#[derive(Serialize_repr, Deserialize_repr, Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ArchEnum {
    /// x86 32 位
    X86,
    /// x86_64 64 位
    X86_64,
    /// ARM 32 位
    Arm,
    /// ARM 64 位 (AArch64)
    AArch64,
    /// 未知架构
    Unknown,
}

impl Default for ArchEnum {
    fn default() -> Self {
        ArchEnum::Unknown
    }
}

impl fmt::Display for ArchEnum {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ArchEnum::X86 => write!(f, "x86"),
            ArchEnum::X86_64 => write!(f, "x86_64"),
            ArchEnum::Arm => write!(f, "arm"),
            ArchEnum::AArch64 => write!(f, "aarch64"),
            ArchEnum::Unknown => write!(f, "unknown"),
        }
    }
}

/// 系统信息结构体
///
/// 在首次访问时通过 [`get_system_info()`] 惰性初始化并缓存。
#[derive(Debug, Clone)]
pub struct SystemInfo {
    /// 操作系统类型
    pub os: Os,
    /// CPU 架构
    pub system_arch: ArchEnum,
    /// 系统名称（完整描述，如 `"windows"`、`"linux"`）
    pub system_name: String,
    /// Linux 发行版标识（如 `"ubuntu"`、`"arch"`），非 Linux 为空字符串
    pub distribution: String,
    /// 格式化的系统描述字符串（如 `"Os:Windows Arch:x86_64"`）
    pub system: String,
    /// 是否为 ARM 处理器
    pub is_arm: bool,
    /// 是否为 64 位操作系统
    pub is_64_bit: bool,
}

/// 全局系统信息缓存（惰性初始化）
static SYSTEM_INFO: LazyLock<SystemInfo> = LazyLock::new(|| SystemInfo::new());

/// 获取系统信息（首次调用时自动检测并缓存）
pub fn get_system_info() -> SystemInfo {
    SYSTEM_INFO.clone()
}

/// 读取 Linux 发行版标识
///
/// 解析 `/etc/os-release` 中的 `ID=` 字段。
fn get_linux_distribution() -> String {
    // 读取 /etc/os-release
    let content = std::fs::read_to_string("/etc/os-release").ok();
    if content.is_some() {
        let content = content.unwrap();
        for line in content.lines() {
            if line.starts_with("ID=") {
                return line[3..].trim_matches('"').to_string();
            }
        }
    }

    String::new()
}

impl SystemInfo {
    /// 初始化并获取系统信息
    fn new() -> Self {
        let arch = std::env::consts::ARCH;
        let is_arm = arch.starts_with("arm") || arch.starts_with("aarch64");
        let is_64_bit = cfg!(target_pointer_width = "64");

        let system_arch = match (is_64_bit, is_arm) {
            (true, true) => ArchEnum::AArch64,
            (true, false) => ArchEnum::X86_64,
            (false, true) => ArchEnum::Arm,
            (false, false) => ArchEnum::X86,
        };

        let os = if cfg!(target_os = "windows") {
            Os::Windows
        } else if cfg!(target_os = "linux") {
            Os::Linux
        } else if cfg!(target_os = "macos") {
            Os::MacOS
        } else {
            Os::None
        };

        let distribution = if os == Os::Linux {
            get_linux_distribution()
        } else {
            String::new()
        };

        let system_name = std::env::consts::OS.to_string();
        let system = format!("Os:{} Arch:{}", os, system_arch);

        Self {
            os,
            system_arch,
            system_name,
            system,
            is_arm,
            distribution,
            is_64_bit,
        }
    }
}

impl fmt::Display for SystemInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.system)
    }
}

/// 程序运行根目录（全局单例）
static BASE_DIR: OnceLock<PathBuf> = OnceLock::new();

/// 初始化程序运行根目录
///
/// 应在程序启动时调用一次，设置后可通过 [`get_base_dir()`] 获取。
///
/// # 参数
///
/// - `dir`: 程序运行目录
pub fn init<P: AsRef<Path>>(dir: P) {
    BASE_DIR.get_or_init(|| dir.as_ref().to_path_buf());
}

/// 获取程序运行根目录
pub fn get_base_dir() -> PathBuf {
    BASE_DIR.get().unwrap().clone()
}
