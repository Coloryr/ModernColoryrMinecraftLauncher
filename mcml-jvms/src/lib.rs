use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    sync::{Arc, LazyLock, Mutex, OnceLock, RwLock},
};

use mcml_base::{ArchEnum, events::EventHandler};
use mcml_config::config_obj::JvmConfigObj;
use mcml_names::names;

pub mod java_helper;

/// Java信息
pub struct JavaInfoObj {
    /// 名字
    pub name: String,
    /// 名字
    pub path: PathBuf,
    /// 版本号
    pub version: String,
    /// 主版本号
    pub major_version: i32,
    /// Java类型
    pub java_type: String,
    /// 进制
    pub arch: ArchEnum,
}

static DIR: OnceLock<PathBuf> = OnceLock::new();
static JAVA_DIR: OnceLock<PathBuf> = OnceLock::new();

static JVMS: LazyLock<RwLock<HashMap<String, Arc<JavaInfoObj>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

static JVM_CHANGE_EVENT: LazyLock<EventHandler> = LazyLock::new(|| EventHandler::new());

pub fn add_jvm_change<F>(handler: F) -> u64
where
    F: Fn() + Send + Sync + 'static,
{
    JVM_CHANGE_EVENT.add_handler(handler)
}

pub fn remove_jvm_change(id: u64) {
    JVM_CHANGE_EVENT.remove_handle(id);
}

pub(crate) fn invoke_jvm_change() {
    JVM_CHANGE_EVENT.emit();
}

/// 初始化
/// - `dir`:  运行的路径
pub fn init<P: AsRef<Path>>(dir: P) {
    let dir = DIR.get_or_init(|| dir.as_ref().to_path_buf());
    let dir = JAVA_DIR.get_or_init(|| dir.join(names::JAVA_DIR));
    if !dir.is_dir() || !dir.exists() {
        fs::create_dir(dir).unwrap();
    }

    let config = mcml_config::read_config();
    let config = &config.java_list;

    add_list(config);
}

/// 获取Java信息
/// - `name`: 名字
pub fn get_java_info(key: &str) -> Option<Arc<JavaInfoObj>> {
    let list = JVMS.read().ok()?;
    let item = list.get(key)?;
    Some(item.clone())
}

/// 删除Java
/// - `name`: 名字
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

/// 删除所有Java
pub fn remove_all() {
    let mut list = JVMS.write().unwrap();

    list.clear();
    let mut config = mcml_config::write_config();
    config.java_list.clear();
    mcml_config::save();
}

/// 添加Java
/// - `name`: 名字
/// - `file`: 路径
pub fn add_item(name: String, file: String) -> Option<String> {
    let dir = DIR.get().unwrap();
    let local = if file.starts_with(dir.to_str().unwrap()) {
        String::from(&file[dir.to_str().unwrap().len()..])
    } else {
        file
    };

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

/// 添加到列表
/// - `list`: 列表
fn add_list(list: &Vec<JvmConfigObj>) {
    let dir = DIR.get().unwrap().clone();
    let list_cloned = list.clone();

    let mut list1 = JVMS.write().unwrap();
    list1.clear();

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

/// 查找对应主版本的Java
/// - `version`: 主版本
/// - `over`: 是否允许获取高版本
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

    filtered.sort_by(|a, b| b.major_version.cmp(&a.major_version));

    filtered.first().map(|&info| info.clone())
}

/// 获取所有Java
pub fn get_all_java() -> Vec<Arc<JavaInfoObj>> {
    let read = JVMS.read().unwrap();
    let mut vec = Vec::new();

    for (_, value) in read.iter() {
        vec.push(value.clone());
    }

    vec
}

/// 从注册表或者标准路径中查找java列表
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

/// 扫描Java并添加到列表中
pub fn scan_java() {
    if let Some(list) = find_java() {
        let mut list1 = JVMS.write().unwrap();

        for (_, item) in list.into_iter().enumerate() {
            list1.insert(item.name.clone(), Arc::new(item));
        }
    }
}
