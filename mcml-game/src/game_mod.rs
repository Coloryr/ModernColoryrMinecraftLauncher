use std::{
    collections::HashSet,
    fs::File,
    io::Read,
    path::{Path, PathBuf},
    sync::Mutex,
};

use mcml_base::{
    file_item::FileHash,
    hash_helper::{self, HashType},
    path_helper,
};
use mcml_names::{
    i18_items::error_type::{CoreResult, ErrorData, ErrorType, FileSystemErrorData},
    names,
};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use serde_json::Value;
use zip::{ZipArchive, read::ZipFile};

use crate::{launcher::instance_setting_obj::InstanceSettingObj, loader::LoaderType};

pub enum LoadSideType {
    Client,
    Server,
    Both,
}

impl Default for LoadSideType {
    fn default() -> Self {
        LoadSideType::Both
    }
}

pub struct ModItemObj {
    /// ID
    pub mod_id: String,
    /// 名字
    pub name: String,
    /// 描述
    pub description: Option<String>,
    /// 版本号
    pub version: Option<String>,
    /// 作者
    pub author: HashSet<String>,
    /// 依赖
    pub dependants: HashSet<String>,
    /// 支持的加载器
    pub loaders: HashSet<LoaderType>,
    /// 加载测
    pub side: LoadSideType,
    /// 网站
    pub url: String,
    /// 图标
    pub icon: Vec<u8>,
}

/// 模组信息
pub struct ModObj {
    pub info: Vec<ModItemObj>,
    /// 是否被禁用
    pub disable: bool,
    /// 是否为Core模组
    pub core: bool,
    /// 校验
    pub hash: FileHash,
    /// 内置的模组
    pub jar_in_jar: Vec<ModObj>,
    /// 是否读取失败
    pub fail: bool,
    /// 文件路径
    pub file: PathBuf,
}

fn read_forge(mut zip: ZipFile<'_, File>) -> CoreResult<ModObj> {
    let mut json = String::new();
    zip.read_to_string(&mut json).map_err(|err| {
        ErrorType::ArchiveReadError(ErrorData {
            error: err.to_string(),
        })
    })?;

    let obj = serde_json::from_str::<Value>(&json).map_err(|err| {
        ErrorType::JsonError(ErrorData {
            error: err.to_string(),
        })
    })?;

    let mut mod_info = ModObj {
        info: Default::default(),
        disable: Default::default(),
        core: Default::default(),
        hash: Default::default(),
        jar_in_jar: Default::default(),
        fail: Default::default(),
        file: Default::default(),
    };

    if obj.is_array() {
        let list = obj.as_array().unwrap();

        for item in list.iter() {

        }
    }

    todo!()
}

fn read_mod_info<P: AsRef<Path>>(path: P) -> CoreResult<ModObj> {
    let mut is_test = false;

    let file = path_helper::open_read(&path)?;
    let mut zip = ZipArchive::new(file).map_err(|err| {
        ErrorType::ArchiveOpenError(FileSystemErrorData {
            path: path.as_ref().to_path_buf(),
            error: err.to_string(),
        })
    })?;

    let file = zip.by_name(names::MC_MOD_INFO_FILE);
    if let Ok(item) = file {
        read_forge(item);
    }

    todo!()
}

fn read_mod<P: AsRef<Path>>(path: P, sha256: bool) -> CoreResult<ModObj> {
    let sha1 = hash_helper::gen_hash_from_file(HashType::Sha1, path.as_ref())?;

    let hash = if sha256 {
        let sha256 = hash_helper::gen_hash_from_file(HashType::Sha256, path.as_ref())?;
        FileHash::Sha1Sha256(sha1, sha256)
    } else {
        FileHash::Sha1(sha1.clone())
    };

    let mut mod_info = read_mod_info(path)?;

    mod_info.hash = hash;

    Ok(mod_info)
}

impl InstanceSettingObj {
    /// 扫描模组
    pub async fn read_mod_fast(&self) -> Vec<ModObj> {
        let dir = self.get_mods_path();
        let files = path_helper::get_files(dir);

        tokio::task::spawn_blocking(move || {
            let list = Mutex::new(Vec::new());
            files.par_iter().for_each(|item| {
                if let Some(ext) = item.extension()
                    && (ext.eq_ignore_ascii_case(names::JAR_EXT)
                        || ext.eq_ignore_ascii_case(names::DISABLE_EXT)
                        || ext.eq_ignore_ascii_case(names::DISABLED_EXT))
                {
                    let disable = ext.eq_ignore_ascii_case(names::DISABLE_EXT)
                        || ext.eq_ignore_ascii_case(names::DISABLED_EXT);
                    if let Ok(hash) = hash_helper::gen_hash_from_file(HashType::Sha1, item) {
                        list.lock().unwrap().push(ModObj {
                            info: Default::default(),
                            disable,
                            core: Default::default(),
                            hash: FileHash::Sha1(hash),
                            file: item.clone(),
                            jar_in_jar: Default::default(),
                            fail: false,
                        });
                    } else {
                        list.lock().unwrap().push(ModObj {
                            info: Default::default(),
                            disable,
                            core: Default::default(),
                            hash: Default::default(),
                            file: item.clone(),
                            jar_in_jar: Default::default(),
                            fail: true,
                        });
                    }
                }
            });
            list.into_inner().unwrap()
        })
        .await
        .unwrap_or_default()
    }

    /// 读取模组列表
    /// - `sha256`: 是否计算SHA256
    pub async fn read_mod(&self, sha256: bool) -> Vec<ModObj> {
        let dir = self.get_mods_path();
        let files = path_helper::get_files(dir);

        tokio::task::spawn_blocking(move || {
            let list = Mutex::new(Vec::new());
            files.par_iter().for_each(|item| {
                if let Some(ext) = item.extension()
                    && (ext.eq_ignore_ascii_case(names::JAR_EXT)
                        || ext.eq_ignore_ascii_case(names::DISABLE_EXT)
                        || ext.eq_ignore_ascii_case(names::DISABLED_EXT))
                {
                    let disable = ext.eq_ignore_ascii_case(names::DISABLE_EXT)
                        || ext.eq_ignore_ascii_case(names::DISABLED_EXT);

                    let mod_info = read_mod(item, sha256);
                    match mod_info {
                        Ok(mut item) => {
                            item.disable = disable;
                            list.lock().unwrap().push(item);
                        }
                        Err(err) => {
                            list.lock().unwrap().push(ModObj {
                                info: Default::default(),
                                disable,
                                core: Default::default(),
                                hash: Default::default(),
                                jar_in_jar: Default::default(),
                                fail: true,
                                file: item.clone(),
                            });

                            mcml_log::error_type(err);
                        }
                    }
                }
            });
            list.into_inner().unwrap()
        })
        .await
        .unwrap_or_default()
    }
}
