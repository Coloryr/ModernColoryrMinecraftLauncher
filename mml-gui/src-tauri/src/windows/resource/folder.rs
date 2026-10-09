//! 资源管理窗口的「打开文件夹」：按类别打开实例资源目录（或选中某个文件）

use std::path::Path;

use mml_names::names;
use mml_sys::{open_helper, path_helper};

use super::{KIND_DATAPACKS, instance_dir, parse_instance, resource_file};

// ==================== 打开文件夹 ====================

/// 打开资源目录（name 为空打开目录本身，目录不存在则先创建；
/// 带 name 时资源管理器定位到该文件。datapacks 类别需要 parent = 存档目录名）
#[gui_macros::ipc_group("resource")]
#[tauri::command]
pub fn resource_open_folder(
    uuid: String,
    kind: String,
    name: Option<String>,
    parent: Option<String>,
) -> Result<(), String> {
    let instance = parse_instance(&uuid)?;
    let dir = if kind == KIND_DATAPACKS {
        let parent = parent
            .as_deref()
            .filter(|p| !p.trim().is_empty())
            .ok_or_else(|| "err.fileName".to_string())?;
        let parent_path = Path::new(parent);
        if parent_path.file_name() != Some(parent_path.as_os_str()) {
            return Err("err.fileName".to_string());
        }
        let saves = instance.read().unwrap().get_saves_path();
        saves.join(parent_path).join(names::GAME_DATAPACK_DIR)
    } else {
        instance_dir(&instance, &kind)?
    };

    let target = match name.as_deref() {
        Some(name) if !name.trim().is_empty() => resource_file(&instance, &kind, name)?,
        _ => {
            if !dir.exists() {
                path_helper::create_dir_all(&dir).map_err(|err| err.to_string())?;
            }
            dir
        }
    };
    if !target.exists() {
        return Err("err.fileNotFound".to_string());
    }

    open_helper::open_file_with_explorer(&target);
    Ok(())
}
