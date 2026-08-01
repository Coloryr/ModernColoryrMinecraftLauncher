//! CurseForge API
//!
//! 提供与 CurseForge REST API 交互的功能，支持搜索和下载：
//!
//! # 支持的内容类型
//!
//! | 常量 | 值 | 用途 |
//! |------|---|------|
//! | `CLASS_MOD` | 6 | 模组 |
//! | `CLASS_MODPACK` | 4471 | 整合包 |
//! | `CLASS_SAVES` | 17 | 存档 |
//! | `CLASS_RESOURCEPACKS` | 12 | 资源包 |
//! | `CLASS_SHADERPACKS` | 6552 | 光影包 |
//!
//! # API Key
//!
//! CurseForge API 需要 API Key，通过 [`set_key()`] 在初始化时设置。

use std::sync::OnceLock;

use mcml_names::i18_items::error_type::{CoreResult, ErrorData, ErrorType};
use reqwest::{
    Method, Url,
    header::{CONTENT_TYPE, HeaderValue},
};
use serde::{Serialize, de::DeserializeOwned};

use crate::{
    curseforge_api::{
        categories_obj::CurseForgeCategoriesObj,
        file_obj::{CurseFogreFilePageObj, CurseForgeFileDataObj, CurseForgeFileObj},
        list_obj::{CurseForgeListObj, CurseForgeListPageObj},
        version_obj::{CurseForgeVersionObj, CurseForgeVersionTypeObj},
    },
    urls,
};

pub mod categories_obj;
pub mod file_obj;
pub mod list_obj;
pub mod version_obj;

/// CurseForge 游戏 ID（Minecraft = 432）
pub const GAME_ID: u32 = 432;
/// 分类 ID：整合包
pub const CLASS_MODPACK: u32 = 4471;
/// 分类 ID：模组
pub const CLASS_MOD: u32 = 6;
/// 分类 ID：存档
pub const CLASS_SAVES: u32 = 17;
/// 分类 ID：资源包
pub const CLASS_RESOURCEPACKS: u32 = 12;
/// 分类 ID：光影包
pub const CLASS_SHADERPACKS: u32 = 6552;
/// 分类 ID：数据包（OpenLoader）
pub const CLASS_OPENLOADER_DATAPACK: u32 = 6945;
/// 类别 ID：数据包
pub const CATEGORYID_DATAPACKS: u32 = 5193;

/// 全局 CurseForge API Key
static API_KEY: OnceLock<String> = OnceLock::new();

/// 搜索排序方式
pub enum CurseForgeSortType {
    /// 流行度
    Popularity,
    /// 特性
    Featured,
    /// 上次更新
    LastUpdated,
    /// 名字
    Name,
    /// 下载次数
    TotalDownloads,
}

impl CurseForgeSortType {
    /// 获取排序方式对应编号
    pub fn get_index(&self) -> u32 {
        match self {
            CurseForgeSortType::Featured => 1,
            CurseForgeSortType::Popularity => 2,
            CurseForgeSortType::LastUpdated => 3,
            CurseForgeSortType::Name => 4,
            CurseForgeSortType::TotalDownloads => 6,
        }
    }

    /// 根据排序方式获取排序方向编号
    pub fn get_order_index(&self) -> u32 {
        match self {
            CurseForgeSortType::Featured
            | CurseForgeSortType::Popularity
            | CurseForgeSortType::LastUpdated
            | CurseForgeSortType::TotalDownloads => 1,
            CurseForgeSortType::Name => 0,
        }
    }
}

impl Default for CurseForgeSortType {
    fn default() -> Self {
        CurseForgeSortType::Popularity
    }
}

/// CurseForge 搜索/列表请求参数
pub struct CurseFogreArg {
    /// 项目编号
    pub id: Option<String>,
    /// 游戏版本
    pub version: Option<String>,
    /// 页数
    pub page: Option<u32>,
    /// 过滤
    pub sort: CurseForgeSortType,
    /// 过滤名
    pub filter: Option<String>,
    /// 页大小
    pub page_size: Option<u32>,
    /// 分类
    pub category: Option<String>,
    /// 模组加载器类型
    pub loader: Option<u32>,
}

