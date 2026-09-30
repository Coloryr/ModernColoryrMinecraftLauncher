//! 方块 / 物品贴图离线渲染模块
//!
//! 从 Minecraft 客户端 jar 提取模型与贴图，离线渲染出创造模式物品栏
//! 使用的方块（等轴测 3D）与物品（平面精灵）图标 PNG，并提供图标路径、
//! 创造分组、语言翻译等查询接口。
//!
//! # 子模块
//!
//! | 模块 | 用途 |
//! |------|------|
//! | [`block`] | 方块图标渲染（等轴测 3D） |
//! | [`gpu`] | GPU 渲染支持 |
//! | [`item`] | 物品图标渲染（平面） |
//! | [`model`] | 游戏模型解析 |

use std::{
    collections::HashMap,
    io::Cursor,
    path::{Path, PathBuf},
    sync::{
        LazyLock, OnceLock, RwLock,
        atomic::{AtomicBool, Ordering},
    },
};

use mml_base::{
    archives::BaseArchive,
    file_item::{FileHash, FileItemObj, LaterRun},
    serialize_tools,
};
use mml_game::{
    gui_hook,
    launcher_path::{assets_path, libraries_path, version_path},
    mojang::{assets_obj::AssetsObj, game_arg_obj::GameArgObj},
};
use mml_names::{
    Lang,
    i18_items::error_type::{CoreResult, DataNotFoundData, ErrorType, PathNotExistsData},
    names,
};
use mml_sys::path_helper;
use tokio_util::sync::CancellationToken;

use crate::block::obj::{BlocksObj, ItemsObj};

pub mod block;
pub mod gpu;
pub mod item;
pub mod model;

/// 渲染防重入标志：load_blocks 可能被并发触发（界面按钮连点、界面与内核各起一轮），
/// 不挡住的话会各自下发一次客户端 jar 下载任务，同一文件下两遍
static LOADING: AtomicBool = AtomicBool::new(false);

/// 本轮渲染的取消令牌（每轮 load_blocks 开始时换新，取消不会串到下一轮）
///
/// 同时用于两处：渲染循环按条目查询（[`cancel_token`] 克隆后无锁读），
/// 以及本轮下发的下载任务（客户端 jar / 语言文件）——取消只影响本轮，
/// 不动下载队列里其它任务（整合包安装等）
static CANCEL: LazyLock<RwLock<CancellationToken>> =
    LazyLock::new(|| RwLock::new(CancellationToken::new()));

/// 全局渲染进度回调（GUI 启动时注册）：调用 [`load_blocks`] 时没传回调的
/// 调用方经它上报进度（界面按钮那条路径自带回调，这里只是兜底）
static GUI: OnceLock<gui_hook::ProgressGui> = OnceLock::new();

/// 注册全局渲染进度回调（启动时调用一次）
pub fn set_gui_handel(gui: gui_hook::ProgressGui) {
    let _ = GUI.set(gui);
}

/// 是否正在渲染（供界面层防重入查询）
pub fn is_loading() -> bool {
    LOADING.load(Ordering::Acquire)
}

/// 取消本轮渲染
///
/// 渲染循环与在途下载都是协作式取消：置位后循环跳过剩余条目、
/// 下载在下一个数据块处收手，`load_blocks` 返回 [`ErrorType::TaskCancel`]。
/// 也挂在核心停止链上，退出程序时不再等渲染跑完
pub fn cancel() {
    CANCEL.read().unwrap().cancel();
}

/// 本轮渲染是否已被取消
pub fn is_cancelled() -> bool {
    CANCEL.read().unwrap().is_cancelled()
}

/// 取本轮取消令牌（渲染循环持有克隆，按条目无锁查询）
pub(crate) fn cancel_token() -> CancellationToken {
    CANCEL.read().unwrap().clone()
}

/// 下载并渲染方块/物品贴图（版本清单 → 客户端jar → 解包渲染）
///
/// `gui`可选，渲染期间按已处理的模型数上报进度（未传时回退全局注册的回调）；
/// `force`为 true 时忽略版本短路，全量重渲染；
/// 已有渲染在跑时直接返回 `Ok(())`（调用方先查 [`is_loading`] 区分）；
/// 被 [`cancel`] 取消时返回 [`ErrorType::TaskCancel`]
pub async fn load_blocks(gui: gui_hook::ProgressGui, force: bool) -> CoreResult<()> {
    if LOADING.swap(true, Ordering::AcqRel) {
        return Ok(());
    }
    // 新一轮：换新令牌，上一轮的取消不残留
    *CANCEL.write().unwrap() = CancellationToken::new();
    let gui = gui.or_else(|| GUI.get().and_then(|g| g.clone()));
    let result = load_blocks_impl(gui.clone(), force).await;
    LOADING.store(false, Ordering::Release);
    // 收尾借回调发一次空文字事件，把 running 已复位的最终状态推给界面。
    // 进度事件只在渲染步进时触发，结束时没有任何后续事件，
    // 界面会一直停在最后一次步进事件的“渲染中”状态
    if let Some(gui) = &gui {
        gui.set_progress_text(None);
    }
    result
}

