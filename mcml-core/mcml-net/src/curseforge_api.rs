use std::sync::OnceLock;

use mcml_names::i18_items::error_type::{CoreResult, ErrorData, ErrorType};
use reqwest::{
    Method, Url,
    header::{CONTENT_TYPE, HeaderValue},
};
use serde::{Serialize, de::DeserializeOwned};

use crate::urls;

pub const GAME_ID: u32 = 432;
pub const CLASS_MODPACK: u32 = 4471;
pub const CLASS_MOD: u32 = 6;
pub const CLASS_SAVES: u32 = 17;
pub const CLASS_RESOURCEPACKS: u32 = 12;
pub const CLASS_SHADERPACKS: u32 = 6552;
pub const CLASS_OPENLOADER_DATAPACK: u32 = 6945;
pub const CATEGORYID_DATAPACKS: u32 = 5193;

static API_KEY: OnceLock<String> = OnceLock::new();

/// 搜索参数
pub struct CurseFogreArg {
    /// 项目编号
    id: Option<String>,
    /// 游戏版本
    version: Option<String>,
    /// 页数
    page: Option<u32>,
    /// 过滤
    sort: Option<u32>,
    /// 过滤名
    filter: Option<String>,
    /// 页大小
    page_size: Option<u32>,
    /// 排序方式
    sort_order: Option<u32>,
    /// 分类
    category: Option<String>,
    /// 模组加载器类型
    loader: Option<u32>,
}

/// 设置API KEY
/// - `key`: 密钥
pub fn set_key(key: &str) {
    API_KEY.get_or_init(|| key.to_string());
}

/// 获取API KEY
pub fn get_key() -> CoreResult<String> {
    match API_KEY.get() {
        Some(key) => Ok(key.clone()),
        None => Err(ErrorType::KeyIsNull),
    }
}

/// 发送请求
/// - `req`: 请求内容
async fn send<T: DeserializeOwned>(mut req: reqwest::Request) -> CoreResult<T> {
    req.headers_mut()
        .insert("x-api-key", get_key()?.parse().unwrap());

    let res = crate::get_work_client().send(req).await?;
    crate::handle_response(res).await
}

async fn get_list<T: DeserializeOwned>(
    classid: u32,
    version: &str,
    page: u32,
    sort: u32,
    filter: &str,
    page_size: u32,
    sort_order: u32,
    category: &str,
    mod_loader_type: u32,
) -> CoreResult<T> {
    let mut url = format!(
        "{}mods/search?gameId={}&classId={classid}&gameVersion={version}&index={}&sortField={sort}&searchFilter={filter}&pageSize={page_size}&sortOrder={sort_order}&categoryId={category}",
        urls::CURSEFORGE,
        GAME_ID,
        page * page_size
    );

    if mod_loader_type != 0 {
        url.push_str(&format!("&modLoaderType={mod_loader_type}"));
    }

    let req = reqwest::Request::new(Method::GET, Url::parse(&url).unwrap());

    send(req).await
}

/// 获取整合包列表
pub async fn get_modpack_list<T: DeserializeOwned>(arg: CurseFogreArg) -> CoreResult<T> {
    get_list(
        CLASS_MODPACK,
        &arg.version.unwrap_or_default(),
        arg.page.unwrap_or(0),
        arg.sort.unwrap_or(2),
        &arg.filter.unwrap_or_default(),
        arg.page_size.unwrap_or(20),
        arg.sort_order.unwrap_or(1),
        &arg.category.unwrap_or_default(),
        0,
    )
    .await
}

/// 获取模组列表
pub async fn get_mod_list<T: DeserializeOwned>(arg: CurseFogreArg) -> CoreResult<T> {
    get_list(
        CLASS_MOD,
        &arg.version.unwrap_or_default(),
        arg.page.unwrap_or(0),
        arg.sort.unwrap_or(2),
        &arg.filter.unwrap_or_default(),
        arg.page_size.unwrap_or(20),
        arg.sort_order.unwrap_or(1),
        &arg.category.unwrap_or_default(),
        arg.loader.unwrap_or(0),
    )
    .await
}

/// 获取存档列表
pub async fn get_save_list<T: DeserializeOwned>(arg: CurseFogreArg) -> CoreResult<T> {
    get_list(
        CLASS_SAVES,
        &arg.version.unwrap_or_default(),
        arg.page.unwrap_or(0),
        arg.sort.unwrap_or(2),
        &arg.filter.unwrap_or_default(),
        arg.page_size.unwrap_or(20),
        arg.sort_order.unwrap_or(1),
        &arg.category.unwrap_or_default(),
        0,
    )
    .await
}

/// 获取资源包列表
pub async fn get_resourcepack_list<T: DeserializeOwned>(arg: CurseFogreArg) -> CoreResult<T> {
    get_list(
        CLASS_RESOURCEPACKS,
        &arg.version.unwrap_or_default(),
        arg.page.unwrap_or(0),
        arg.sort.unwrap_or(2),
        &arg.filter.unwrap_or_default(),
        arg.page_size.unwrap_or(20),
        arg.sort_order.unwrap_or(1),
        &arg.category.unwrap_or_default(),
        0,
    )
    .await
}

