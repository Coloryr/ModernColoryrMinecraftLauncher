//! 资源收藏

use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::{LazyLock, OnceLock, RwLock},
};

use mcml_base::serialize_tools;
use mcml_config::config_save;
use mcml_game::launcher::{FileType, ModPackType};
use mcml_names::{i18_items::error_type::CoreResult, names, uuids};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 收藏项目
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct CollectItemObj {
    #[serde(skip)]
    pub uuid: String,
    /// 下载源
    #[serde(rename = "Source")]
    pub source: ModPackType,
    /// 资源类型
    #[serde(rename = "FileType")]
    pub file_type: FileType,
    /// 资源名字
    #[serde(rename = "Name")]
    pub name: String,
    /// 项目ID
    #[serde(rename = "Pid")]
    pub pid: String,
    /// 图标
    #[serde(rename = "Icon")]
    pub icon: Option<String>,
    /// 网址
    #[serde(rename = "Url")]
    pub url: String,
}

impl Default for CollectItemObj {
    fn default() -> Self {
        Self {
            uuid: Default::default(),
            source: Default::default(),
            file_type: Default::default(),
            name: Default::default(),
            pid: Default::default(),
            icon: Default::default(),
            url: Default::default(),
        }
    }
}

/// 资源收藏
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct CollectObj {
    /// 收藏项目列表
    #[serde(rename = "Items")]
    pub items: HashMap<String, CollectItemObj>,
    /// 收藏分组列表
    #[serde(rename = "Groups")]
    pub groups: HashMap<String, HashSet<String>>,
}

impl Default for CollectObj {
    fn default() -> Self {
        Self {
            items: Default::default(),
            groups: Default::default(),
        }
    }
}

static FILE: OnceLock<PathBuf> = OnceLock::new();
static COLLECT: LazyLock<RwLock<CollectObj>> = LazyLock::new(|| RwLock::new(Default::default()));

/// 记录收藏文件路径（启动时调用）；实际读取见 [`load`]
pub fn init<P: AsRef<Path>>(path: P) {
    FILE.get_or_init(|| path.as_ref().join(names::COLLECT_FILE));
}

/// 取当前收藏（克隆一份，供命令层转换 DTO）
pub fn get() -> CollectObj {
    COLLECT.read().unwrap().clone()
}

pub fn load() -> CoreResult<()> {
    let file = FILE.get().unwrap();
    let mut obj: CollectObj = if file.exists() {
        serialize_tools::json_from_file(file)?
    } else {
        Default::default()
    };

    for (key, value) in obj.items.iter_mut() {
        value.uuid = key.clone();
    }

    for group in obj.groups.values_mut() {
        group.retain(|key1| obj.items.contains_key(key1));
    }

    *COLLECT.write().unwrap() = obj;

    Ok(())
}

pub fn save() {
    config_save::save(
        uuids::COLLECT_UUID,
        &*COLLECT.read().unwrap(),
        FILE.get().unwrap(),
    );
}

pub fn remove_item(obj: CollectItemObj) {
    {
        let mut collect = COLLECT.write().unwrap();

        let key = match collect
            .items
            .iter()
            .find(|(_, value)| value.source == obj.source && value.pid == obj.pid)
        {
            Some((key, _)) => key.clone(),
            None => return,
        };

        for (_, value) in collect.groups.iter_mut() {
            value.remove(&key);
        }

        collect.items.remove(&key);
    }

    save();
}

pub fn remove_uuid(uuid: &str) {
    {
        let mut collect = COLLECT.write().unwrap();

        collect.items.remove(uuid);

        for (_, value) in collect.groups.iter_mut() {
            value.remove(uuid);
        }
    }

    save();
}

pub fn add_item(obj: CollectItemObj) {
    {
        let mut collect = COLLECT.write().unwrap();

        let have = collect
            .items
            .iter()
            .any(|(_, value)| value.source == obj.source && value.pid == obj.pid);

        if have {
            return;
        }

        let uuid = Uuid::new_v4().to_string();
        collect.items.insert(uuid, obj);
    }

    save();
}

pub fn add_group(name: &str) {
    {
        let mut collect = COLLECT.write().unwrap();

        if collect.groups.contains_key(name) {
            return;
        }

        collect.groups.insert(name.to_string(), Default::default());
    }

    save();
}

pub fn remove_group(name: &str) {
    {
        let mut collect = COLLECT.write().unwrap();
        collect.groups.remove(name);
    }

    save();
}

/// 清空全部收藏（分组保留，只清成员）
pub fn clear() {
    {
        let mut collect = COLLECT.write().unwrap();

        collect.items.clear();

        for (_, value) in collect.groups.iter_mut() {
            value.clear();
        }
    }

    save();
}

/// 只清空某个分组的成员（收藏条目本身保留）
pub fn clear_group(name: &str) {
    {
        let mut collect = COLLECT.write().unwrap();

        if let Some(value) = collect.groups.get_mut(name) {
            value.clear();
        }
    }

    save();
}

/// 从分组移除若干收藏（条目本身保留）
pub fn remove_group_items(group: &str, uuids: &[String]) {
    {
        let mut collect = COLLECT.write().unwrap();

        if let Some(value) = collect.groups.get_mut(group) {
            for uuid in uuids {
                value.remove(uuid);
            }
        }
    }

    save();
}

/// 把若干收藏加入分组（分组不存在时不做任何事）
pub fn set_group_items(group: &str, uuids: &[String]) {
    {
        let mut collect = COLLECT.write().unwrap();

        if let Some(value) = collect.groups.get_mut(group) {
            for uuid in uuids {
                value.insert(uuid.clone());
            }
        }
    }

    save();
}

pub fn is_star(pid: &str) -> bool {
    let data = COLLECT.read().unwrap();

    // items 以收藏条目的 uuid 为键，按 pid 匹配值里的项目ID
    data.items.values().any(|item| item.pid == pid)
}
