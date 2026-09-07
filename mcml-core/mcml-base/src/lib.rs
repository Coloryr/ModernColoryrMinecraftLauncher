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
//! | [`inner_path`] | 内部数据存储路径 |
//! | [`file_item`] | 文件下载项定义 |

pub mod archives;
pub mod events;
pub mod file_item;
pub mod hash_helper;
pub mod inner_path;
pub mod serialize_tools;
pub mod tools;
pub mod version_parse;

use std::{
    path::{Path, PathBuf},
    sync::OnceLock,
};

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

#[cfg(test)]
mod tests {
    use super::*;

    /// init / get_base_dir 全局根目录
    ///
    /// 注意：BASE_DIR 为进程级全局单例（OnceLock + get_or_init），
    /// 首次调用的参数生效，重复 init 不会 panic 也不会覆盖。
    #[test]
    fn test_init_and_get_base_dir() {
        let dir = std::env::temp_dir().join(format!(
            "mcml_base_init_test_{}_{}",
            std::process::id(),
            uuid::Uuid::new_v4().simple()
        ));
        init(&dir);
        assert_eq!(get_base_dir(), dir);

        // 重复调用不应 panic，且保留第一次的值
        init("/mcml_base_should_not_override");
        assert_eq!(get_base_dir(), dir);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
