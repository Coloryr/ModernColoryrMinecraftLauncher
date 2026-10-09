//! 收藏窗口
//!
//! 数据由 `crate::collect_utils` 持有（`collect.json`），这里只做 DTO 转换与增删改命令。
//! 类型过滤状态属于 `gui_config`，走窗口配置那条 IPC，不在这里。
//! 分组与过滤都在前端算（数据一次性给全），所以切分组 / 切过滤不需要往返。

use mml_game::launcher::{FileType, ModPackType};
use mml_net::curseforge_api;
use mml_net::modrinth_api;
use mml_net::modrinth_api::search_obj::HitObj;
use tauri::{AppHandle, Emitter};

use crate::collect_utils::{self, CollectItemObj};
use crate::dtos::CollectDataDto;
use crate::dtos::add_resource_dto::ProjectItemDto;
use crate::image_manager;
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
/// 参数是 IPC 契约（`bindings.ts` 由 Rust 源码生成，见 AGENTS.md §4），不能合并成结构体。
#[allow(clippy::too_many_arguments)]
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
        // 落盘存**原始图片网址**：本地协议地址（`.../icon/<sha>`）的登记只在内存里，
        // 存下来重启后就解析不了（既回不了源、也没法做过期校验）
        icon: icon.map(|url| image_manager::remote_url(&url).unwrap_or(url)),
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

/// 取项目图标地址（收藏时没存到图标的老条目回源用）
///
/// 收藏时图标是从列表项里顺手存下的，早期条目可能为空 —— 这种就按 下载源 + 项目 ID
/// 去对应平台的 API 补一次；地址同样经 image_manager 转发（与列表页一致）。
#[tauri::command]
pub async fn collect_project_icon(source: String, pid: String) -> Result<Option<String>, String> {
    match ModPackType::from_string(&source) {
        ModPackType::CurseForge => {
            let data = curseforge_api::get_mod_info(&pid)
                .await
                .map_err(|err| err.to_string())?;

            Ok(data
                .data
                .logo
                .url
                .filter(|url| !url.is_empty())
                .map(|url| image_manager::push_image_url(&url)))
        }
        ModPackType::Modrinth => {
            let data = modrinth_api::get_project(&pid)
                .await
                .map_err(|err| err.to_string())?;

            // icon_url 是 String（不是 Option）：空串按没有处理 —— 上面 CurseForge 那支同理
            Ok(if data.icon_url.is_empty() {
                None
            } else {
                Some(image_manager::push_image_url(&data.icon_url))
            })
        }
        _ => Err(String::from("err.sourceType")),
    }
}

/// 登记一个原始图片网址，返回可直接放到 `src` 上的地址
///
/// 收藏里存的是**原始网址**（见 `collect_star`），前端渲染前要过这一道：
/// 登记进 image_manager 之后，图片才走统一的缓存与 ETag 过期校验，
/// 也才能被 `mml-image` 协议服务。
#[tauri::command]
pub fn collect_image_url(url: String) -> String {
    image_manager::push_image_url(&url)
}

/// 取项目条目（收藏窗口跳转到下载窗口时用）
///
/// 收藏条目里只有 名字 / 图标 / 网址 / 源 / 项目ID，直接拿它造下载窗口的条目会缺
/// 下载次数、更新时间、收藏状态 —— 详情页正好要显示这些，所以按 源 + 项目ID 现取一次。
#[tauri::command]
pub async fn collect_project_item(
    source: String,
    pid: String,
    file_type: String,
) -> Result<Option<ProjectItemDto>, String> {
    let file_type = match FileType::from_string(&file_type) {
        Some(data) => data,
        None => return Err(String::from("err.fileTypeNotFound")),
    };
    // 收藏状态以本地收藏夹为准
    let is_star = collect_utils::is_star(&pid);

    let item = match ModPackType::from_string(&source) {
        ModPackType::CurseForge => {
            let data = curseforge_api::get_mod_info(&pid)
                .await
                .map_err(|err| err.to_string())?;

            ProjectItemDto::new_curseforge(&data.data, file_type, false, true, is_star, false, None)
        }
        ModPackType::Modrinth => {
            let data = modrinth_api::get_project(&pid)
                .await
                .map_err(|err| err.to_string())?;

            // 详情接口只给简介 / 正文 / 作者 / 标签 / 截图，列表条目要的那几项在这里补
            let hit = HitObj {
                project_id: data.id.clone(),
                title: data.title.clone(),
                description: data.description.clone(),
                categories: data.categories.clone(),
                downloads: data.downloads,
                icon_url: Some(data.icon_url.clone()),
                date_modified: data.updated.clone(),
                ..Default::default()
            };

            ProjectItemDto::new_modrinth(&hit, file_type, false, true, is_star, false, None).await
        }
        _ => return Err(String::from("err.sourceType")),
    };

    Ok(Some(item))
}
