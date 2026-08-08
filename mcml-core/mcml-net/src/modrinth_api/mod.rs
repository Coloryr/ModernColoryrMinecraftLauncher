use std::collections::HashMap;

use mcml_names::{
    i18_items::error_type::{CoreResult, ErrorData, ErrorType},
    names,
};
use serde::{Deserialize, Serialize};

use crate::{
    modrinth_api::{
        project_obj::ModrinthProjectObj, search_obj::ModrinthSearchObj, team_obj::ModrinthTeamObj,
        version_obj::ModrinthVersionObj,
    },
    urls,
};

pub mod project_obj;
pub mod search_obj;
pub mod team_obj;
pub mod version_obj;

pub const CLASS_MODPACK: &str = "modpack";
pub const CLASS_MOD: &str = "mod";
pub const CLASS_RESOURCEPACK: &str = "resourcepack";
pub const CLASS_SHADERPACK: &str = "shader";
pub const CATEGORIES_DATA_PACK: &str = "datapack";

const LIMITE_PER_MIN: u32 = 300;

/// 类型分类器
pub struct FacetsObj {
    pub data: String,
    pub values: Vec<String>,
}

/// Modrinth搜索排序方式
pub enum ModrinthSortType {
    /// 推荐
    Relevance,
    /// 下载次数
    Downloads,
    /// 订阅数量
    Follows,
    /// 最新发布
    Newest,
    /// 最后更新
    Updated,
}

impl ModrinthSortType {
    /// 获取排序方式名称
    pub fn get_index(&self) -> &'static str {
        match self {
            ModrinthSortType::Relevance => "relevance",
            ModrinthSortType::Downloads => "downloads",
            ModrinthSortType::Follows => "follows",
            ModrinthSortType::Newest => "newest",
            ModrinthSortType::Updated => "updated",
        }
    }
}

/// Modrinth搜索参数
pub struct ModrinthSearchArg {
    /// 游戏版本号
    pub verions: Option<String>,
    /// 搜索的名字
    pub query: Option<String>,
    /// 搜索排序
    pub sort: ModrinthSortType,
    /// 页数
    pub page: Option<u32>,
    /// 一页的大小
    pub page_size: Option<u32>,
    /// 筛选器
    pub category: Option<String>,
    /// 加载器类型
    pub loader: Option<String>,
}

fn build_facets(list: Vec<FacetsObj>) -> String {
    let mut str = String::new();

    str.push('[');

    for item in list.iter() {
        if item.values.is_empty() {
            continue;
        }

        for item1 in item.values.iter() {
            str.push_str(&format!("[\"{}:{}\"],", &item.data, item1));
        }
    }

    str.remove(str.len() - 1);
    str.push(']');

    str
}

fn build_categories(values: Vec<String>) -> FacetsObj {
    FacetsObj {
        data: "categories".to_string(),
        values,
    }
}

fn build_versions(values: Vec<String>) -> FacetsObj {
    FacetsObj {
        data: "versions".to_string(),
        values,
    }
}

fn build_project_type(values: Vec<String>) -> FacetsObj {
    FacetsObj {
        data: "project_type".to_string(),
        values,
    }
}

/// 搜索内容
async fn search(
    query: &str,
    index: ModrinthSortType,
    offset: u32,
    limit: u32,
    facets: Vec<FacetsObj>,
) -> CoreResult<ModrinthSearchObj> {
    // 查询词与 facets 含空格、引号、方括号等特殊字符，
    // 必须经 URL 编码，否则服务端会静默返回空结果
    let mut url = reqwest::Url::parse(&format!("{}search", urls::MODRINTH)).unwrap();
    url.query_pairs_mut()
        .append_pair("query", query)
        .append_pair("index", index.get_index())
        .append_pair("offset", &offset.to_string())
        .append_pair("limit", &limit.to_string())
        .append_pair("facets", &build_facets(facets));

    crate::get_work_client()
        .get_json_limited(url.as_str(), LIMITE_PER_MIN)
        .await
}

/// 获取整合包列表
pub async fn get_modpack_list(arg: ModrinthSearchArg) -> CoreResult<ModrinthSearchObj> {
    let page_size = arg.page_size.unwrap_or(20);
    let mut facets = Vec::new();

    facets.push(build_project_type(vec![CLASS_MODPACK.to_string()]));

    if let Some(verions) = arg.verions {
        facets.push(build_versions(vec![verions.clone()]));
    }

    if let Some(category) = arg.category {
        facets.push(build_categories(vec![category.clone()]));
    }

    search(
        &arg.query.unwrap_or_default(),
        arg.sort,
        arg.page.unwrap_or(0) * page_size,
        page_size,
        facets,
    )
    .await
}

