use mcml_game::launcher::{FileType, ModPackType};
use mcml_game::modrinth;
use mcml_names::i18_items::error_type::CoreResult;
use mcml_net::{
    curseforge_api::{file_obj::CurseForgeFileDataObj, list_obj::CurseForgeListDataObj},
    modrinth_api::{self, project_obj::ModrinthProjectObj, search_obj::HitObj, version_obj::ModrinthVersionObj},
    urls,
};
use serde::{Deserialize, Serialize};

use crate::image_manager::{self};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileListDto {
    pub list: Vec<FileListItemDto>,
    /// 项目总数
    pub count: u64,
    /// 项目名字
    pub name: String,
    /// 最大页
    pub max_page: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileListItemDto {
    /// 名字
    pub name: String,
    /// 下载次数
    pub download: u64,
    /// 文件大小
    pub size: u64,
    /// 时间
    pub time: String,
    /// 是否已经下载
    pub is_download: bool,
    /// 是否正在下载
    pub download_now: bool,
    /// 下载源信息
    pub source: SourceTypeDto,
}

impl FileListItemDto {
    pub fn new_curseforge(
        data: &CurseForgeFileDataObj,
        file_type: FileType,
        is_download: bool,
        download_now: bool,
    ) -> Self {
        Self {
            name: data.display_name.clone(),
            download: data.download_count,
            size: data.file_length,
            is_download,
            time: data.file_date.clone(),
            download_now,
            source: SourceTypeDto {
                file_type: file_type.to_string(),
                source: ModPackType::CurseForge.to_string(),
                pid: data.mod_id.to_string(),
                fid: data.id.to_string(),
            },
        }
    }

    pub fn new_modrinth(
        data: &ModrinthVersionObj,
        file_type: FileType,
        is_download: bool,
        download_now: bool,
    ) -> Self {
        let file = data
            .files
            .iter()
            .filter(|item| item.primary)
            .next()
            .unwrap_or(data.files.first().unwrap());
        Self {
            name: data.name.clone(),
            download: data.downloads,
            size: file.size,
            time: data.date_published.clone(),
            is_download,
            download_now,
            source: SourceTypeDto {
                file_type: file_type.to_string(),
                source: ModPackType::Modrinth.to_string(),
                // pid 是项目、fid 是文件（版本号），与 CurseForge 侧一致；
                // 反了会让 add_modpack_install 拿版本号当项目号去查
                pid: data.project_id.clone(),
                fid: data.id.clone(),
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceTypeDto {
    /// 资源类型
    pub file_type: String,
    /// 下载源
    pub source: String,
    /// 项目ID
    pub pid: String,
    /// 文件ID
    pub fid: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectDto {
    /// 项目列表
    pub items: Vec<ProjectItemDto>,
    /// 项目总数
    pub count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectItemDto {
    /// 名字
    pub name: String,
    /// 描述
    pub summary: String,
    /// 图片地址 从image_manager加载
    pub image: Option<String>,
    /// 作者
    pub authors: Vec<PicDto>,
    /// 标签
    pub tag: Vec<TagDto>,
    /// 截图
    pub screenshots: Vec<DecPicDto>,
    /// 下载次数
    pub download_count: u64,
    /// 更新时间
    pub date: String,
    /// 是否已经下载
    pub download: bool,
    /// 能否收藏
    pub can_star: bool,
    /// 是否已经收藏
    pub is_star: bool,
    /// 是否正在下载
    pub download_now: bool,
    /// 跳转网址
    pub url: String,
    /// 百科翻译
    pub mcmod: Option<McmodDto>,
    /// 下载源信息
    pub source: SourceTypeDto,
}

impl ProjectItemDto {
    pub fn new_curseforge(
        data: &CurseForgeListDataObj,
        file_type: FileType,
        download: bool,
        can_star: bool,
        is_star: bool,
        download_now: bool,
        mcmod: Option<McmodDto>,
    ) -> Self {
        let mut authors = Vec::new();
        for item in data.authors.iter() {
            authors.push(PicDto {
                name: item.name.clone(),
                logo: item
                    .avatar_url
                    .as_ref()
                    .map(|item| image_manager::push_image_url(item)),
            });
        }

        let mut tag = Vec::new();
        for item in data.categories.iter() {
            tag.push(TagDto {
                name: item.name.clone(),
                logo: Some(image_manager::push_image_url(&item.icon_url)),
                svg: None,
            });
        }

        let mut screenshots = Vec::new();
        for item in data.screenshots.iter() {
            screenshots.push(DecPicDto {
                name: item.title.clone(),
                logo: image_manager::push_image_url(&item.url),
                description: item.description.clone(),
            });
        }

        Self {
            name: data.name.clone(),
            summary: data.summary.clone(),
            image: data
                .logo
                .url
                .as_ref()
                .map(|item| image_manager::push_image_url(item)),
            authors,
            tag,
            screenshots,
            download_count: data.download_count,
            date: data.date_modified.clone(),
            download,
            can_star,
            is_star,
            download_now,
            url: data.links.website_url.clone(),
            mcmod,
            source: SourceTypeDto {
                file_type: file_type.to_string(),
                source: ModPackType::CurseForge.to_string(),
                pid: data.id.to_string(),
                fid: Default::default(),
            },
        }
    }

    /// 列表项只从 HitObj 构造，不发 project / team 请求（对齐旧 C# GetModPackListAsync）；
    /// 作者头像 / 截图在列表里留空，等项目详情（GetFileItemAsync）再补。
    /// 分类 svg 来自 OnceLock 缓存的 /tag/category，逐条取没有额外网络请求
    pub async fn new_modrinth(
        data: &HitObj,
        file_type: FileType,
        download: bool,
        can_star: bool,
        is_star: bool,
        download_now: bool,
        mcmod: Option<McmodDto>,
    ) -> Self {
        let authors = vec![PicDto {
            name: data.author.clone(),
            logo: None,
        }];

        let mut tag = Vec::new();

        for item in data.categories.iter() {
            tag.push(TagDto {
                name: item.clone(),
                logo: None,
                svg: modrinth::get_categories_icon(item).await,
            });
        }

        Self {
            name: data.title.clone(),
            summary: data.description.clone(),
            image: data
                .icon_url
                .as_ref()
                .map(|item| image_manager::push_image_url(item)),
            authors,
            tag,
            screenshots: Vec::new(),
            download_count: data.downloads,
            date: data.date_modified.clone(),
            download,
            can_star,
            is_star,
            download_now,
            url: get_modrinth_url(&file_type, &data.project_id),
            mcmod,
            source: SourceTypeDto {
                file_type: file_type.to_string(),
                source: ModPackType::Modrinth.to_string(),
                pid: data.project_id.clone(),
                fid: Default::default(),
            },
        }
    }
}

/// 项目详情（双击列表项弹出）：简介 + 正文 + 作者 + 标签 + 截图
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectDetailDto {
    /// 简介
    pub summary: String,
    /// 正文（Modrinth 的 body，markdown 文本）；CurseForge 没有正文，只有简介
    pub body: Option<String>,
    /// 作者
    pub authors: Vec<PicDto>,
    /// 标签
    pub tag: Vec<TagDto>,
    /// 截图
    pub screenshots: Vec<DecPicDto>,
}

impl ProjectDetailDto {
    /// Modrinth 详情：project 拿正文 / 截图，team 拿作者头像
    pub async fn new_modrinth(data: &ModrinthProjectObj) -> CoreResult<Self> {
        let mut authors = Vec::new();

        let team = modrinth_api::get_team(&data.id).await?;

        for item in team {
            authors.push(PicDto {
                name: item.user.username,
                logo: item
                    .user
                    .avatar_url
                    .map(|item| image_manager::push_image_url(&item)),
            });
        }

        let mut tag = Vec::new();

        for item in data.loaders.iter().chain(data.categories.iter()) {
            tag.push(TagDto {
                name: item.clone(),
                logo: None,
                svg: modrinth::get_categories_icon(item).await,
            });
        }

        let mut gallery: Vec<_> = data.gallery.iter().collect();
        gallery.sort_by(|a, b| a.ordering.cmp(&b.ordering));

        let mut screenshots = Vec::new();
        for item in gallery {
            screenshots.push(DecPicDto {
                name: item.title.clone().unwrap_or_default(),
                logo: image_manager::push_image_url(&item.raw_url),
                description: item.description.clone().unwrap_or_default(),
            });
        }

        Ok(Self {
            summary: data.description.clone(),
            body: Some(data.body.clone()),
            authors,
            tag,
            screenshots,
        })
    }

    /// CurseForge 详情：只有简介（数据来自列表缓存或 mod_info，没有正文）
    pub fn new_curseforge(data: &CurseForgeListDataObj) -> Self {
        let mut authors = Vec::new();
        for item in data.authors.iter() {
            authors.push(PicDto {
                name: item.name.clone(),
                logo: item
                    .avatar_url
                    .as_ref()
                    .map(|item| image_manager::push_image_url(item)),
            });
        }

        let mut tag = Vec::new();
        for item in data.categories.iter() {
            tag.push(TagDto {
                name: item.name.clone(),
                logo: Some(image_manager::push_image_url(&item.icon_url)),
                svg: None,
            });
        }

        let mut screenshots = Vec::new();
        for item in data.screenshots.iter() {
            screenshots.push(DecPicDto {
                name: item.title.clone(),
                logo: image_manager::push_image_url(&item.url),
                description: item.description.clone(),
            });
        }

        Self {
            summary: data.summary.clone(),
            body: None,
            authors,
            tag,
            screenshots,
        }
    }
}

fn get_modrinth_url(file_type: &FileType, id: &str) -> String {
    format!(
        "{}{}/{id}",
        urls::MODRINTH,
        match file_type {
            FileType::Modpack => "modpack",
            FileType::Shaderpack => "shaders",
            FileType::Resourcepack => "resourcepacks",
            FileType::DataPacks => "datapacks",
            _ => "mod",
        }
    )
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McmodDto {
    pub mcmod_id: String,
    pub mcmod_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PicDto {
    pub name: String,
    pub logo: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TagDto {
    pub name: String,
    pub logo: Option<String>,
    pub svg: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DecPicDto {
    pub name: String,
    pub logo: String,
    pub description: String,
}

/// 实例存档条目（数据包安装时选择目标存档用）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceSaveDto {
    /// 世界名（level.dat 里的 LevelName）
    pub name: String,
    /// 存档文件夹名（saves/ 下的目录名）
    pub dir: String,
}

/// 资源下载任务条目（添加资源窗口顶部进度条）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceTaskDto {
    /// 项目 ID
    pub pid: String,
    /// 文件 ID
    pub fid: String,
    /// 显示名
    pub name: String,
    /// 下载进度（0–100）
    pub progress: f64,
    /// 下载完成
    pub done: bool,
    /// 下载失败
    pub failed: bool,
}

/// 资源下载任务总览（add-resource-status 事件负载 / add_resource_status 返回）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceStatusDto {
    /// 添加资源窗口是否打开（关闭时由主窗口显示进度条）
    pub window_open: bool,
    /// 任务列表
    pub tasks: Vec<ResourceTaskDto>,
}
