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

/// 下载选定的 Java 包并解包注册
///
/// `java_download_get_list` 缓存的下载信息（`JAVA_ITEMS`）按 uuid 取出，
/// 压缩包经下载器落到下载目录，随后 `mml_jvms::unzip_java` 解包识别并
/// 注册进 Java 列表（`java-change` 事件由 mml_jvms 回调自动发出）。
///
/// # 参数
///
/// - `uuid`: 列表项 uuid（`java_download_get_list` 返回的 `uuid` 字段）
#[tauri::command]
pub async fn java_download_start(uuid: String) -> Result<(), String> {
    let Ok(uuid) = Uuid::parse_str(&uuid) else {
        return Err("err.javaItemNotFound".to_string());
    };

    let item = {
        let locker = JAVA_ITEMS.read().unwrap();
        locker.get(&uuid).cloned()
    };
    let Some(item) = item else {
        return Err("err.javaItemNotFound".to_string());
    };

    // 下载压缩包（进度在下载窗口可见）
    let ok = mml_downloader::start_download_task(vec![item.clone()]).await;
    if !ok {
        return Err("err.downloadFailed".to_string());
    }

    // 解包识别并注册（解包无需进度回调，传 None）
    let archive = item.file.clone();
    mml_jvms::unzip_java(Some(item.name.clone()), &archive, None)
        .await
        .map_err(|err| err.to_string())?;

    Ok(())
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
                        file: mml_downloader::get_download_path().join(&item.binary.name),
                        url: item.binary.package.link,
                        hash: FileHash::Sha256(item.binary.package.checksum),
                        later: LaterRun::None,
                    },
                );

                out.push(JavaDownloadItemDto {
                    uuid: uuid.to_string(),
                    name,
                    java_version: item.version.semver,
                    filename: item.binary.name,
                    size: item.binary.package.size as u64,
                });
            }
        }
        JavaTypes::Zulu => {
            let list = mml_net::zulu_api::get_java_list()
                .await
                .map_err(|err| err.to_string())?;

            // 先过滤再按 zulu 版本倒序（新版本在前）
            let mut list: Vec<_> = list
                .into_iter()
                .filter(|item| {
                    item.bundle_type == java_type
                        && item.java_version.first().copied() == Some(major as i32)
                        && item.os == system
                        && zulu_arch(&item.arch, &item.hw_bitness) == arch
                        && unpackable(&item.url)
                })
                .collect();
            list.sort_by(|a, b| b.zulu_version.cmp(&a.zulu_version));

            let mut locker = JAVA_ITEMS.write().unwrap();

            for item in list {
                let uuid = Uuid::new_v4();
                let name = format!(
                    "{}_{}",
                    item.zulu_version
                        .iter()
                        .map(|v| v.to_string())
                        .collect::<Vec<_>>()
                        .join("."),
                    item.bundle_type
                );
                // Zulu 列表没有文件名，从下载地址取最后一段
                let filename = file_name_of(&item.url).to_string();
                locker.insert(
                    uuid.clone(),
                    FileItemObj {
                        name: name.clone(),
                        file: mml_downloader::get_download_path().join(&filename),
                        url: item.url,
                        hash: FileHash::Sha256(item.sha256_hash),
                        later: LaterRun::None,
                    },
                );

                out.push(JavaDownloadItemDto {
                    uuid: uuid.to_string(),
                    name,
                    java_version: item
                        .java_version
                        .iter()
                        .map(|v| v.to_string())
                        .collect::<Vec<_>>()
                        .join("."),
                    filename,
                    // Zulu 列表接口不带包大小
                    size: 0,
                });
            }
        }
        JavaTypes::OpenJ9 => {
            // openj9_api 内部持有 rquickjs 运行时，future 非 Send，
            // 不能直接跨 Tauri 的异步命令 await；丢到阻塞线程上用
            // block_on 单线程驱动（阻塞线程不是异步 worker，block_on 合法）
            let obj = tauri::async_runtime::spawn_blocking(move || {
                tauri::async_runtime::block_on(mml_net::openj9_api::get_java_list())
            })
            .await
            .map_err(|err| err.to_string())?
            .map_err(|err| err.to_string())?;

            let mut locker = JAVA_ITEMS.write().unwrap();

            for item in obj.download {
                if item.version != major as i32 || item.os != system || item.arch != arch {
                    continue;
                }
                // 每个条目 jdk / jre 各一个直链，按所选类型取对应侧
                let side = if java_type == "jre" {
                    item.jre
                } else {
                    item.jdk
                };
                if side.opt1.download_link.is_empty() {
                    continue;
                }
                let url = side.opt1.download_link;
                if !unpackable(&url) {
                    continue;
                }
                let uuid = Uuid::new_v4();
                let name = format!("{}_{}", item.name, java_type);
                let filename = file_name_of(&url).to_string();
                locker.insert(
                    uuid.clone(),
                    FileItemObj {
                        name: name.clone(),
                        file: mml_downloader::get_download_path().join(&filename),
                        url,
                        hash: FileHash::Sha256(side.opt1.checksum),
                        later: LaterRun::None,
                    },
                );

                out.push(JavaDownloadItemDto {
                    uuid: uuid.to_string(),
                    name,
                    java_version: item.name,
                    filename,
                    // OpenJ9 列表不带包大小
                    size: 0,
                });
            }
        }
        JavaTypes::Foojay => {
            let mut list = mml_net::foojay_api::get_java_list(major, &system, &arch, &java_type)
                .await
                .map_err(|err| err.to_string())?;

            // API 返回顺序是乱的，按 OpenJDK 版本号倒序（新版本在前）
            list.sort_by(|a, b| version_key(&b.java_version).cmp(&version_key(&a.java_version)));

            let mut locker = JAVA_ITEMS.write().unwrap();

            for item in list {
                if !unpackable(&item.filename) {
                    continue;
                }
                let uuid = Uuid::new_v4();
                let name = format!("{}_{}", item.distribution_version, item.package_type);
                locker.insert(
                    uuid.clone(),
                    FileItemObj {
                        name: name.clone(),
                        file: mml_downloader::get_download_path().join(&item.filename),
                        // 服务端 302 到发行方 CDN 直链的重定向地址
                        url: item.links.pkg_download_redirect,
                        // Foojay /packages 不带校验值
                        hash: FileHash::None,
                        later: LaterRun::None,
                    },
                );

                out.push(JavaDownloadItemDto {
                    uuid: uuid.to_string(),
                    name,
                    java_version: item.java_version,
                    filename: item.filename,
                    // API 对未知大小返回 -1，归零让前端不显示大小
                    size: item.size.max(0) as u64,
                });
            }
        }
    }

    Ok(out)
}

/// 从下载地址取文件名（最后一段路径）
fn file_name_of(url: &str) -> &str {
    url.rsplit('/').next().unwrap_or(url)
}

/// 只保留能被 `mml_jvms::unzip_java` 解包的压缩包
///
/// msi / exe / pkg / dmg 是安装器 / 磁盘镜像，无法解包注册，不应进列表
fn unpackable(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    !(lower.ends_with(".msi")
        || lower.ends_with(".exe")
        || lower.ends_with(".pkg")
        || lower.ends_with(".dmg"))
}

/// 版本号拆成数值段（如 "21.0.5+11" → [21, 0, 5, 11]），供逐段比较排序
///
/// 非数值段跳过；字典序比较会把 "9" 排在 "21" 后面，数值逐段比较不会
fn version_key(version: &str) -> Vec<u32> {
    version
        .split(['.', '+', '_'])
        .filter_map(|item| item.parse().ok())
        .collect()
}
