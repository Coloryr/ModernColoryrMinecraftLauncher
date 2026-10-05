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
    ///
    /// 1.20.5+ 之后不少包不再写它、只写 `supported_formats`，所以是可选的。
    #[serde(
        default,
        deserialize_with = "serialize_tools::deserialize_number_or_max"
    )]
    pack_format: i64,
    /// 支持的最小格式版本
    ///
    /// 两种写法都认（见 [`supported_range`]）：
    /// - 直接给数字：`"min_format": 5`
    /// - 1.20.5+ 的区间对象：`"supported_formats": {"min_inclusive": 5, "max_inclusive": 75}`
    #[serde(
        default,
        deserialize_with = "serialize_tools::deserialize_number_or_min"
    )]
    min_format: i64,
    /// 支持的最大格式版本（同上）
    #[serde(
        default,
        deserialize_with = "serialize_tools::deserialize_number_or_max"
    )]
    max_format: i64,
    /// 简介文字（三种写法，见 [`pack_description`]）
    #[serde(default, deserialize_with = "deserialize_text_component")]
    description: String,
    /// 支持的格式区间（1.20.5+ 的新写法，与 min/max_format 二选一）
    #[serde(default)]
    supported_formats: Option<SupportedFormats>,
}

/// `supported_formats` 的**三种**写法
///
/// 三种都在真实资源包里见过（每次都是"整包读取失败"报上来的）：
/// - 对象：`{"min_inclusive": 5, "max_inclusive": 75}`
/// - 数组：`[8, 9999]`（就是 `[min, max]`）
/// - 单个数字：`20`（"从这一版起"）
///
/// **漏掉任何一种都会让整个 `PackInfo` 反序列化失败**（untagged 要求所有变体都试过、
/// 全不匹配就报错），而不是简单地"区间读不到" —— 所以新增写法时务必补在这里。
#[derive(Deserialize)]
#[serde(untagged)]
enum SupportedFormats {
    /// 区间对象（键都是可选的 —— 只写 min 表示"这个版本及以后"）
    Range {
        #[serde(default)]
        min_inclusive: Option<i64>,
        #[serde(default)]
        max_inclusive: Option<i64>,
    },
    /// 数组 `[min, max]`；只给一个元素时等价于"从这一版起"
    Bounds(Vec<i64>),
    /// 单个数字：`"supported_formats": 20`
    Single(i64),
}

impl SupportedFormats {
    /// 拆成 `(min, max)`，缺的按"未知"给 `0`
    fn range(&self) -> (i64, i64) {
        match self {
            SupportedFormats::Range {
                min_inclusive,
                max_inclusive,
            } => (min_inclusive.unwrap_or(0), max_inclusive.unwrap_or(0)),
            // `[min]` / `[min, max]`；空数组当"没给"
            SupportedFormats::Bounds(list) => (
                list.first().copied().unwrap_or(0),
                list.get(1).copied().unwrap_or(0),
            ),
            SupportedFormats::Single(n) => (*n, 0),
        }
    }
}

/// 反序列化 `description`：字符串 / 组件数组 / 组件对象都认
///
/// 这个字段游戏三种写法都接受，但早期版本只写字符串、新版本越来越多用组件数组：
/// - `"description": "一句话"`
/// - `"description": [{"text": "一句话"}]`
/// - `"description": {"text": "一句话"}`
///
/// 以前这里声明成 `String`，**数组 / 对象形式会让整个 `PackMeta` 反序列化失败**
/// —— 于是 `process_resourcepack` 走 `Err(_)` 分支、`fail = true`，
/// 界面上整包显示"读取失败"（这就是那个数据包报错的原因）。
fn deserialize_text_component<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    // 先收成通用 JSON 值，再按形状取值（三种形态差异太大，用 enum 反而绕）
    let value = serde_json::Value::deserialize(deserializer)?;
    Ok(flatten_text(&value))
}

/// 从文本组件的 JSON 值里抠出纯文本
///
/// 只认 `text`：`translate` / `keybind` 这类要查语言表，只有客户端渲染得出来，
/// 启动器拿不到那份信息，硬凑不如留空。数组按顺序拼接（多个组件就是一段话）。
fn flatten_text(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Array(list) => list.iter().map(flatten_text).collect(),
        serde_json::Value::Object(map) => map
            .get("text")
            .map(flatten_text)
            .unwrap_or_default(),
        _ => String::new(),
    }
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
            Ok(m) => {
                // 版本区间：`min_format` / `max_format` 优先；两者都没写时才用
                // `supported_formats`（1.20.5+ 的新写法）—— 别让新写法盖掉显式给的旧字段
                let (sf_min, sf_max) = m
                    .pack
                    .supported_formats
                    .as_ref()
                    .map(|s| s.range())
                    .unwrap_or((0, 0));
                ResourcepackObj {
                    description: m.pack.description,
                    pack_format: m.pack.pack_format,
                    min_format: if m.pack.min_format != 0 {
                        m.pack.min_format
                    } else {
                        sf_min
                    },
                    max_format: if m.pack.max_format != 0 {
                        m.pack.max_format
                    } else {
                        sf_max
                    },
                    ..Default::default()
                }
            }
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
