//! 游戏实例资源包相关
use std::{
    io::Read,
    path::{Path, PathBuf},
    sync::Mutex,
};

use mml_base::{
    file_item::FileHash,
    hash_helper::{self, HashType},
    serialize_tools,
};
use mml_names::{
    i18_items::error_type::{CoreResult, ErrorData, ErrorType, FileSystemErrorData},
    names,
};
use mml_sys::path_helper;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use serde::Deserialize;
use zip::ZipArchive;

use crate::launcher::instance_setting_obj::InstanceSettingObj;

/// pack.mcmeta 的反序列化结构体
#[derive(Deserialize)]
struct PackMeta {
    /// pack 信息段
    pack: PackInfo,
}

/// pack 信息段
#[derive(Deserialize)]
struct PackInfo {
    /// 资源包格式版本
    #[serde(
        default,
        deserialize_with = "serialize_tools::deserialize_number_or_max"
    )]
    pack_format: i64,
    /// 支持的最小格式版本
    #[serde(
        default,
        deserialize_with = "serialize_tools::deserialize_number_or_min"
    )]
    min_format: i64,
    /// 支持的最大格式版本
    #[serde(
        default,
        deserialize_with = "serialize_tools::deserialize_number_or_max"
    )]
    max_format: i64,
    /// 简介文字
    #[serde(default)]
    description: String,
}

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
    pub path: PathBuf,
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
            path: Default::default(),
            icon: Default::default(),
            fail: Default::default(),
        }
    }
}

/// 解析材质包
///
/// # 参数
///
/// - `path`: 材质包文件路径（zip）
///
/// # 返回值
///
/// 返回材质包信息；打开或读取失败返回对应错误
pub fn process_resourcepack<P: AsRef<Path>>(path: P) -> CoreResult<ResourcepackObj> {
    let file = path_helper::open_read(&path)?;
    let mut zip = ZipArchive::new(file).map_err(|err| {
        ErrorType::ArchiveOpenError(FileSystemErrorData {
            path: path.as_ref().to_path_buf(),
            error: err.to_string(),
        })
    })?;

    // 解析 pack.mcmeta
    let mut pack = {
        let meta = zip.by_name(names::PACK_META_FILE).map_err(|err| {
            ErrorType::ArchiveReadError(ErrorData {
                error: err.to_string(),
            })
        })?;

        match serialize_tools::json_from_stream::<PackMeta>(meta) {
            Ok(m) => ResourcepackObj {
                description: m.pack.description,
                pack_format: m.pack.pack_format,
                min_format: m.pack.min_format,
                max_format: m.pack.max_format,
                ..Default::default()
            },
            Err(_) => ResourcepackObj {
                fail: true,
                ..Default::default()
            },
        }
    };

    // 读取图标
    if let Ok(mut icon) = zip.by_name(names::PACK_ICON_FILE) {
        let size = icon.size() as usize;
        let mut vec = Vec::with_capacity(size);
        icon.read_to_end(&mut vec).map_err(|err| {
            ErrorType::ArchiveReadError(ErrorData {
                error: err.to_string(),
            })
        })?;
        pack.icon = Some(vec);
    }

    Ok(pack)
}

impl ResourcepackObj {
    /// 删除
    ///
    /// # 返回值
    ///
    /// 成功返回 `Ok(())`；删除失败返回对应错误
    pub fn remove(&self) -> CoreResult<()> {
        path_helper::move_to_trash(&self.path)
    }
}

impl InstanceSettingObj {
    /// 获取资源包列表
    ///
    /// # 返回值
    ///
    /// 返回资源包列表（读取失败的文件会记录日志并标记 `fail`）
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
                                obj.path = item.clone();

                                list.lock().unwrap().push(obj);
                                return;
                            }
                            Err(err) => {
                                mml_log::error_type(err);
                            }
                        }
                    }

                    list.lock().unwrap().push(ResourcepackObj {
                        description: Default::default(),
                        pack_format: Default::default(),
                        min_format: Default::default(),
                        max_format: Default::default(),
                        hash,
                        path: item.clone(),
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
