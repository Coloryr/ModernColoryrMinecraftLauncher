use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{LazyLock, OnceLock, RwLock},
};

use mcml_base::{
    archives::BaseArchive,
    file_item::{FileHash, FileItemObj, LaterRun},
    serialize_tools,
};
use mcml_game::{gui_hook, launcher_path::{libraries_path, version_path}};
use mcml_names::{
    Lang,
    i18_items::error_type::{CoreResult, DataNotFoundData, ErrorType, PathNotExistsData},
    names,
};
use mcml_sys::path_helper;

use crate::block::obj::{BlocksObj, ItemsObj};

pub mod block;
pub mod cpu;
pub mod gpu;
pub mod item;
pub mod model;

/// 下载并渲染方块/物品贴图（版本清单 → 客户端jar → 解包渲染）
///
/// `gui`可选，渲染期间按已处理的模型数上报进度
pub async fn load_blocks(gui: gui_hook::ProgressGui) -> CoreResult<()> {
    // 版本清单（本地缓存，缺了才在线拉）；方块与物品分开短路，只补渲染缺的那部分
    let versions = version_path::get_version_obj_online().await?;
    let last = versions.latest.release.clone();
    let block_done = block::mcml_tex_draw_id() == last;
    let item_done = item::items_id() == last;
    if block_done && item_done {
        return Ok(());
    }

    let Some(ver) = versions.versions.iter().find(|v| v.id == last) else {
        return Err(ErrorType::DataNotFound(DataNotFoundData::Version(last)));
    };
    // 版本JSON（带下载源切换和本地缓存）
    let obj = version_path::add_game(ver).await?;
    if obj.downloads.client.url.is_empty() {
        return Err(ErrorType::DataNotFound(DataNotFoundData::Version(last)));
    }

    // 客户端jar（与游戏库共用一份，已存在且sha1匹配时跳过下载）
    let item = FileItemObj {
        name: format!("{last}.jar"),
        file: libraries_path::get_game_file(&last),
        url: mcml_net::url_helper::get_minecraft_client(&obj.downloads.client.url, &last),
        hash: FileHash::Sha1(obj.downloads.client.sha1.clone()),
        later: LaterRun::None,
    };
    if !item.check_hash() && !mcml_downloader::start_download_task(vec![item.clone()]).await {
        return Err(ErrorType::DownloadFileFail);
    }

    // 打开jar并渲染
    let archive = BaseArchive::open(&item.file)?;
    if !block_done {
        block::render_blocks(&archive, gui.clone())?;
        blocks_write().id = last.clone();
    }
    if !item_done {
        item::render_items(&archive, gui.clone())?;
        items_write().id = last.clone();
    }
    save()?;

    Ok(())
}

/// 方块语言表缓存（按语言缓存整个翻译表）
static LANGS: LazyLock<RwLock<HashMap<Lang, HashMap<String, String>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

/// 读取方块语言翻译（数据来自blocks/langs下的语言文件，随load_blocks从客户端jar提取）
pub fn get_lang(lang: Lang, key: &str) -> Option<String> {
    // 命中缓存
    if let Some(map) = LANGS.read().unwrap().get(&lang) {
        return map.get(key).cloned();
    }

    // 从磁盘加载语言文件
    let name = match lang {
        Lang::zh_cn => "zh_cn",
        Lang::en_us => "en_us",
    };
    let file = get_lang_dir()?.join(format!("{name}.json"));
    let text = path_helper::read_text(&file).ok()?;
    let map: HashMap<String, String> = serialize_tools::json_from_str(&text).ok()?;

    let res = map.get(key).cloned();
    LANGS.write().unwrap().insert(lang, map);
    res
}

static BLOCK_FILE: OnceLock<PathBuf> = OnceLock::new();
static BLOCK_DIR: OnceLock<PathBuf> = OnceLock::new();
static LANG_DIR: OnceLock<PathBuf> = OnceLock::new();
static ITEM_FILE: OnceLock<PathBuf> = OnceLock::new();
static ITEM_DIR: OnceLock<PathBuf> = OnceLock::new();

static BLOCKS: LazyLock<RwLock<BlocksObj>> = LazyLock::new(|| RwLock::new(BlocksObj::default()));
static ITEMS: LazyLock<RwLock<ItemsObj>> = LazyLock::new(|| RwLock::new(ItemsObj::default()));

