//! Foojay Disco API
//!
//! foojay.io 聚合了各 OpenJDK 发行版（Temurin / Zulu / Semeru / Corretto 等）
//! 的下载元数据，`/disco/v3.0/parameters` 给出全部参数的合法取值，
//! `/disco/v3.0/packages` 按条件筛选可下载的 Java 包。
//! 下载走 `links.pkg_download_redirect`，由服务端 302 到发行方 CDN 直链。

use std::sync::OnceLock;

use mml_names::i18_items::error_type::{CoreResult, DataNotFoundData, ErrorType};
use serde::{Deserialize, Serialize};

use crate::{WORK_CLIENT, urls};

/// 可选项缓存（首次查询后复用）
static OPTIONS: OnceLock<FoojayOptionsObj> = OnceLock::new();

/// 四个下拉框的候选列表
#[derive(Debug, Clone)]
pub struct FoojayOptionsObj {
    /// 发行类型（jdk / jre）
    pub types: Vec<String>,
    /// 主版本号（降序）
    pub majors: Vec<String>,
    /// 系统（windows / linux / macos …）
    pub systems: Vec<String>,
    /// 架构（x64 / x86 / aarch64 …）
    pub archs: Vec<String>,
}

/// `/parameters` 响应包装
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct FoojayParametersResultObj {
    pub result: Vec<FoojayParametersObj>,
    pub message: String,
}

impl Default for FoojayParametersResultObj {
    fn default() -> Self {
        Self {
            result: Default::default(),
            message: Default::default(),
        }
    }
}

/// `/parameters` 响应中的单条参数集合
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct FoojayParametersObj {
    /// `/packages` 各参数的合法取值
    pub packages: FoojayPackagesParamsObj,
}

impl Default for FoojayParametersObj {
    fn default() -> Self {
        Self {
            packages: Default::default(),
        }
    }
}

/// `/packages` 各参数的合法取值（逗号分隔字符串）
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct FoojayPackagesParamsObj {
    pub architecture: String,
    pub major_version: String,
    pub operating_system: String,
    pub package_type: String,
}

impl Default for FoojayPackagesParamsObj {
    fn default() -> Self {
        Self {
            architecture: Default::default(),
            major_version: Default::default(),
            operating_system: Default::default(),
            package_type: Default::default(),
        }
    }
}

/// `/packages` 响应包装
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct FoojayResultObj {
    pub result: Vec<FoojayObj>,
    pub message: String,
}

impl Default for FoojayResultObj {
    fn default() -> Self {
        Self {
            result: Default::default(),
            message: Default::default(),
        }
    }
}

/// Foojay 单个 Java 包
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct FoojayObj {
    /// 包 ID（用于拼下载重定向地址）
    pub id: String,
    /// 打包格式（msi / exe / zip / tar.gz / pkg …）
    pub archive_type: String,
    /// 发行版（zulu / temurin / semeru …）
    pub distribution: String,
    /// 主版本号
    pub major_version: i32,
    /// OpenJDK 版本号（如 21+35）
    pub java_version: String,
    /// 发行方自己的版本号（如 21.28.85）
    pub distribution_version: String,
    /// 发布状态（ga / ea）
    pub release_status: String,
    /// 目标系统
    pub operating_system: String,
    /// 目标架构
    pub architecture: String,
    /// 包类型（jdk / jre）
    pub package_type: String,
    /// 是否内置 JavaFX
    pub javafx_bundled: bool,
    /// 是否可经 API 直接下载
    pub directly_downloadable: bool,
    /// 包文件名
    pub filename: String,
    /// 包大小（字节）
    pub size: u64,
    /// 相关链接
    pub links: FoojayLinksObj,
}

impl Default for FoojayObj {
    fn default() -> Self {
        Self {
            id: Default::default(),
            archive_type: Default::default(),
            distribution: Default::default(),
            major_version: Default::default(),
            java_version: Default::default(),
            distribution_version: Default::default(),
            release_status: Default::default(),
            operating_system: Default::default(),
            architecture: Default::default(),
            package_type: Default::default(),
            javafx_bundled: Default::default(),
            directly_downloadable: Default::default(),
            filename: Default::default(),
            size: Default::default(),
            links: Default::default(),
        }
    }
}

/// 包的相关链接
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct FoojayLinksObj {
    /// 包详情页
    pub pkg_info_uri: String,
    /// 下载重定向地址（302 到发行方 CDN 直链）
    pub pkg_download_redirect: String,
}

impl Default for FoojayLinksObj {
    fn default() -> Self {
        Self {
            pkg_info_uri: Default::default(),
            pkg_download_redirect: Default::default(),
        }
    }
}

/// 拆分逗号分隔的取值串，去空项
fn split_values(data: &str) -> Vec<String> {
    data.split(',')
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
        .collect()
}

/// 获取四个下拉框的候选列表（发行类型 / 主版本 / 系统 / 架构）
///
/// 数据来自 `/parameters`，主版本按数值降序，架构排序去重。
/// 首次查询后缓存。
///
/// # 返回值
///
/// 返回各下拉框的候选值列表
pub async fn get_options() -> CoreResult<FoojayOptionsObj> {
    if let Some(obj) = OPTIONS.get() {
        return Ok(obj.clone());
    }

    let url = format!("{}parameters", urls::FOOJAY);

    let res = WORK_CLIENT
        .get()
        .unwrap()
        .get_json::<FoojayParametersResultObj>(&url)
        .await?;

    let Some(item) = res.result.first() else {
        return Err(ErrorType::DataNotFound(DataNotFoundData::Info));
    };

    let packages = &item.packages;

    let types = split_values(&packages.package_type);

    let mut majors: Vec<u32> = split_values(&packages.major_version)
        .iter()
        .filter_map(|item| item.parse().ok())
        .collect();
    majors.sort_unstable_by(|a, b| b.cmp(a));
    let majors = majors.iter().map(|item| item.to_string()).collect();

    let systems = split_values(&packages.operating_system);

    let mut archs = split_values(&packages.architecture);
    archs.sort();
    archs.dedup();

    let obj = FoojayOptionsObj {
        types,
        majors,
        systems,
        archs,
    };

    let _ = OPTIONS.set(obj.clone());

    Ok(obj)
}

/// 获取Java包列表
///
/// 固定带 `release_status=ga` 与 `directly_downloadable=true`，只返回可直接下载的正式版。
///
/// # 参数
///
/// - `major`: Java主版本
/// - `os`: 系统（API 的 operating_system 取值，如 windows / macos），空串表示不限
/// - `arch`: 架构（如 x64 / aarch64），空串表示不限
/// - `package_type`: 包类型（jdk / jre），空串表示不限
///
/// # 返回值
///
/// 返回符合条件的 Java 包列表
pub async fn get_java_list(
    major: u32,
    os: &str,
    arch: &str,
    package_type: &str,
) -> CoreResult<Vec<FoojayObj>> {
    let mut url = format!("{}packages?version={major}", urls::FOOJAY);

    if !os.is_empty() {
        url += &format!("&operating_system={os}");
    }
    if !arch.is_empty() {
        url += &format!("&architecture={arch}");
    }
    if !package_type.is_empty() {
        url += &format!("&package_type={package_type}");
    }

    url += "&release_status=ga&directly_downloadable=true";

    let res = WORK_CLIENT
        .get()
        .unwrap()
        .get_json::<FoojayResultObj>(&url)
        .await?;

    Ok(res.result)
}
