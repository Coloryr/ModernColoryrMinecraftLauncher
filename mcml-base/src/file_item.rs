use std::path::PathBuf;

/// 一个文件项目
pub struct FileItemObj {
    /// 名字
    pub name: String,
    /// 文件位置
    pub local: PathBuf,
    /// 下载地址
    pub url: String,
    /// 文件校验
    pub sha1: String,
}