/// 初始化
pub fn init<P: AsRef<Path>>(path: P) -> CoreResult<()> {
    BLOCK_FILE.get_or_init(|| path.as_ref().join(names::BLOCK_FILE));
    ITEM_FILE.get_or_init(|| path.as_ref().join(names::ITEM_FILE));

    let dir = BLOCK_DIR.get_or_init(|| path.as_ref().join(names::BLOCK_DIR));
    if !dir.exists() {
        path_helper::create_dir_all(dir)?;
    }

    // 语言文件目录与block平级（方块/物品共用，随渲染从客户端jar提取）
    let dir = LANG_DIR.get_or_init(|| path.as_ref().join(names::LANG_DIR));
    if !dir.exists() {
        path_helper::create_dir_all(dir)?;
    }

    let dir = ITEM_DIR.get_or_init(|| path.as_ref().join(names::ITEM_DIR));
    if !dir.exists() {
        path_helper::create_dir_all(dir)?;
    }

    Ok(())
}

/// 加载数据
///
/// 读回上次的结果后，在后台执行load_blocks（版本短路命中时直接返回），
/// 首次运行没有数据文件也正常，等后台下载渲染完成再save
pub fn load() -> CoreResult<()> {
    if let Ok(obj) = serialize_tools::json_from_file::<BlocksObj>(BLOCK_FILE.get().unwrap()) {
        *BLOCKS.write().unwrap() = obj;
    }
    if let Some(file) = ITEM_FILE.get()
        && let Ok(obj) = serialize_tools::json_from_file::<ItemsObj>(file)
    {
        *ITEMS.write().unwrap() = obj;
    }

    spawn_load_task();

    Ok(())
}

/// 后台执行load_blocks，不阻塞调用方，错误只记日志
fn spawn_load_task() {
    let task = async {
        if let Err(err) = load_blocks(None).await {
            mcml_log::error_type(err);
        }
    };
    match tokio::runtime::Handle::try_current() {
        // 已在异步环境（如GUI的tauri::async_runtime），挂到当前runtime
        Ok(handle) => {
            handle.spawn(task);
        }
        // 同步环境，独立线程带自己的runtime
        Err(_) => {
            std::thread::spawn(move || {
                tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap()
                    .block_on(task);
            });
        }
    }
}

/// 保存数据
pub fn save() -> CoreResult<()> {
    let file = BLOCK_FILE.get().ok_or_else(|| {
        ErrorType::FileNotExists(PathNotExistsData {
            path: PathBuf::from(names::BLOCK_FILE),
        })
    })?;
    let obj = BLOCKS.read().unwrap().clone();
    serialize_tools::json_to_file(&obj, file)?;

    if let Some(file) = ITEM_FILE.get() {
        let obj = ITEMS.read().unwrap().clone();
        serialize_tools::json_to_file(&obj, file)?;
    }

    Ok(())
}

/// 获取方块图片路径
pub fn get_block_path(id: &str) -> Option<PathBuf> {
    let dir = BLOCK_DIR.get().unwrap();
    let binding = BLOCKS.read().unwrap();
    let file = binding.tex.get(id)?;

    Some(dir.join(file))
}

pub use block::icons::SpecialForm;

/// 判断方块ID是否有特殊形态图标，返回对应的形态
///
/// ID带不带"minecraft:"前缀均可
pub fn block_special_form(id: &str) -> Option<SpecialForm> {
    let base = id.strip_prefix("minecraft:").unwrap_or(id);
    block::icons::special_form(&format!("minecraft:{base}"))
}

/// 获取方块特殊形态的图片路径（形态图标ID = 基础ID + "_" + 形态后缀，如 oak_door_open）
///
/// 与`block_special_form`配合使用：该方块无此形态时返回None。
/// 形态条目不注册进方块列表（blocks()查不到），文件名按规则直接拼出
pub fn get_block_path_form(id: &str, form: SpecialForm) -> Option<PathBuf> {
    if block_special_form(id) != Some(form) {
        return None;
    }
    let base = id.strip_prefix("minecraft:").unwrap_or(id);
    Some(
        BLOCK_DIR.get()?.join(format!(
            "minecraft_{base}_{}.png",
            form.suffix()
        )),
    )
}

/// 获取方块数据目录
pub(crate) fn get_block_dir() -> Option<PathBuf> {
    BLOCK_DIR.get().cloned()
}

/// 获取方块语言文件目录
pub(crate) fn get_lang_dir() -> Option<PathBuf> {
    LANG_DIR.get().cloned()
}

