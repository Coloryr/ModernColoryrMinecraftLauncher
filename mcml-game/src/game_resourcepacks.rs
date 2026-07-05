use std::{
    cmp,
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
use tokio_util::io::simplex::new;
use zip::ZipArchive;

use crate::launcher::instance_setting_obj::InstanceSettingObj;

/// 资源包信息
pub struct ResourcepackObj {
    /// 简介
    pub description: String,
    /// 版本号
    pub pack_format: i64,
    /// 最小版本
    pub min_format: i64,
    /// 最大版本号
    pub max_format: i64,
    /// 文件校验
    pub hash: FileHash,
    /// 路径
    pub local: PathBuf,
    /// 图标
    pub icon: Option<Vec<u8>>,
    /// 是否读取失败
    pub fail: bool,
}

impl Default for ResourcepackObj {
    fn default() -> Self {
        Self {
            description: Default::default(),
            pack_format: Default::default(),
            min_format: Default::default(),
            max_format: Default::default(),
            hash: Default::default(),
            local: Default::default(),
            icon: Default::default(),
            fail: Default::default(),
        }
    }
}

fn process_resourcepack<P: AsRef<Path>>(path: P) -> CoreResult<ResourcepackObj> {
    let file = path_helper::open_read(&path)?;
    let mut zip = ZipArchive::new(file).map_err(|err| {
        ErrorType::ArchiveOpenError(FileSystemErrorData {
            path: path.as_ref().to_path_buf(),
            error: err.to_string(),
        })
    })?;

    let meta = zip.by_name(names::PACK_META_FILE).map_err(|err| {
        ErrorType::ArchiveReadError(ErrorData {
            error: err.to_string(),
        })
    })?;

    let json = serde_json::from_reader::<_, Value>(meta).map_err(|err| {
        ErrorType::StreamError(ErrorData {
            error: err.to_string(),
        })
    })?;

    let mut pack = ResourcepackObj::default();

    // 读取json
    match json {
        Value::Object(map) => {
            if let Some(Value::Object(objs)) = map.get("pack") {
                let format = objs.get("pack_format");
                let min_format = objs.get("min_format");
                let max_format = objs.get("max_format");
                let description = objs.get("description");

                if let Some(Value::Number(value)) = format {
                    pack.pack_format = value.as_i64().unwrap_or(0);
                }
                if let Some(Value::Number(value)) = min_format {
                    pack.min_format = value.as_i64().unwrap_or(0);
                }
                if let Some(Value::Number(value)) = max_format {
                    pack.max_format = value.as_i64().unwrap_or(0);
                }
                if let Some(Value::String(value)) = description {
                    pack.description = value.clone();
                }

                if let Some(Value::Array(list)) = min_format {
                    let mut min = 0i64;
                    if let Some(item) = list.iter().next() {
                        if item.is_number() {
                            min = item.as_i64().unwrap_or(0);
                        }
                    }
                    for item in list {
                        min = cmp::min(min, item.as_i64().unwrap_or(0));
                    }

                    pack.min_format = min;
                }
                if let Some(Value::Array(list)) = max_format {
                    let mut max = 0i64;
                    if let Some(item) = list.iter().next() {
                        if item.is_number() {
                            max = item.as_i64().unwrap_or(0);
                        }
                    }
                    for item in list {
                        max = cmp::max(max, item.as_i64().unwrap_or(0));
                    }

                    pack.max_format = max;
                }
            }
        }
        _ => {
            pack.fail = true;
        }
    }

    // 读取图标
    let icon = zip.by_name(names::PACK_ICON_FILE);
    if let Ok(mut icon) = icon {
        let mut vec = Vec::new();
        icon.read_to_end(&mut vec).map_err(|err| {
            ErrorType::ArchiveReadError(ErrorData {
                error: err.to_string(),
            })
        })?;
        pack.icon = Some(vec);
    }

    Ok(pack)
}

impl InstanceSettingObj {
    /// 获取资源包列表
    pub async fn get_resourcepacks(&self) -> Vec<ResourcepackObj> {
        let dir = self.get_resourcepacks_path();
        if !dir.exists() || !dir.is_dir() {
            Default::default()
        } else {
            let files = path_helper::get_files(&dir);

            tokio::task::spawn_blocking(move || {
                let list = Mutex::new(Vec::new());

                files.par_iter().for_each(|item| {
                    let sha1 = hash_helper::gen_hash_from_file(HashType::Sha1, item);
                    let sha256 = hash_helper::gen_hash_from_file(HashType::Sha256, item);

                    if sha1.is_err() || sha256.is_err() {
                        return;
                    }

                    let hash = FileHash::Sha1Sha256(sha1.unwrap(), sha256.unwrap());

                    // 如果是压缩包
                    if let Some(ext) = item.extension()
                        && ext.eq_ignore_ascii_case(names::ZIP_EXT)
                    {
                        match process_resourcepack(item) {
                            Ok(mut obj) => {
                                obj.hash = hash;
                                obj.local = item.clone();

                                list.lock().unwrap().push(obj);
                                return;
                            }
                            Err(err) => {
                                mcml_log::error_type(err);
                            }
                        }
                    }

                    list.lock().unwrap().push(ResourcepackObj {
                        description: Default::default(),
                        pack_format: Default::default(),
                        min_format: Default::default(),
                        max_format: Default::default(),
                        hash,
                        local: item.clone(),
                        icon: Default::default(),
                        fail: true,
                    });
                });

                list.into_inner().unwrap()
            })
            .await
            .unwrap_or_default()
        }
    }
}
