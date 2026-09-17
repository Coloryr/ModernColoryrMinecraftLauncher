use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileListDto {
    pub list: Vec<FileListItemDto>,
    /// 项目总数
    pub count: u64,
    /// 项目名字
    pub name: String,
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
    pub id_download: bool,
    /// 项目名字
    pub project_name: String,
    /// 下载源信息
    pub source: SourceTypeDto,
}

impl FileListItemDto {
    pub fn new_curseforge(data: &CurseForgeFileDataObj) -> Self {

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
    pub url: String,
    pub sha1: Option<String>,
    pub sha512: Option<String>,
}