/// 获取数据包列表
pub async fn get_datapacks_list<T: DeserializeOwned>(arg: CurseFogreArg) -> CoreResult<T> {
    get_list(
        CLASS_RESOURCEPACKS,
        &arg.version.unwrap_or_default(),
        arg.page.unwrap_or(0),
        arg.sort.unwrap_or(2),
        &arg.filter.unwrap_or_default(),
        arg.page_size.unwrap_or(20),
        arg.sort_order.unwrap_or(1),
        &CATEGORYID_DATAPACKS.to_string(),
        0,
    )
    .await
}

/// 获取光影包列表
pub async fn get_shaders_list<T: DeserializeOwned>(arg: CurseFogreArg) -> CoreResult<T> {
    get_list(
        CLASS_SHADERPACKS,
        &arg.version.unwrap_or_default(),
        arg.page.unwrap_or(0),
        arg.sort.unwrap_or(2),
        &arg.filter.unwrap_or_default(),
        arg.page_size.unwrap_or(20),
        arg.sort_order.unwrap_or(1),
        "",
        0,
    )
    .await
}

/// 获取模组信息
/// - `pid`: 项目编号
/// - `fid`: 文件编号
pub async fn get_mod<T: DeserializeOwned>(pid: &str, fid: &str) -> CoreResult<T> {
    let url = format!("{}mods/{pid}/files/{fid}", urls::CURSEFORGE);

    let req = reqwest::Request::new(Method::GET, Url::parse(&url).unwrap());

    send(req).await
}

#[derive(Serialize, Debug)]
#[serde(default)]
struct CurseForgeGetFilesObj {
    #[serde(rename = "fileIds")]
    pub file_ids: Vec<u64>,
}

pub fn json<T: Serialize>(req: &mut reqwest::Request, json: &T) -> CoreResult<()> {
    match serde_json::to_vec(json) {
        Ok(body) => {
            req.headers_mut()
                .entry(CONTENT_TYPE)
                .or_insert_with(|| HeaderValue::from_static("application/json"));
            *req.body_mut() = Some(body.into());

            Ok(())
        }
        Err(err) => Err(ErrorType::SerializerError(ErrorData {
            error: err.to_string(),
        })),
    }
}

/// 获取文件列表
pub async fn get_files<T: DeserializeOwned>(ids: Vec<u64>) -> CoreResult<T> {
    let obj = CurseForgeGetFilesObj { file_ids: ids };

    let url = format!("{}mods/files", urls::CURSEFORGE);

    let mut req = reqwest::Request::new(Method::POST, Url::parse(&url).unwrap());

    json(&mut req, &obj)?;

    send(req).await
}

/// 获取分类信息
pub async fn get_categories<T: DeserializeOwned>() -> CoreResult<T> {
    let url = format!("{}categories?gameId={}", urls::CURSEFORGE, GAME_ID);

    let req = reqwest::Request::new(Method::GET, Url::parse(&url).unwrap());

    send(req).await
}

/// 获取版本信息
pub async fn get_version<T: DeserializeOwned>() -> CoreResult<T> {
    let url = format!("{}games/{}/versions", urls::CURSEFORGE, GAME_ID);

    let req = reqwest::Request::new(Method::GET, Url::parse(&url).unwrap());

    send(req).await
}

/// 获取版本信息
pub async fn get_version_type<T: DeserializeOwned>() -> CoreResult<T> {
    let url = format!("{}games/{}/version-types", urls::CURSEFORGE, GAME_ID);

    let req = reqwest::Request::new(Method::GET, Url::parse(&url).unwrap());

    send(req).await
}

/// 获取版本信息
pub async fn get_mod_info<T: DeserializeOwned>(id: &str) -> CoreResult<T> {
    let url = format!("{}mods/{id}", urls::CURSEFORGE);

    let req = reqwest::Request::new(Method::GET, Url::parse(&url).unwrap());

    send(req).await
}

#[derive(Serialize, Debug)]
#[serde(default)]
struct CurseForgeModsInfoObj {
    #[serde(rename = "modIds")]
    pub mod_ids: Vec<u64>,
    #[serde(rename = "filterPcOnly")]
    pub filter: bool,
}

/// 获取版本信息
pub async fn get_mods_info<T: DeserializeOwned>(ids: Vec<u64>) -> CoreResult<T> {
    let obj = CurseForgeModsInfoObj {
        mod_ids: ids,
        filter: true,
    };

    let url = format!("{}mods", urls::CURSEFORGE);

    let mut req = reqwest::Request::new(Method::POST, Url::parse(&url).unwrap());

    json(&mut req, &obj)?;

    send(req).await
}

/// 获取文件列表
pub async fn get_files_page<T: DeserializeOwned>(arg: CurseFogreArg) -> CoreResult<T> {
    let mut url = format!(
        "{}mods/{}/files?index={}pageSize=50&gameVersion={}",
        urls::CURSEFORGE,
        arg.id.unwrap_or_default(),
        arg.page.unwrap_or(0) * 50,
        arg.version.unwrap_or_default()
    );

    if let Some(loader) = arg.loader {
        url.push_str(&format!("&modLoaderType={loader}"));
    }

    let req = reqwest::Request::new(Method::POST, Url::parse(&url).unwrap());

    send(req).await
}