async fn load_blocks_impl(gui: gui_hook::ProgressGui, force: bool) -> CoreResult<()> {
    // 本轮取消令牌：渲染循环按条目查询，下载任务也挂在它上面
    let cancel = cancel_token();

    // 版本清单（本地缓存，缺了才在线拉）；方块与物品分开短路，只补渲染缺的那部分
    let versions = version_path::get_version_obj_online().await?;
    let last = versions.latest.release.clone();
    let block_done = !force && block::mml_tex_draw_id() == last;
    let item_done = !force && item::items_id() == last;
    // 语言文件也算“已完成”：客户端 jar 只带 en_us，中文要按资源索引单独补下，
    // 已渲染过的版本也可能还没下过（短路掉就永远补不上）
    if block_done && item_done && langs_ready() {
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
        url: mml_net::url_helper::get_minecraft_client(&obj.downloads.client.url, &last),
        hash: FileHash::Sha1(obj.downloads.client.sha1.clone()),
        later: LaterRun::None,
    };
    if !item.check_hash()
        && !mml_downloader::start_download_task_cancellable(vec![item.clone()], cancel.clone()).await
    {
        return Err(cancel_or(ErrorType::DownloadFileFail, &cancel));
    }

    // 语言文件（方块/物品显示名）：jar 里只有 en_us，中文等按资源索引补下。
    // 取消要往外抛（本轮已作废），其它失败只记日志：界面退化成显示 ID，贴图照常渲染
    match download_langs(&obj, &cancel).await {
        Ok(()) => {}
        Err(ErrorType::TaskCancel) => return Err(ErrorType::TaskCancel),
        Err(err) => mml_log::error_type(err),
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

/// 收尾错误：本轮已被取消时报取消（用户主动中断，不算失败），否则报原错误
///
/// # 参数
///
/// - `err`: 取消未发生时要返回的错误
/// - `cancel`: 本轮取消令牌
///
/// # 返回值
///
/// 已取消返回 [`ErrorType::TaskCancel`]，否则返回 `err`
fn cancel_or(err: ErrorType, cancel: &CancellationToken) -> ErrorType {
    if cancel.is_cancelled() {
        ErrorType::TaskCancel
    } else {
        err
    }
}

/// 方块语言表缓存（按语言缓存整个翻译表）
static LANGS: LazyLock<RwLock<HashMap<Lang, HashMap<String, String>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

/// 读取方块语言翻译（数据来自blocks/langs下的语言文件，随load_blocks从客户端jar提取）
///
/// - `lang`: 目标语言
/// - `key`: 语言键（如 `block.minecraft.stone`）
///
/// # 返回值
///
/// 返回翻译文本，语言键不存在或语言文件未提取时返回 `None`
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
///
/// - `path`: 数据根目录，方块 / 语言 / 物品目录及数据文件都创建在其下
///
/// # 返回值
///
/// 目录创建失败时返回相应错误，成功返回 `Ok(())`
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
/// 只读回上次的渲染结果（版本号在 `block.json` / `items.json` 里），
/// **不**在启动时自动补渲染：渲染一律由界面按钮经 [`load_blocks`] 触发。
/// 首次运行没有数据文件也正常，读不到就是「未渲染」
pub fn load() -> CoreResult<()> {
    if let Ok(obj) = serialize_tools::json_from_file::<BlocksObj>(BLOCK_FILE.get().unwrap()) {
        *BLOCKS.write().unwrap() = obj;
    }
    if let Some(file) = ITEM_FILE.get()
        && let Ok(obj) = serialize_tools::json_from_file::<ItemsObj>(file)
    {
        *ITEMS.write().unwrap() = obj;
    }

    Ok(())
}

/// 保存数据
pub fn save() -> CoreResult<()> {
    let file = BLOCK_FILE.get().ok_or_else(|| {
        ErrorType::PathNotExists(PathNotExistsData {
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
///
/// - `id`: 方块ID（`minecraft:stone` 或裸 ID `stone`）
///
/// # 返回值
///
/// 返回图标 PNG 路径，方块未渲染时返回 `None`
pub fn get_block_path(id: &str) -> Option<PathBuf> {
    let dir = BLOCK_DIR.get().unwrap();
    let binding = BLOCKS.read().unwrap();
    let file = binding.tex.get(id)?;

    Some(dir.join(file))
}

pub use block::icons::SpecialForm;
pub use block::skin::{SKIN_CAT, add_skin_block, remove_skin_block};

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

/// 需要从资源索引补下的语言文件
///
/// 客户端 jar 只内置 `en_us.json`（由 [`extract_langs`] 提取），其余语言
/// 只在资源索引里列出；这里列的是除 en_us 外界面用得到的语言
const EXTRA_LANGS: [&str; 1] = ["zh_cn"];

/// 语言文件路径（`langs/<name>.json`，不保证存在）
///
/// # 参数
///
/// - `name`: 语言名（如 `zh_cn`）
///
/// # 返回值
///
/// 返回文件路径；语言目录尚未初始化时返回 `None`
fn lang_path(name: &str) -> Option<PathBuf> {
    get_lang_dir().map(|dir| dir.join(format!("{name}.json")))
}

/// [`EXTRA_LANGS`] 里的语言文件是否都已在本地
///
/// # 返回值
///
/// 全部存在返回 `true`
fn langs_ready() -> bool {
    EXTRA_LANGS
        .iter()
        .all(|name| lang_path(name).is_some_and(|file| file.is_file()))
}

/// 从资源索引下载方块/物品名称需要的语言文件
///
/// 客户端 jar 只带 `en_us.json`，`zh_cn` 等语言由版本 JSON 的 `assetIndex`
/// 列出、经资源下载站取回；缺了只能回退显示方块 ID。
/// 失败不算渲染失败（只记日志）：界面退化成显示 ID，贴图渲染照常。
///
/// # 参数
///
/// - `obj`: 版本数据（取其 `assetIndex` 定位资源索引）
/// - `cancel`: 本轮取消令牌（下载挂在它上面，取消只影响本次任务）
///
/// # 返回值
///
/// 成功或无需下载返回 `Ok(())`；本轮被取消返回 [`ErrorType::TaskCancel`]；
/// 索引/语言文件获取失败返回对应错误
async fn download_langs(obj: &GameArgObj, cancel: &CancellationToken) -> CoreResult<()> {
    let Some(index) = &obj.asset_index else {
        return Ok(());
    };

    // 资源索引：与游戏本体的资源检查共用一份本地缓存（assets/indexes/<id>.json），缺了才在线拉
    let assets = match assets_path::get_index(index) {
        Ok(assets) => assets,
        Err(_) => {
            let mut url = index.url.clone();
            mml_net::url_helper::change_source(&mut url);
            let data = mml_net::mojang_api::get_assets(&url).await?;
            let assets: AssetsObj = serialize_tools::json_from_bytes(&data)?;
            assets_path::add_index(obj, &mut Cursor::new(data));
            assets
        }
    };

    // 只下本地缺失或大小不符的（大小不符 = 客户端换版本了）
    let mut list = Vec::new();
    for name in EXTRA_LANGS {
        let Some(asset) = assets.objects.get(&format!("minecraft/lang/{name}.json")) else {
            continue;
        };
        let Some(file) = lang_path(name) else {
            continue;
        };
        let done = match std::fs::metadata(&file) {
            Ok(meta) => meta.len() as i64 == asset.size,
            Err(_) => false,
        };
        if done {
            continue;
        }

        list.push(FileItemObj {
            name: format!("{name}.json"),
            file,
            url: mml_net::url_helper::get_download_assets(&asset.hash),
            hash: FileHash::Sha1(asset.hash.clone()),
            later: LaterRun::None,
        });
    }
    if list.is_empty() {
        return Ok(());
    }

    if !mml_downloader::start_download_task_cancellable(list, cancel.clone()).await {
        return Err(cancel_or(ErrorType::DownloadFileFail, cancel));
    }
    // 语言表按语言整体缓存，重下后要清掉旧版本的翻译
    LANGS.write().unwrap().clear();

    Ok(())
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

/// 方块数据的渲染版本（未渲染过为空串）
pub fn block_version() -> String {
    BLOCKS.read().unwrap().id.clone()
}

/// 方块ID → 语言键（block.minecraft.stone 之类，翻译经 `get_lang`）
pub fn block_name_key(id: &str) -> Option<String> {
    BLOCKS.read().unwrap().name.get(id).cloned()
}

/// 物品数据的渲染版本（未渲染过为空串）
pub fn item_version() -> String {
    ITEMS.read().unwrap().id.clone()
}

/// 物品ID → 语言键（item.minecraft.apple 之类，翻译经 `get_lang`）
pub fn item_name_key(id: &str) -> Option<String> {
    ITEMS.read().unwrap().name.get(id).cloned()
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
///
/// - `id`: 物品ID（`minecraft:apple` 或裸 ID `apple`）
///
/// # 返回值
///
/// 返回图标 PNG 路径，物品未渲染时返回 `None`
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
        "mml-tex-draw-test-{}-{}-{}",
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
