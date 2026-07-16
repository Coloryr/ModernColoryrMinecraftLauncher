/// 游戏实例资源包相关
use std::{
    cmp,
    io::Read,
    path::{Path, PathBuf},
    sync::Mutex,
};

use mcml_base::{
    file_item::FileHash, hash_helper::{self, HashType}, path_helper, serialize_tools,
};
use mcml_names::{
    i18_items::error_type::{CoreResult, ErrorData, ErrorType, FileSystemErrorData},
    names,
};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use serde::Deserialize;
use serde_json::Number;
use zip::ZipArchive;

use crate::launcher::instance_setting_obj::InstanceSettingObj;

/// Deserialize a JSON value that can be either a number or an array of numbers.
/// For a number: returns it directly.
/// For an array: returns the minimum value (0 for empty array).
fn deserialize_number_or_min<'de, D>(deserializer: D) -> Result<i64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::{self, SeqAccess, Visitor};
    use std::fmt;

    struct V;
    impl<'de> Visitor<'de> for V {
        type Value = i64;
        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_str("a number or array of numbers")
        }
        fn visit_i64<E: de::Error>(self, v: i64) -> Result<i64, E> {
            Ok(v)
        }
        fn visit_u64<E: de::Error>(self, v: u64) -> Result<i64, E> {
            Ok(v as i64)
        }
        fn visit_f64<E: de::Error>(self, v: f64) -> Result<i64, E> {
            Ok(v as i64)
        }
        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<i64, A::Error> {
            let mut min = i64::MAX;
            let mut found = false;
            while let Some(v) = seq.next_element::<Number>()? {
                if let Some(n) = v.as_i64() {
                    min = cmp::min(min, n);
                    found = true;
                }
            }
            Ok(if found { min } else { 0 })
        }
    }
    deserializer.deserialize_any(V)
}

/// Deserialize a JSON value that can be either a number or an array of numbers.
/// For a number: returns it directly.
/// For an array: returns the maximum value (0 for empty array).
fn deserialize_number_or_max<'de, D>(deserializer: D) -> Result<i64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::{self, SeqAccess, Visitor};
    use std::fmt;

    struct V;
    impl<'de> Visitor<'de> for V {
        type Value = i64;
        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_str("a number or array of numbers")
        }
        fn visit_i64<E: de::Error>(self, v: i64) -> Result<i64, E> {
            Ok(v)
        }
        fn visit_u64<E: de::Error>(self, v: u64) -> Result<i64, E> {
            Ok(v as i64)
        }
        fn visit_f64<E: de::Error>(self, v: f64) -> Result<i64, E> {
            Ok(v as i64)
        }
        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<i64, A::Error> {
            let mut max = i64::MIN;
            let mut found = false;
            while let Some(v) = seq.next_element::<Number>()? {
                if let Some(n) = v.as_i64() {
                    max = cmp::max(max, n);
                    found = true;
                }
            }
            Ok(if found { max } else { 0 })
        }
    }
    deserializer.deserialize_any(V)
}

/// Deserialization struct for pack.mcmeta
#[derive(Deserialize)]
struct PackMeta {
    pack: PackInfo,
}

#[derive(Deserialize)]
struct PackInfo {
    #[serde(default, deserialize_with = "deserialize_number_or_min")]
    pack_format: i64,
    #[serde(default, deserialize_with = "deserialize_number_or_min")]
    min_format: i64,
    #[serde(default, deserialize_with = "deserialize_number_or_max")]
    max_format: i64,
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

fn process_resourcepack<P: AsRef<Path>>(path: P) -> CoreResult<ResourcepackObj> {
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

        match serialize_tools::json_stream::<PackMeta, _>(meta) {
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
    pub fn remove(&self) -> CoreResult<()> {
        path_helper::move_to_trash(&self.path)
    }
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
                                obj.path = item.clone();

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