impl Default for CurseFogreArg {
    fn default() -> Self {
        Self {
            id: Default::default(),
            version: Default::default(),
            page: Default::default(),
            sort: Default::default(),
            filter: Default::default(),
            page_size: Default::default(),
            category: Default::default(),
            loader: Default::default(),
        }
    }
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
///
/// - `req`: 请求内容
async fn send<T: DeserializeOwned>(mut req: reqwest::Request) -> CoreResult<T> {
    req.headers_mut()
        .insert("x-api-key", get_key()?.parse().unwrap());

    let res = crate::get_work_client().send(req).await?;
    crate::handle_response(res).await
}

async fn get_list(
    classid: u32,
    version: &str,
    page: u32,
    sort: u32,
    filter: &str,
    page_size: u32,
    sort_order: u32,
    category: &str,
    mod_loader_type: u32,
) -> CoreResult<CurseForgeListPageObj> {
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
pub async fn get_modpack_list(arg: CurseFogreArg) -> CoreResult<CurseForgeListPageObj> {
    get_list(
        CLASS_MODPACK,
        &arg.version.unwrap_or_default(),
        arg.page.unwrap_or(0),
        arg.sort.get_index(),
        &arg.filter.unwrap_or_default(),
        arg.page_size.unwrap_or(20),
        arg.sort.get_order_index(),
        &arg.category.unwrap_or_default(),
        0,
    )
    .await
}

/// 获取模组列表
pub async fn get_mod_list(arg: CurseFogreArg) -> CoreResult<CurseForgeListPageObj> {
    get_list(
        CLASS_MOD,
        &arg.version.unwrap_or_default(),
        arg.page.unwrap_or(0),
        arg.sort.get_index(),
        &arg.filter.unwrap_or_default(),
        arg.page_size.unwrap_or(20),
        arg.sort.get_order_index(),
        &arg.category.unwrap_or_default(),
        arg.loader.unwrap_or(0),
    )
    .await
}

/// 获取存档列表
pub async fn get_save_list(arg: CurseFogreArg) -> CoreResult<CurseForgeListPageObj> {
    get_list(
        CLASS_SAVES,
        &arg.version.unwrap_or_default(),
        arg.page.unwrap_or(0),
        arg.sort.get_index(),
        &arg.filter.unwrap_or_default(),
        arg.page_size.unwrap_or(20),
        arg.sort.get_order_index(),
        &arg.category.unwrap_or_default(),
        0,
    )
    .await
}

/// 获取资源包列表
pub async fn get_resourcepack_list(arg: CurseFogreArg) -> CoreResult<CurseForgeListPageObj> {
    get_list(
        CLASS_RESOURCEPACKS,
        &arg.version.unwrap_or_default(),
        arg.page.unwrap_or(0),
        arg.sort.get_index(),
        &arg.filter.unwrap_or_default(),
        arg.page_size.unwrap_or(20),
        arg.sort.get_order_index(),
        &arg.category.unwrap_or_default(),
        0,
    )
    .await
}

/// 获取数据包列表
pub async fn get_datapacks_list(arg: CurseFogreArg) -> CoreResult<CurseForgeListPageObj> {
    get_list(
        CLASS_RESOURCEPACKS,
        &arg.version.unwrap_or_default(),
        arg.page.unwrap_or(0),
        arg.sort.get_index(),
        &arg.filter.unwrap_or_default(),
        arg.page_size.unwrap_or(20),
        arg.sort.get_order_index(),
        &CATEGORYID_DATAPACKS.to_string(),
        0,
    )
    .await
}

/// 获取光影包列表
pub async fn get_shaders_list(arg: CurseFogreArg) -> CoreResult<CurseForgeListPageObj> {
    get_list(
        CLASS_SHADERPACKS,
        &arg.version.unwrap_or_default(),
        arg.page.unwrap_or(0),
        arg.sort.get_index(),
        &arg.filter.unwrap_or_default(),
        arg.page_size.unwrap_or(20),
        arg.sort.get_order_index(),
        "",
        0,
    )
    .await
}

/// 获取模组信息
///
/// - `pid`: 项目编号
/// - `fid`: 文件编号
pub async fn get_mod(pid: &str, fid: &str) -> CoreResult<CurseForgeFileObj> {
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
pub async fn get_files(ids: Vec<u64>) -> CoreResult<Vec<CurseForgeFileDataObj>> {
    let obj = CurseForgeGetFilesObj { file_ids: ids };

    let url = format!("{}mods/files", urls::CURSEFORGE);

    let mut req = reqwest::Request::new(Method::POST, Url::parse(&url).unwrap());

    json(&mut req, &obj)?;

    send(req).await
}

/// 获取分类信息
pub async fn get_categories() -> CoreResult<CurseForgeCategoriesObj> {
    let url = format!("{}categories?gameId={}", urls::CURSEFORGE, GAME_ID);

    let req = reqwest::Request::new(Method::GET, Url::parse(&url).unwrap());

    send(req).await
}

/// 获取版本信息
pub async fn get_version() -> CoreResult<CurseForgeVersionObj> {
    let url = format!("{}games/{}/versions", urls::CURSEFORGE, GAME_ID);

    let req = reqwest::Request::new(Method::GET, Url::parse(&url).unwrap());

    send(req).await
}

/// 获取版本信息
pub async fn get_version_type() -> CoreResult<CurseForgeVersionTypeObj> {
    let url = format!("{}games/{}/version-types", urls::CURSEFORGE, GAME_ID);

    let req = reqwest::Request::new(Method::GET, Url::parse(&url).unwrap());

    send(req).await
}

/// 获取版本信息
pub async fn get_mod_info(id: &str) -> CoreResult<CurseForgeListObj> {
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
pub async fn get_mods_info(ids: Vec<u64>) -> CoreResult<CurseForgeListPageObj> {
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
pub async fn get_files_page(arg: CurseFogreArg) -> CoreResult<CurseFogreFilePageObj> {
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

impl CurseForgeFileDataObj {
    /// 修正下载地址
    pub fn fix_download_url(&mut self) {
        if self.download_url.is_none() {
            self.download_url = Some(format!(
                "{}files/{}/{}/{}",
                urls::CURSEFORGE_DOWNLOAD,
                self.id / 1000,
                self.id % 1000,
                self.file_name
            ))
        }
    }

    /// 提取 SHA1 哈希值
    #[inline]
    pub fn sha1_hash(&self) -> String {
        self.hashes
            .iter()
            .find(|h| h.algo == 1)
            .map(|h| h.value.clone())
            .unwrap_or_default()
    }
}
