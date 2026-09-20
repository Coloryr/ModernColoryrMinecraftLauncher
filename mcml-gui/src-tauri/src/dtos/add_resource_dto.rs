use mcml_game::launcher::ModPackType;
use mcml_net::{
    curseforge_api::{file_obj::CurseForgeFileDataObj, list_obj::CurseForgeListDataObj},
    modrinth_api::version_obj::ModrinthVersionObj,
};
use serde::{Deserialize, Serialize};

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
    /// 下载源信息
    pub source: SourceTypeDto,
}

impl FileListItemDto {
    pub fn new_curseforge(
        data: &CurseForgeFileDataObj,
        file_type: String,
        is_download: bool,
    ) -> Self {
        Self {
            name: data.display_name.clone(),
            download: data.download_count,
            size: data.file_length,
            is_download,
            time: data.file_date.clone(),
            source: SourceTypeDto {
                file_type: file_type,
                source: ModPackType::CurseForge.to_string(),
                pid: data.mod_id.to_string(),
                fid: data.id.to_string(),
            },
        }
    }

    pub fn new_modrinth(data: &ModrinthVersionObj, file_type: String, is_download: bool) -> Self {
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
            source: SourceTypeDto {
                file_type,
                source: ModPackType::Modrinth.to_string(),
                pid: data.id.clone(),
                fid: data.project_id.clone(),
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
    pub image: String,
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
        file_type: String,
        download: bool,
        can_star: bool,
        is_star: bool,
        mcmod: Option<McmodDto>,
    ) -> Self {
        let mut authors = Vec::new();
        for item in data.authors.iter() {
            authors.push(PicDto {
                name: item.name.clone(),
                logo: item.avatar_url.clone(),
            });
        }

        let mut tag = Vec::new();
        for item in data.categories.iter() {
            tag.push(TagDto {
                name: item.name.clone(),
                logo: item.icon_url.clone(),
                svg: None,
            });
        }

        let mut screenshots = Vec::new();
        for item in data.screenshots.iter() {
            screenshots.push(DecPicDto {
                name: item.title.clone(),
                logo: item.url.clone(),
                description: item.description.clone(),
            });
        }

        Self {
            name: data.name.clone(),
            summary: data.summary.clone(),
            image: data.logo.url.clone(),
            authors,
            tag,
            screenshots,
            download_count: data.download_count,
            date: data.date_modified.clone(),
            download,
            can_star,
            is_star,
            download_now: false,
            url: data.links.website_url.clone(),
            mcmod,
            source: SourceTypeDto {
                file_type,
                source: ModPackType::CurseForge.to_string(),
                pid: data.id.to_string(),
                fid: Default::default(),
            },
        }
    }
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
    pub logo: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TagDto {
    pub name: String,
    pub logo: String,
    pub svg: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DecPicDto {
    pub name: String,
    pub logo: String,
    pub description: String,
}
