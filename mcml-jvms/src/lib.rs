use std::{
    collections::HashMap,
    fs,
    path::{self, PathBuf},
    sync::{Arc, OnceLock, RwLock},
};

use mcml_config::config_obj::JvmConfigObj;
use mcml_names::names;

use crate::java_info_obj::{ArchEnum, JavaInfoObj};

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

/// 添加Java
/// - `name`: 名字
/// - `file`: 路径
pub fn add_item(name: String, file: String) {
    let dir = DIR.get().unwrap();
    if file.starts_with(dir.to_string_lossy().to_string()) {
        
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

            let info = java_helper::get_java_info(&path).await;
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
                        arch: ArchEnum::Unknow,
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
            let list2 = java_helper::find_java().await;
            if list2.is_some() {
                let list2 = list2.unwrap();

                let list1 = JVMS.get().unwrap().read().unwrap();

                for item in list2.iter() {
                    list1.insert(k, v)
                }
            }
        }
    });
}
