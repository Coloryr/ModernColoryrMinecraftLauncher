//! Java 运行时管理模块
//!
//! 本模块负责管理启动器中配置的 Java 运行时环境（JRE/JDK）。
//!
//! # 核心功能
//!
//! - **自动扫描** — 从系统注册表（Windows）、标准路径（Linux/macOS）搜索已安装的 Java
//! - **手动添加** — 用户可手动指定 Java 可执行文件路径
//! - **版本匹配** — 根据 Minecraft 版本自动选择兼容的 Java 版本
//! - **架构匹配** — 自动过滤与系统架构一致的 Java（x86_64 / aarch64）
//! - **变更通知** — 通过事件回调通知 UI 层 Java 列表已变更
//!
//! # 数据结构
//!
//! [`JavaInfoObj`] 包含 Java 的名称、路径、版本、主版本号、类型和架构信息。

use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    sync::{Arc, LazyLock, OnceLock, RwLock},
};

use mcml_base::{ArchEnum, events::EventHandler};
use mcml_config::config_obj::JvmConfigObj;
use mcml_names::names;

pub mod java_helper;

/// Java 运行时信息
pub struct JavaInfoObj {
    /// Java 显示名称（如 "OpenJDK-17.0.1-x86_64"）
    pub name: String,
    /// Java 可执行文件的完整路径
    pub path: PathBuf,
    /// Java 完整版本号字符串（如 "17.0.1"）
    pub version: String,
    /// Java 主版本号（如 8、11、17、21）
    pub major_version: i32,
    /// Java 发行版类型（如 "OpenJDK"、"Oracle"）
    pub java_type: String,
    /// CPU 架构
    pub arch: ArchEnum,
}

/// Java 运行时存放目录（`{运行目录}/java/`）
static JAVA_DIR: OnceLock<PathBuf> = OnceLock::new();

/// 全局 Java 运行时列表（按名称索引）
static JVMS: LazyLock<RwLock<HashMap<String, Arc<JavaInfoObj>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

/// Java 列表变更事件（通知 UI 刷新）
static JVM_CHANGE_EVENT: LazyLock<EventHandler> = LazyLock::new(|| EventHandler::new());

/// 注册 Java 列表变更回调
///
/// # 返回值
///
/// 返回回调 ID，可用于 [`remove_jvm_change`] 取消注册
pub fn add_jvm_change<F>(handler: F) -> u64
where
    F: Fn() + Send + Sync + 'static,
{
    JVM_CHANGE_EVENT.add_handler(handler)
}

/// 移除 Java 列表变更回调
///
/// # 参数
///
/// - `id`: 回调注册时返回的 ID
pub fn remove_jvm_change(id: u64) {
    JVM_CHANGE_EVENT.remove_handle(id);
}

/// 触发 Java 列表变更事件（内部使用）
pub(crate) fn invoke_jvm_change() {
    JVM_CHANGE_EVENT.emit();
}

/// 初始化 Java 运行时管理
///
/// 创建 Java 存放目录，加载配置中保存的 Java 列表。
///
/// # 参数
///
/// - `dir`: 程序运行根目录
pub fn init<P: AsRef<Path>>(dir: P) {
    let dir = JAVA_DIR.get_or_init(|| dir.as_ref().join(names::JAVA_DIR));
    if !dir.is_dir() || !dir.exists() {
        fs::create_dir(dir).unwrap();
    }

    let config = mcml_config::read_config();
    let config = &config.java_list;

    add_list(config);
}

/// 根据名称获取 Java 信息
///
/// # 参数
///
/// - `key`: Java 名称
///
/// # 返回值
///
/// 找到则返回 `Arc<JavaInfoObj>` 的克隆，未找到返回 `None`
pub fn get_java_info(key: &str) -> Option<Arc<JavaInfoObj>> {
    let list = JVMS.read().ok()?;
    let item = list.get(key)?;
    Some(item.clone())
}

/// 删除指定名称的 Java
///
/// 同时从内存列表和配置文件中的 Java 列表中移除。
///
/// # 参数
///
/// - `name`: Java 名称
pub fn remove(name: &str) {
    let mut list = JVMS.write().unwrap();
    if list.remove(name).is_some() {
        invoke_jvm_change();
    }

    let mut config = mcml_config::write_config();
    let javas = &mut config.java_list;
    let mut find = false;
    javas.retain(|item| {
        find = true;
        item.name.eq_ignore_ascii_case(name)
    });

    if find {
        mcml_config::save();
    }
}

/// 删除所有 Java 并保存配置
pub fn remove_all() {
    let mut list = JVMS.write().unwrap();

    list.clear();
    let mut config = mcml_config::write_config();
    config.java_list.clear();
    mcml_config::save();
}

