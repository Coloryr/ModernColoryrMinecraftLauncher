//! Java 下载窗口：Java 运行时下载的规格 + IPC 命令
//!
//! 搜索源（Adoptium / Zulu / OpenJ9 / Foojay）选定后，经 `java_download_get_options`
//! 取该源的四个下拉框候选（发行类型 / 主版本 / 系统 / 架构）筛选拟下载的 Java，
//! 选好后经 `java_download_start` 触发下载，完成后经 `java-change` 事件
//! 通知各窗口刷新 Java 列表（与手动添加同一条刷新链路）。

use std::{
    collections::HashMap,
    sync::{LazyLock, RwLock},
};

use mml_base::file_item::{FileHash, FileItemObj, LaterRun};
use mml_net::adoptium_api;
use uuid::Uuid;

use crate::dtos::{JavaDownloadItemDto, JavaDownloadOptionsDto, JavaTypes};

static JAVA_ITEMS: LazyLock<RwLock<HashMap<Uuid, FileItemObj>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

/// 获取全部搜索源（Adoptium / Zulu / OpenJ9 / Foojay）
#[tauri::command]
pub fn java_download_get_types() -> Vec<JavaTypes> {
    JavaTypes::all()
}

/// 获取指定搜索源的 Java 下载可选项（四个下拉框的候选列表）
///
/// 各源候选来源不同：
/// - Foojay：`/parameters` 接口，四组候选齐全
/// - Adoptium：主版本动态拉取，类型 / 系统 / 架构为其支持的固定集合
/// - Zulu：从包列表提取（`bundle_type` 区分 jdk / jre，主版本取 `java_version` 首段）
/// - OpenJ9：页面抓取结果自带 arch / os / 主版本候选
///
/// # 参数
///
/// - `source`: 搜索源
#[tauri::command]
pub async fn java_download_get_options(
    source: JavaTypes,
) -> Result<JavaDownloadOptionsDto, String> {
    match source {
        JavaTypes::Foojay => {
            let obj = mml_net::foojay_api::get_options()
                .await
                .map_err(|e| e.to_string())?;
            Ok(JavaDownloadOptionsDto {
                types: obj.types,
                majors: obj
                    .majors
                    .iter()
                    .filter_map(|item| item.parse().ok())
                    .collect(),
                systems: obj.systems,
                archs: obj.archs,
            })
        }
        JavaTypes::Adoptium => {
            let versions = mml_net::adoptium_api::get_java_version()
                .await
                .map_err(|e| e.to_string())?;
            let mut majors: Vec<u32> = versions
                .iter()
                .filter_map(|item| item.parse().ok())
                .collect();
            majors.sort_unstable_by(|a, b| b.cmp(a));
            Ok(JavaDownloadOptionsDto {
                types: ["jdk", "jre"].iter().map(|s| s.to_string()).collect(),
                majors,
                // 与 adoptium_api::get_os 取值一致
                systems: ["windows", "linux", "alpine-linux", "mac", "aix", "solaris"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                // Adoptium 官方支持的架构集合
                archs: [
                    "x64", "x32", "aarch64", "arm", "ppc64le", "ppc64", "s390x", "riscv64",
                ]
                .iter()
                .map(|s| s.to_string())
                .collect(),
            })
        }
        JavaTypes::Zulu => {
            let list = mml_net::zulu_api::get_java_list()
                .await
                .map_err(|e| e.to_string())?;

            let mut types: Vec<String> = list
                .iter()
                .map(|item| item.bundle_type.clone())
                .filter(|item| !item.is_empty())
                .collect();
            types.sort();
            types.dedup();

            let mut majors: Vec<u32> = list
                .iter()
                .filter_map(|item| item.java_version.first().map(|v| *v as u32))
                .collect();
            majors.sort_unstable();
            majors.dedup();
            majors.reverse();

            let mut systems: Vec<String> = list
                .iter()
                .map(|item| item.os.clone())
                .filter(|item| !item.is_empty())
                .collect();
            systems.sort();
            systems.dedup();

            let mut archs: Vec<String> = list
                .iter()
                .map(|item| zulu_arch(&item.arch, &item.hw_bitness))
                .collect();
            archs.sort();
            archs.dedup();

            Ok(JavaDownloadOptionsDto {
                types,
                majors,
                systems,
                archs,
            })
        }
        JavaTypes::OpenJ9 => {
            // openj9_api 内部持有 rquickjs 运行时，future 非 Send，
            // 不能直接跨 Tauri 的异步命令 await；丢到阻塞线程上用
            // block_on 单线程驱动（阻塞线程不是异步 worker，block_on 合法）
            let obj = tauri::async_runtime::spawn_blocking(move || {
                tauri::async_runtime::block_on(mml_net::openj9_api::get_java_list())
            })
            .await
            .map_err(|e| e.to_string())?
            .map_err(|e| e.to_string())?;
            let mut majors: Vec<u32> = obj
                .main_version
                .iter()
                .filter_map(|item| item.parse().ok())
                .collect();
            majors.sort_unstable_by(|a, b| b.cmp(a));
            // openj9 列表自带 "" 占位项（原页面的 any 选项），这里过滤掉
            Ok(JavaDownloadOptionsDto {
                types: ["jdk", "jre"].iter().map(|s| s.to_string()).collect(),
                majors,
                systems: obj.os,
                archs: obj.arch,
            })
        }
    }
}

/// Zulu 的 `arch` + `hw_bitness` 组合转架构名（与 ArchEnum 命名一致）
///
/// `java_download_start` 里做反向映射即可还原回 arch + bitness 过滤条件
fn zulu_arch(arch: &str, bitness: &str) -> String {
    match (arch, bitness) {
        ("x86", "64") => "x86_64".to_string(),
        ("arm", "64") => "aarch64".to_string(),
        ("ppc", "64") => "ppc64".to_string(),
        (arch, _) => arch.to_string(),
    }
}

/// 下载选定的 Java（发行类型 / 主版本 / 系统 / 架构组合），
/// 解包注册到 Java 列表后返回；下载 / 解包进度可另发事件
#[tauri::command]
pub async fn java_download_start(
    java_type: String,
    major: u32,
    system: String,
    arch: String,
) -> Result<(), String> {
    let _ = (java_type, major, system, arch);
    todo!("java_download_start 待实现")
}

/// 获取按当前筛选条件匹配的 Java 包列表（窗口下方的文本列表）
///
/// # 参数
///
/// - `source`: 搜索源
/// - `java_type` / `major` / `system` / `arch`: 四个下拉框的当前值
#[tauri::command]
pub async fn java_download_get_list(
    source: JavaTypes,
    java_type: String,
    major: u32,
    system: String,
    arch: String,
) -> Result<Vec<JavaDownloadItemDto>, String> {
    {
        let mut locker = JAVA_ITEMS.write().unwrap();
        locker.clear();
    }
    let mut out = Vec::new();
    match source {
        JavaTypes::Adoptium => {
            let list = adoptium_api::get_java_list(major, &system)
                .await
                .map_err(|err| err.to_string())?;

            let mut locker = JAVA_ITEMS.write().unwrap();

            for item in list {
                if item.binary.image_type == "debugimage" {
                    continue;
                }
                if item.binary.architecture != arch || item.binary.image_type != java_type {
                    continue;
                }
                let uuid = Uuid::new_v4();
                let name = format!("{}_{}", item.binary.scm_ref, item.binary.image_type);
                locker.insert(
                    uuid.clone(),
                    FileItemObj {
                        name: name.clone(),
                        file: Default::default(),
                        url: item.binary.package.link,
                        hash: FileHash::Sha256(item.binary.package.checksum),
                        later: LaterRun::None,
                    },
                );
                
                out.push(JavaDownloadItemDto {
                    uuid: uuid.to_string(),
                    name,
                    java_version: item.version.openjdk_version,
                    filename: item.binary.name,
                    size: item.binary.package.size as u64,
                });
            }
        }
        JavaTypes::Zulu => todo!(),
        JavaTypes::OpenJ9 => todo!(),
        JavaTypes::Foojay => todo!(),
    }

    Ok(out)
}