/// 获取模组列表
pub async fn get_mod_list(arg: ModrinthSearchArg) -> CoreResult<ModrinthSearchObj> {
    let page_size = arg.page_size.unwrap_or(20);
    let mut facets = Vec::new();

    facets.push(build_project_type(vec![CLASS_MOD.to_string()]));

    if let Some(verions) = arg.verions {
        facets.push(build_versions(vec![verions.clone()]));
    }

    let mut cate = build_categories(Vec::new());

    if let Some(category) = arg.category {
        cate.values.push(category.clone());
    }

    if let Some(loader) = arg.loader {
        cate.values.push(loader.clone());
    }

    facets.push(cate);

    search(
        &arg.query.unwrap_or_default(),
        arg.sort,
        arg.page.unwrap_or(0) * page_size,
        page_size,
        facets,
    )
    .await
}

/// 获取资源包列表
pub async fn get_resourcepack_list(arg: ModrinthSearchArg) -> CoreResult<ModrinthSearchObj> {
    let page_size = arg.page_size.unwrap_or(20);
    let mut facets = Vec::new();

    facets.push(build_project_type(vec![CLASS_RESOURCEPACK.to_string()]));

    if let Some(verions) = arg.verions {
        facets.push(build_versions(vec![verions.clone()]));
    }

    if let Some(category) = arg.category {
        facets.push(build_categories(vec![category.clone()]));
    }

    search(
        &arg.query.unwrap_or_default(),
        arg.sort,
        arg.page.unwrap_or(0) * page_size,
        page_size,
        facets,
    )
    .await
}

/// 获取光影包列表
pub async fn get_shaderpack_list(arg: ModrinthSearchArg) -> CoreResult<ModrinthSearchObj> {
    let page_size = arg.page_size.unwrap_or(20);
    let mut facets = Vec::new();

    facets.push(build_project_type(vec![CLASS_SHADERPACK.to_string()]));

    if let Some(verions) = arg.verions {
        facets.push(build_versions(vec![verions.clone()]));
    }

    if let Some(category) = arg.category {
        facets.push(build_categories(vec![category.clone()]));
    }

    search(
        &arg.query.unwrap_or_default(),
        arg.sort,
        arg.page.unwrap_or(0) * page_size,
        page_size,
        facets,
    )
    .await
}

/// 获取数据包列表
pub async fn get_datapack_list(arg: ModrinthSearchArg) -> CoreResult<ModrinthSearchObj> {
    let page_size = arg.page_size.unwrap_or(20);
    let mut facets = Vec::new();

    facets.push(build_project_type(vec![CLASS_MOD.to_string()]));

    if let Some(verions) = arg.verions {
        facets.push(build_versions(vec![verions.clone()]));
    }

    if let Some(category) = arg.category {
        facets.push(build_categories(vec![
            category.clone(),
            CATEGORIES_DATA_PACK.to_string(),
        ]));
    }

    search(
        &arg.query.unwrap_or_default(),
        arg.sort,
        arg.page.unwrap_or(0) * page_size,
        page_size,
        facets,
    )
    .await
}

/// 获取指定版本号的内容
///
/// - `id`: 项目编号
/// - `version`: 版本号
pub async fn get_version(id: &str, version: &str) -> CoreResult<ModrinthVersionObj> {
    let url = format!("{}project/{id}/version/{version}", urls::MODRINTH);

    crate::get_work_client()
        .get_json_limited(&url, LIMITE_PER_MIN)
        .await
}

/// 根据版本号获取项目信息
///
/// - `ids`: 版本号
pub async fn get_versions(ids: Vec<String>) -> CoreResult<Vec<ModrinthVersionObj>> {
    // 用紧凑 JSON + URL 编码：`json_to_string` 输出 pretty 多行格式，
    // 直接拼进 URL 会带换行与缩进（服务端解析失败或必须依赖 reqwest 兜底清理）
    let ids_json = serde_json::to_string(&ids).map_err(|err| {
        ErrorType::SerializerError(ErrorData {
            error: err.to_string(),
        })
    })?;

    let mut url = reqwest::Url::parse(&format!("{}versions", urls::MODRINTH)).unwrap();
    url.query_pairs_mut().append_pair("ids", &ids_json);

    crate::get_work_client()
        .get_json_limited(url.as_str(), LIMITE_PER_MIN)
        .await
}

