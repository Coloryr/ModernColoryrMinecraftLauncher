use std::path::PathBuf;

/// 文件校验
#[derive(Debug, Clone)]
pub enum FileHash {
    None,
    Md5(String),
    Sha1(String),
    Sha256(String),
    Sha1Sha256(String, String),
}

impl FileHash {
    /// 检查是否符合校验值
    /// - `check`: 目标校验值
    pub fn eq(&self, check: &str) -> bool {
        match self {
            FileHash::None => true,
            FileHash::Md5(hash) => hash == check,
            FileHash::Sha1(hash) => hash == check,
            FileHash::Sha256(hash) => hash == check,
            FileHash::Sha1Sha256(hash1, hash2) => hash1 == check || hash2 == check,
        }
    }
}

/// 一个文件项目
#[derive(Debug, Clone)]
pub struct FileItemObj {
    /// 名字
    pub name: String,
    /// 文件位置
    pub local: PathBuf,
    /// 下载地址
    pub url: String,
    /// 文件校验
    pub hash: FileHash,
}
