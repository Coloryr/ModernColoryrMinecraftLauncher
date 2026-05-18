use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::{Arc, OnceLock, RwLock},
};

use mcml_base::{ArchEnum, events::core_jvm_change, get_system_info};
use mcml_config::config_obj::JvmConfigObj;
use mcml_names::names;

use crate::java_info_obj::JavaInfoObj;

pub mod java_helper;
pub mod java_info_obj;

static DIR: OnceLock<PathBuf> = OnceLock::new();
static JAVA_DIR: OnceLock<PathBuf> = OnceLock::new();

static JVMS: OnceLock<RwLock<HashMap<String, Arc<JavaInfoObj>>>> = OnceLock::new();

/// 初始化
/// - `dir`:  运行的路径
pub fn init(dir: &PathBuf) {
    let dir = DIR.get_or_init(|| dir.clone());
    let dir = JAVA_DIR.get_or_init(|| dir.join(names::NAME_JAVA_DIR));
    if !dir.is_dir() || !dir.exists() {
        fs::create_dir(dir).unwrap();
    }

    let config = mcml_config::CONFIG.get().unwrap().read().unwrap();
    let config = &config.java_list;

    add_list(config);
}

/// 获取Java信息
/// - `name`: 名字
pub fn get_info(name: &String) -> Option<Arc<JavaInfoObj>> {
    let list = JVMS.get()?.read().ok()?;
    let item = list.get(name)?;
    Some(item.clone())
}

/// 删除Java
/// - `name`: 名字
pub fn remove(name: &String) {
    let mut list = JVMS.get().unwrap().write().unwrap();
    if list.remove(name).is_some() {
        core_jvm_change::invoke_jvm_change();
    }

    let mut config = mcml_config::CONFIG.get().unwrap().write().unwrap();
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
    let mut list = JVMS.get().unwrap().write().unwrap();

    list.clear();
    let mut config = mcml_config::CONFIG.get().unwrap().write().unwrap();
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

    let path = if local.starts_with(names::NAME_JAVA_DIR) {
        dir.join(&local)
    } else {
        Path::new(&local).to_path_buf()
    };

    let info = java_helper::get_java_info(&path);
    match info {
        None => None,
        Some(info) => {
            let mut list = JVMS.get().unwrap().write().unwrap();
            list.insert(name.clone(), Arc::new(info));

            core_jvm_change::invoke_jvm_change();

            let mut config = mcml_config::CONFIG.get().unwrap().write().unwrap();
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

    let mut list1 = JVMS.get().unwrap().write().unwrap();
    list1.clear();

    tokio::task::spawn(async move {
        let mut empty: bool = false;
        for item in list_cloned.iter() {
            let path = item.local.clone();
            let path = if path.starts_with(names::NAME_JAVA_DIR) {
                dir.join(path)
            } else {
                PathBuf::from(path)
            };

            let info = java_helper::get_java_info(&path);
            let mut list1 = JVMS.get().unwrap().write().unwrap();
            list1.remove(&item.name);

            if info.is_none() {
                list1.insert(
                    item.name.clone(),
                    Arc::new(JavaInfoObj {
                        name: item.name.clone(),
                        path: item.local.clone(),
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
            let list2 = java_helper::find_java();
            if list2.is_some() {
                let list2 = list2.unwrap();
                let mut list1 = JVMS.get().unwrap().write().unwrap();

                for (_, item) in list2.into_iter().enumerate() {
                    list1.insert(item.name.clone(), Arc::new(item));
                }
            }
        }
    });
}

/// 查找对应主版本的Java
/// - `version`: 主版本
/// - `over`: 是否允许获取高版本
pub fn find_java(version: i32, over: bool) -> Option<Arc<JavaInfoObj>> {
    let list = JVMS.get()?.read().ok()?;

    let system_arch = get_system_info().system_arch;

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