/// 获取团队列表
///
/// - `id`: 项目编号
pub async fn get_team(id: &str) -> CoreResult<Vec<ModrinthTeamObj>> {
    let url = format!("{}project/{id}/members", urls::MODRINTH);

    crate::get_work_client()
        .get_json_limited(&url, LIMITE_PER_MIN)
        .await
}

/// 获取指定项目的内容
///
/// - `id`: 项目编号
pub async fn get_project(id: &str) -> CoreResult<ModrinthProjectObj> {
    let url = format!("{}project/{id}", urls::MODRINTH);

    crate::get_work_client()
        .get_json_limited(&url, LIMITE_PER_MIN)
        .await
}

/// 获取文件列表
///
/// - `id`: 项目编号
/// - `version`: 游戏版本
/// - `loader`: 加载器版本
pub async fn get_file_versions(
    id: &str,
    version: Option<&str>,
    loader: Option<&str>,
) -> CoreResult<Vec<ModrinthVersionObj>> {
    let url = match version {
        Some(version) => {
            let mut url = format!(
                "{}project/{id}/version?game_versions=[\"{version}\"]",
                urls::MODRINTH
            );

            if let Some(loader) = loader {
                url.push_str(&format!("&loaders=[\"{}\"]", loader.to_lowercase()));
            }

            url
        }
        None => {
            let mut url = format!("{}project/{id}/version?", urls::MODRINTH);

            if let Some(loader) = loader {
                url.push_str(&format!("loaders=[\"{}\"]", loader.to_lowercase()));
            }

            url
        }
    };

    crate::get_work_client()
        .get_json_limited(&url, LIMITE_PER_MIN)
        .await
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct ModrinthGameVersionObj {
    pub version: String,
}

impl Default for ModrinthGameVersionObj {
    fn default() -> Self {
        Self {
            version: Default::default(),
        }
    }
}

/// 获取所有游戏版本
pub async fn get_game_versions() -> CoreResult<Vec<ModrinthGameVersionObj>> {
    let url = format!("{}tag/game_version", urls::MODRINTH);

    crate::get_work_client()
        .get_json_limited(&url, LIMITE_PER_MIN)
        .await
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct ModrinthCategoriesObj {
    pub icon: String,
    pub name: String,
    pub project_type: String,
    pub header: String,
}

impl Default for ModrinthCategoriesObj {
    fn default() -> Self {
        Self {
            icon: Default::default(),
            name: Default::default(),
            project_type: Default::default(),
            header: Default::default(),
        }
    }
}

/// 获取所有类型
pub async fn get_categories() -> CoreResult<Vec<ModrinthCategoriesObj>> {
    let url = format!("{}tag/category", urls::MODRINTH);

    crate::get_work_client()
        .get_json_limited(&url, LIMITE_PER_MIN)
        .await
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
struct VersionHashObj {
    pub hashes: Vec<String>,
    pub algorithm: String,
}

impl Default for VersionHashObj {
    fn default() -> Self {
        Self {
            hashes: Default::default(),
            algorithm: Default::default(),
        }
    }
}

/// 从文件Sha1获取项目
///
/// - `sha1`: 文件sha1
pub async fn get_version_from_sha1(
    sha1: Vec<String>,
) -> CoreResult<HashMap<String, ModrinthVersionObj>> {
    let url = format!("{}version_files", urls::MODRINTH);

    crate::get_work_client()
        .post_json_get_json_limited(
            &url,
            &VersionHashObj {
                hashes: sha1,
                algorithm: names::SHA1_EXT.to_string(),
            },
            LIMITE_PER_MIN,
        )
        .await
}

/// 从文件Sha512获取项目
///
/// - `sha512`: 文件sha512
pub async fn get_version_from_sha512(
    sha512: Vec<String>,
) -> CoreResult<HashMap<String, ModrinthVersionObj>> {
    let url = format!("{}version_files", urls::MODRINTH);

    crate::get_work_client()
        .post_json_get_json_limited(
            &url,
            &VersionHashObj {
                hashes: sha512,
                algorithm: names::SHA512_EXT.to_string(),
            },
            LIMITE_PER_MIN,
        )
        .await
}
