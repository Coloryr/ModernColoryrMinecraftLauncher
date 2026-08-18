//! 文件下载项目定义
//!
//! 定义下载文件相关的核心数据结构，包括哈希类型、下载后处理操作和文件条目。

use std::{
    io::{Seek, SeekFrom},
    path::PathBuf,
};

use mcml_sys::path_helper;

use crate::hash_helper::{self, HashType};

/// 文件校验
#[derive(Debug, Clone)]
pub enum FileHash {
    None,
    Md5(String),
    Sha1(String),
    Sha256(String),
    Sha512(String),
    Sha1Sha256(String, String),
    Sha1Sha512(String, String),
}

impl Default for FileHash {
    fn default() -> Self {
        FileHash::None
    }
}

/// 下载后运行
#[derive(Debug, Clone)]
pub enum LaterRun {
    None,
    /// 解压
    UnpackNative(PathBuf),
    /// 存档
    UnpackSave(PathBuf),
}

impl Default for LaterRun {
    fn default() -> Self {
        LaterRun::None
    }
}

impl FileHash {
    /// 检查是否符合校验值
    ///
    /// - `check`: 目标校验值
    pub fn eq(&self, check: &str) -> bool {
        match self {
            FileHash::None => true,
            FileHash::Md5(hash) => hash == check,
            FileHash::Sha1(hash) => hash == check,
            FileHash::Sha256(hash) => hash == check,
            FileHash::Sha1Sha256(hash1, hash2) => hash1 == check || hash2 == check,
            FileHash::Sha512(hash) => hash == check,
            FileHash::Sha1Sha512(hash1, hash2) => hash1 == check || hash2 == check,
        }
    }

    /// 获取SHA1
    pub fn get_sha1(&self) -> Option<String> {
        match self {
            FileHash::None => None,
            FileHash::Md5(_) => None,
            FileHash::Sha1(sha1) => Some(sha1.clone()),
            FileHash::Sha256(_) => None,
            FileHash::Sha512(_) => None,
            FileHash::Sha1Sha256(sha1, _) => Some(sha1.clone()),
            FileHash::Sha1Sha512(sha1, _) => Some(sha1.clone()),
        }
    }
}

/// 一个文件项目
#[derive(Debug, Clone)]
pub struct FileItemObj {
    /// 名字
    pub name: String,
    /// 文件位置
    pub file: PathBuf,
    /// 下载地址
    pub url: String,
    /// 文件校验
    pub hash: FileHash,
    /// 后续执行内容
    pub later: LaterRun,
}

impl Default for FileItemObj {
    fn default() -> Self {
        Self {
            name: Default::default(),
            file: Default::default(),
            url: Default::default(),
            hash: Default::default(),
            later: Default::default(),
        }
    }
}

impl FileItemObj {
    /// 检查文件是否正常
    pub fn check_hash(&self) -> bool {
        if self.file.exists() && self.file.is_file() {
            if let Ok(mut stream) = path_helper::open_read(&self.file) {
                match &self.hash {
                    FileHash::None => true,
                    FileHash::Md5(md5) => {
                        if let Ok(hash) =
                            hash_helper::gen_hash_from_reader(HashType::Md5, &mut stream)
                        {
                            hash.eq_ignore_ascii_case(md5)
                        } else {
                            false
                        }
                    }
                    FileHash::Sha1(sha1) => {
                        if let Ok(hash) =
                            hash_helper::gen_hash_from_reader(HashType::Sha1, &mut stream)
                        {
                            hash.eq_ignore_ascii_case(sha1)
                        } else {
                            false
                        }
                    }
                    FileHash::Sha256(sha256) => {
                        if let Ok(hash) =
                            hash_helper::gen_hash_from_reader(HashType::Sha256, &mut stream)
                        {
                            hash.eq_ignore_ascii_case(sha256)
                        } else {
                            false
                        }
                    }
                    FileHash::Sha1Sha256(sha1, sha256) => {
                        if let Ok(hash) =
                            hash_helper::gen_hash_from_reader(HashType::Sha1, &mut stream)
                            && hash.eq_ignore_ascii_case(sha1)
                        {
                            stream.seek(SeekFrom::Start(0)).unwrap();
                            if let Ok(hash) =
                                hash_helper::gen_hash_from_reader(HashType::Sha256, &mut stream)
                            {
                                hash.eq_ignore_ascii_case(sha256)
                            } else {
                                false
                            }
                        } else {
                            false
                        }
                    }
                    FileHash::Sha512(sha512) => {
                        if let Ok(hash) =
                            hash_helper::gen_hash_from_reader(HashType::Sha512, &mut stream)
                        {
                            hash.eq_ignore_ascii_case(sha512)
                        } else {
                            false
                        }
                    }
                    FileHash::Sha1Sha512(sha1, sha512) => {
                        if let Ok(hash) =
                            hash_helper::gen_hash_from_reader(HashType::Sha1, &mut stream)
                            && hash.eq_ignore_ascii_case(sha1)
                        {
                            stream.seek(SeekFrom::Start(0)).unwrap();
                            if let Ok(hash) =
                                hash_helper::gen_hash_from_reader(HashType::Sha512, &mut stream)
                            {
                                hash.eq_ignore_ascii_case(sha512)
                            } else {
                                false
                            }
                        } else {
                            false
                        }
                    }
                }
            } else {
                false
            }
        } else {
            false
        }
    }
}
