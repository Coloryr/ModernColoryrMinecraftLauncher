pub mod path_helper;
pub mod inner_path;

use std::{env, fmt};

/// 操作系统类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Os {
    None,
    Windows,
    Linux,
    MacOS,
    AlpineLinux,
    AIX,
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

/// 系统架构枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchEnum {
    X86,
    X86_64,
    Arm,
    AArch64,
    Unknown,
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
#[derive(Debug)]
pub struct SystemInfo {
    /// 当前语言/区域设置
    pub culture_info: String,
    /// 操作系统类型
    pub os: Os,
    /// 系统架构
    pub system_arch: ArchEnum,
    /// 系统名称（完整描述）
    pub system_name: String,
    /// 格式化的系统字符串
    pub system: String,
    /// 是否为 ARM 处理器
    pub is_arm: bool,
    /// 是否为 64 位操作系统
    pub is_64_bit: bool,
}

impl SystemInfo {
    /// 初始化并获取系统信息
    pub fn init() -> Self {
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

        let system_name = std::env::consts::OS.to_string();
        let culture_info = Self::get_current_locale();

        let system = format!("Os:{} Arch:{}", os, system_arch);

        Self {
            culture_info,
            os,
            system_arch,
            system_name,
            system,
            is_arm,
            is_64_bit,
        }
    }

    /// 获取当前区域设置/语言
    fn get_current_locale() -> String {
        // 方法1：检查环境变量
        if let Ok(lang) = env::var("LANG") {
            return lang;
        }
        if let Ok(lang) = env::var("LC_ALL") {
            return lang;
        }
        if let Ok(lang) = env::var("LANGUAGE") {
            return lang;
        }

        // 默认值
        "zh_CN".to_string()
    }

    /// 刷新系统信息（如果需要动态更新）
    pub fn refresh(&mut self) {
        *self = Self::init();
    }
}

impl fmt::Display for SystemInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.system)
    }
}

lazy_static::lazy_static! {
    static ref SYSTEM_INFO: SystemInfo = SystemInfo::init();
}

pub fn get_system_info() -> &'static SystemInfo {
    &SYSTEM_INFO
}