/// 提取语言文件到langs目录，返回（方块ID→语言键, 物品ID→语言键）映射
/// （zh_cn优先，en_us兜底；新版jar可能只有en_us）
pub(crate) fn extract_langs(
    archive: &BaseArchive,
) -> (
    std::collections::HashMap<String, String>,
    std::collections::HashMap<String, String>,
) {
    for name in ["zh_cn", "en_us"] {
        let Ok(data) = archive.read(&format!("assets/minecraft/lang/{name}.json")) else {
            continue;
        };
        // 复制到langs目录，供get_lang查询翻译
        if let Some(dir) = get_lang_dir() {
            let _ = path_helper::write_bytes(dir.join(format!("{name}.json")), &data);
        }
        let Ok(buf) = String::from_utf8(data) else {
            continue;
        };
        let Ok(lang) = serialize_tools::json_from_str::<
            std::collections::HashMap<String, String>,
        >(&buf) else {
            continue;
        };
        // 方块/物品ID → 语言键：block.minecraft.stone / item.minecraft.apple → minecraft:stone / minecraft:apple
        //（Name只存语言键，由GUI按当前语言经get_lang翻译）
        let mut block_map = std::collections::HashMap::new();
        let mut item_map = std::collections::HashMap::new();
        for lang_key in lang.into_keys() {
            let keys: Vec<&str> = lang_key.split('.').collect();
            if keys.len() == 3 && keys[0] == "block" {
                block_map.insert(format!("{}:{}", keys[1], keys[2]), lang_key.clone());
            } else if keys.len() == 3 && keys[0] == "item" {
                item_map.insert(format!("{}:{}", keys[1], keys[2]), lang_key);
            }
        }
        return (block_map, item_map);
    }
    (std::collections::HashMap::new(), std::collections::HashMap::new())
}

/// 获取方块数据
pub fn blocks() -> Vec<String> {
    BLOCKS
        .read()
        .unwrap()
        .tex
        .keys()
        .map(|item| item.clone())
        .collect()
}

/// 锁定方块数据表（内部写入用）
pub(crate) fn blocks_write() -> std::sync::RwLockWriteGuard<'static, BlocksObj> {
    BLOCKS.write().unwrap()
}

/// 锁定方块数据表（内部读取用）
pub(crate) fn blocks_read() -> std::sync::RwLockReadGuard<'static, BlocksObj> {
    BLOCKS.read().unwrap()
}

/// 获取物品图片路径
pub fn get_item_path(id: &str) -> Option<PathBuf> {
    let dir = ITEM_DIR.get().unwrap();
    let binding = ITEMS.read().unwrap();
    let file = binding.tex.get(id)?;

    Some(dir.join(file))
}

/// 获取方块创造分组（itemGroup lang键尾段，Name字段为语言键，翻译经get_lang）
pub fn block_cat(id: &str) -> Option<String> {
    BLOCKS.read().unwrap().cat.get(id).cloned()
}

/// 获取物品创造分组（itemGroup lang键尾段，与block_cat同一套）
pub fn item_cat(id: &str) -> Option<String> {
    ITEMS.read().unwrap().cat.get(id).cloned()
}

/// 获取物品数据目录
pub(crate) fn get_item_dir() -> Option<PathBuf> {
    ITEM_DIR.get().cloned()
}

/// 获取物品数据
pub fn items() -> Vec<String> {
    ITEMS
        .read()
        .unwrap()
        .tex
        .keys()
        .map(|item| item.clone())
        .collect()
}

/// 锁定物品数据表（内部写入用）
pub(crate) fn items_write() -> std::sync::RwLockWriteGuard<'static, ItemsObj> {
    ITEMS.write().unwrap()
}

/// 锁定物品数据表（内部读取用）
pub(crate) fn items_read() -> std::sync::RwLockReadGuard<'static, ItemsObj> {
    ITEMS.read().unwrap()
}

/// 生成测试用的唯一临时目录（不自动创建，由调用方决定）
///
/// 目录位于系统临时目录下，带有进程 ID 与纳秒级时间戳，避免并发冲突。
#[cfg(test)]
fn unique_temp_dir(tag: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "mcml-tex-draw-test-{}-{}-{}",
        tag,
        std::process::id(),
        nanos
    ))
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use super::*;

    /// init 应在指定路径下创建 block 数据目录，且目录名来自 names::BLOCK_DIR
    ///
    /// 注意：BLOCK_FILE / BLOCK_DIR 为进程级 OnceLock，首次调用后不再变化，
    /// 因此本二进制内的初始化断言集中在这一条测试中顺序执行。
    #[test]
    fn init_creates_block_dir() {
        let root = unique_temp_dir("unit");
        fs::create_dir_all(&root).unwrap();

        let result = init(&root);
        assert!(result.is_ok());

        let dir = root.join(names::BLOCK_DIR);
        assert!(dir.exists(), "init 后应创建 block 目录：{}", dir.display());

        // 重复调用不应报错（OnceLock 保持首次的路径）
        assert!(init(&root).is_ok());

        // 清理临时目录
        let _ = fs::remove_dir_all(&root);
    }

    /// names 模块提供的常量应为预期的相对名称
    #[test]
    fn names_constants() {
        assert_eq!(names::BLOCK_DIR, "block");
        assert_eq!(names::BLOCK_FILE, "block.json");

        // 路径拼接不依赖平台分隔符写法
        let path = Path::new("data").join(names::BLOCK_FILE);
        assert!(path.ends_with("block.json"));
    }
}
