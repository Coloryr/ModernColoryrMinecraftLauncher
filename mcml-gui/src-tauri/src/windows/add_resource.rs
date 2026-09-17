//! 下载资源

use std::collections::HashMap;

use mcml_game::{
    curseforge,
    launcher::{FileType, ModPackType},
    loader::LoaderType,
    modrinth,
};
use mcml_net::{
    curseforge_api::{self, CurseFogreArg, CurseForgeSortType},
    modrinth_api::ModrinthSortType,
};

use crate::dtos::add_resource_dto::FileListDto;

/// 获取下载源
#[tauri::command]
pub fn get_source_type() -> Result<Vec<String>, String> {
    Ok(vec![
        ModPackType::Modrinth.to_string(),
        ModPackType::CurseForge.to_string(),
    ])
}

/// 获取分组
#[tauri::command]
pub async fn get_categories(
    source: String,
    file_type: String,
) -> Result<HashMap<String, String>, String> {
    let source = ModPackType::from_string(&source);
    let file_type = FileType::from_string(&file_type);
    if file_type.is_none() {
        return Err(String::from("file type not found"));
    }
    let file_type = file_type.unwrap();

    match source {
        ModPackType::CurseForge => curseforge::get_categories(file_type)
            .await
            .map_err(|err| err.to_string()),
        ModPackType::Modrinth => modrinth::get_categories(file_type)
            .await
            .map_err(|err| err.to_string()),
        _ => Err(String::from("error source type")),
    }
}

/// 获取排序方式
#[tauri::command]
pub fn get_sort_type(source: String) -> Result<Vec<String>, String> {
    let source = ModPackType::from_string(&source);

    match source {
        ModPackType::CurseForge => Ok(vec![
            CurseForgeSortType::Popularity.to_string(),
            CurseForgeSortType::Featured.to_string(),
            CurseForgeSortType::LastUpdated.to_string(),
            CurseForgeSortType::Name.to_string(),
            CurseForgeSortType::TotalDownloads.to_string(),
        ]),
        ModPackType::Modrinth => Ok(vec![
            ModrinthSortType::Relevance.to_string(),
            ModrinthSortType::Downloads.to_string(),
            ModrinthSortType::Follows.to_string(),
            ModrinthSortType::Newest.to_string(),
            ModrinthSortType::Updated.to_string(),
        ]),
        _ => Err(String::from("error source type")),
    }
}

/// 获取支持的游戏版本列表
#[tauri::command]
pub async fn get_game_versions(source: String) -> Result<Vec<String>, String> {
    let source = ModPackType::from_string(&source);

    match source {
        ModPackType::CurseForge => curseforge::get_game_versions()
            .await
            .map_err(|err| err.to_string()),
        ModPackType::Modrinth => modrinth::get_game_versions()
            .await
            .map_err(|err| err.to_string()),
        _ => Err("error source type".to_string()),
    }
}

/// 获取文件列表
#[tauri::command]
pub async fn get_file_versions(
    source: String,
    pid: String,
    file_type: String,
    page: u32,
    version: Option<String>,
    loader: Option<String>,
) -> Result<FileListDto, String> {
    let source = ModPackType::from_string(&source);
    let file_type = FileType::from_string(&file_type);
    if file_type.is_none() {
        return Err(String::from("error type not found"));
    }
    let file_type = file_type.unwrap();
    let mod_loader = LoaderType::from_string(&loader.unwrap_or_default());
    if mod_loader.is_none() {
        return Err(String::from("error type not found"));
    }
    let mod_loader = if matches!(file_type, FileType::Mod) {
        mod_loader.unwrap()
    } else {
        LoaderType::Normal
    };

    match source {
        ModPackType::CurseForge => {
            let data = curseforge_api::get_mod_info(&pid)
                .await
                .map_err(|err| err.to_string())?;
            let list = curseforge_api::get_files_page(CurseFogreArg {
                id: Some(pid.clone()),
                version: version,
                page: Some(page),
                loader: curseforge::to_loader_id(&mod_loader),
                ..Default::default()
            })
            .await
            .map_err(|err| err.to_string())?;

            Ok(FileListDto {
                list: list.data.iter().map(|item| ),
                count: list.pagination.total_count,
                name: data.data.name
            })

            todo!()
        }
        ModPackType::Modrinth => {
            todo!()
        }
        _ => Err(String::from("error source type")),
    }
}