/// 添加一个 Java 运行时
///
/// 测试 Java 可执行文件是否有效，有效则加入列表并保存配置。
///
/// # 参数
///
/// - `name`: Java 显示名称
/// - `file`: Java 可执行文件路径
///
/// # 返回值
///
/// 添加成功返回 `Some(name)`，无效的 Java 返回 `None`
pub fn add_item(name: String, file: String) -> Option<String> {
    let dir = mcml_base::get_base_dir();
    let local = if file.starts_with(dir.to_str().unwrap()) {
        String::from(&file[dir.to_str().unwrap().len()..])
    } else {
        file
    };

    // 先移除同名旧条目
    remove(&name);

    let path = if local.starts_with(names::JAVA_DIR) {
        dir.join(&local)
    } else {
        Path::new(&local).to_path_buf()
    };

    let info = java_helper::test_java(&path);
    match info {
        None => None,
        Some(info) => {
            let mut list = JVMS.write().unwrap();
            list.insert(name.clone(), Arc::new(info));

            invoke_jvm_change();

            let mut config = mcml_config::write_config();
            let javas = &mut config.java_list;
            javas.push(JvmConfigObj {
                name: name.clone(),
                local: local.clone(),
            });
            mcml_config::save();

            Some(name.clone())
        }
    }
}

/// 从配置列表批量测试并添加 Java
///
/// # 参数
///
/// - `list`: 配置文件中保存的 Java 列表
fn add_list(list: &Vec<JvmConfigObj>) {
    let dir = mcml_base::get_base_dir();
    let list_cloned = list.clone();

    let mut list1 = JVMS.write().unwrap();
    list1.clear();

    // 在异步任务中逐个测试 Java
    tokio::task::spawn(async move {
        let mut empty: bool = false;
        for item in list_cloned.iter() {
            let path = item.local.clone();
            let path = if path.starts_with(names::JAVA_DIR) {
                dir.join(path)
            } else {
                PathBuf::from(path)
            };

            let info = java_helper::test_java(&path);
            let mut list1 = JVMS.write().unwrap();
            list1.remove(&item.name);

            if info.is_none() {
                // Java 无效，保留占位条目
                list1.insert(
                    item.name.clone(),
                    Arc::new(JavaInfoObj {
                        name: item.name.clone(),
                        path,
                        version: String::new(),
                        major_version: -1,
                        java_type: String::new(),
                        arch: ArchEnum::Unknown,
                    }),
                );
            } else {
                let mut info = info.unwrap();
                info.name = item.name.clone();
                list1.insert(item.name.clone(), Arc::new(info));
            }

            empty = false;
        }

        if empty {
            scan_java();
        }
    });
}

/// 根据版本需求查找匹配的 Java
///
/// # 参数
///
/// - `version`: 所需的主版本号（如 17、21）
/// - `over`: `true` 允许返回更高版本的 Java，`false` 要求精确匹配
///
/// # 返回值
///
/// 找到则返回匹配的 Java 信息，未找到返回 `None`
pub fn get_java(version: i32, over: bool) -> Option<Arc<JavaInfoObj>> {
    let list = JVMS.read().ok()?;
    let system_arch = mcml_base::get_system_info().system_arch;

    let mut filtered: Vec<&Arc<JavaInfoObj>> = list
        .iter()
        .filter(|item| {
            if over {
                item.1.major_version >= version
            } else {
                item.1.major_version == version
            }
        })
        .filter(|item| item.1.arch == system_arch)
        .map(|item| item.1)
        .collect();

    // 按版本号降序排列（优先选择最新版本）
    filtered.sort_by(|a, b| b.major_version.cmp(&a.major_version));

    filtered.first().map(|&info| info.clone())
}

/// 获取所有已配置的 Java 运行时列表
pub fn get_all_java() -> Vec<Arc<JavaInfoObj>> {
    let read = JVMS.read().unwrap();
    let mut vec = Vec::new();

    for (_, value) in read.iter() {
        vec.push(value.clone());
    }

    vec
}

/// 从系统注册表或标准路径中搜索已安装的 Java
///
/// 返回去重后的 Java 列表，按路径排序。
fn find_java() -> Option<Vec<JavaInfoObj>> {
    let mut java_paths = HashSet::new();

    java_helper::find_java_inner(&mut java_paths);

    if java_paths.is_empty() {
        return None;
    }

    // 获取详细信息
    let mut java_list = Vec::new();
    for path in java_paths {
        if let Some(info) = java_helper::test_java(&path) {
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

/// 扫描系统中已安装的 Java 并添加到列表
///
/// 此函数执行系统级的 Java 搜索（注册表、常见路径等）。
pub fn scan_java() {
    if let Some(list) = find_java() {
        let mut list1 = JVMS.write().unwrap();

        for (_, item) in list.into_iter().enumerate() {
            list1.insert(item.name.clone(), Arc::new(item));
        }
    }
}
