//! 收藏窗口
//!
//! 数据由 `crate::collect_utils` 持有（`collect.json`），这里只做 DTO 转换与增删改命令。
//! 类型过滤状态属于 `gui_config`，走窗口配置那条 IPC，不在这里。
//! 分组与过滤都在前端算（数据一次性给全），所以切分组 / 切过滤不需要往返。

use mcml_game::launcher::{FileType, ModPackType};
use tauri::{AppHandle, Emitter};

use crate::collect_utils::{self, CollectItemObj};
use crate::dtos::CollectDataDto;
use crate::listens;

/// 收藏变更事件（收藏窗口刷新用）
#[gui_macros::emit]
fn emit_collect_change(app: &AppHandle) {
    let _ = app.emit(listens::COLLECT_CHANGE, ());
}

/// 获取收藏数据（收藏项 + 分组）
#[tauri::command]
pub fn collect_get_data() -> Result<CollectDataDto, String> {
    Ok(collect_utils::get().into())
}

/// 添加分组（重名返回错误，前端提示）
#[tauri::command]
pub fn collect_add_group(app: AppHandle, name: String) -> Result<(), String> {
    let name = name.trim();
    if name.is_empty() {
        return Err(String::from("err.groupEmpty"));
    }
    if collect_utils::get().groups.contains_key(name) {
        return Err(String::from("err.groupExists"));
    }

    collect_utils::add_group(name);
    emit_collect_change(&app);

    Ok(())
}

/// 删除分组（分组内的收藏条目保留）
#[tauri::command]
pub fn collect_remove_group(app: AppHandle, name: String) {
    collect_utils::remove_group(&name);
    emit_collect_change(&app);
}

/// 清空收藏
///
/// - `group` 为 `None`：清空全部收藏（分组保留）
/// - `group` 为 `Some`：只清空该分组的成员（收藏条目保留）
#[tauri::command]
pub fn collect_clear(app: AppHandle, group: Option<String>) {
    match group {
        Some(name) => collect_utils::clear_group(&name),
        None => collect_utils::clear(),
    }
    emit_collect_change(&app);
}

/// 移除收藏
///
/// - `group` 为 `None`：从收藏中删除（并从所有分组移除）
/// - `group` 为 `Some`：只从该分组移除，条目仍在收藏里
#[tauri::command]
pub fn collect_remove_items(app: AppHandle, uuids: Vec<String>, group: Option<String>) {
    match group {
        Some(name) => collect_utils::remove_group_items(&name, &uuids),
        None => {
            for uuid in &uuids {
                collect_utils::remove_uuid(uuid);
            }
        }
    }
    emit_collect_change(&app);
}

/// 把收藏加入分组
#[tauri::command]
pub fn collect_set_group_items(app: AppHandle, group: String, uuids: Vec<String>) {
    collect_utils::set_group_items(&group, &uuids);
    emit_collect_change(&app);
}

/// 收藏 / 取消收藏在线项目（整合包、资源窗口列表与详情里的星标）
///
/// `star = true` 加入收藏（重复收藏忽略），`false` 移除（按 下载源 + 项目ID 匹配）
#[tauri::command]
pub fn collect_star(
    app: AppHandle,
    source: String,
    file_type: String,
    pid: String,
    name: String,
    icon: Option<String>,
    url: String,
    star: bool,
) -> Result<(), String> {
    let source = ModPackType::from_string(&source);
    let file_type = match FileType::from_string(&file_type) {
        Some(data) => data,
        None => return Err(String::from("err.fileTypeNotFound")),
    };

    let item = CollectItemObj {
        source,
        file_type,
        name,
        pid,
        icon,
        url,
        ..Default::default()
    };

    if star {
        collect_utils::add_item(item);
    } else {
        collect_utils::remove_item(item);
    }
    emit_collect_change(&app);

    Ok(())
}
