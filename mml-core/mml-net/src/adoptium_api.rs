//! Adoptium (Eclipse Temurin) API
//!
//! 提供从 Adoptium 官方 API 获取可用 Java 版本列表和下载链接的功能。
//! Adoptium 是 Eclipse 基金会维护的 OpenJDK 发行版（前身为 AdoptOpenJDK）。

use std::sync::OnceLock;

use mml_names::i18_items::error_type::ErrorType;
use mml_sys::Os;
use serde::{Deserialize, Serialize};

use crate::{WORK_CLIENT, urls::ADOPTIUM_URL};

/// 可用 Java 主版本缓存
static JAVA_VERSION: OnceLock<Vec<String>> = OnceLock::new();

/// 下载包信息
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct PackageObj {
    /// 包校验值
    pub checksum: String,
    /// 下载链接
    pub link: String,
    /// 包文件名
    pub name: String,
    /// 包大小（字节）
    pub size: i64,
}

impl Default for PackageObj {
    fn default() -> Self {
        Self {
            checksum: String::new(),
            link: String::new(),
            name: String::new(),
            size: 0,
        }
    }
}

/// 构建产物二进制信息
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct BinaryObj {
    /// 目标 CPU 架构
    pub architecture: String,
    /// 镜像类型（jdk / jre）
    pub image_type: String,
    /// 目标操作系统
    pub os: String,
    /// 下载包信息
    pub package: PackageObj,
    /// 源码版本引用
    pub scm_ref: String,
}

impl Default for BinaryObj {
    fn default() -> Self {
        Self {
            architecture: Default::default(),
            image_type: Default::default(),
            os: Default::default(),
            package: Default::default(),
            scm_ref: Default::default(),
        }
    }
}

/// OpenJDK 版本信息
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct AdoptiumVersionObj {
    /// OpenJDK 完整版本号
    pub openjdk_version: String,
}

impl Default for AdoptiumVersionObj {
    fn default() -> Self {
        Self {
            openjdk_version: Default::default(),
        }
    }
}

/// Adoptium 单个 Java 发行项
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct AdoptiumObj {
    /// 二进制产物信息
    pub binary: BinaryObj,
    /// OpenJDK 版本信息
    pub version: AdoptiumVersionObj,
}

impl Default for AdoptiumObj {
    fn default() -> Self {
        Self {
            binary: Default::default(),
            version: Default::default(),
        }
    }
}

/// Adoptium 可用版本信息
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct AdoptiumJavaVersionObj {
    /// 可用的 Java 主版本列表
    pub available_releases: Vec<i32>,
}

impl Default for AdoptiumJavaVersionObj {
    fn default() -> Self {
        Self {
            available_releases: Vec::new(),
        }
    }
}

/// 获取API系统对应的字符串
///
/// - `ostype`: 系统类型
///
/// # 返回值
///
/// 返回 Adoptium API 的 os 参数取值；未知系统返回空字符串
fn get_os(ostype: Os) -> &'static str {
    match ostype {
        Os::Windows => "windows",
        Os::Linux => "linux",
        Os::AlpineLinux => "alpine-linux",
        Os::MacOS => "mac",
        Os::AIX => "aix",
        Os::Solaris => "solaris",
        _ => "",
    }
}

/// 获取支持的Java主版本
///
/// # 返回值
///
/// 返回可用的 Java 主版本列表（首次查询后缓存）
pub async fn get_java_version() -> Result<Vec<String>, ErrorType> {
    if let Some(list) = JAVA_VERSION.get() {
        return Ok(list.to_vec());
    }

    let url = ADOPTIUM_URL.to_string() + "v3/info/available_releases";

    let res = WORK_CLIENT
        .get()
        .unwrap()
        .get_json::<AdoptiumJavaVersionObj>(&url)
        .await?;

    let mut list = Vec::new();
    for item in res.available_releases.iter() {
        list.push(item.to_string());
    }

    Ok(list)
}

/// 获取Java文件列表
///
/// - `version`: Java主版本
/// - `os`: 系统类型
///
/// # 返回值
///
/// 返回该版本各构建产物的下载信息列表
pub async fn get_java_list(version: String, os: Os) -> Result<Vec<AdoptiumObj>, ErrorType> {
    let mut url = String::from(ADOPTIUM_URL);
    if os == Os::None {
        url += &format!("v3/assets/latest/{}/hotspot", version);
    } else {
        url += &format!("v3/assets/latest/{}/hotspot?os={}", version, get_os(os));
    }

    let res = WORK_CLIENT
        .get()
        .unwrap()
        .get_json::<Vec<AdoptiumObj>>(&url)
        .await?;

    Ok(res)
}
