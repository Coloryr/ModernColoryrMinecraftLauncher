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

use crate::block_obj::BlocksObj;

pub mod block_obj;
pub mod block_render;

/// 下载并渲染方块贴图（版本清单 → 客户端jar → 解包渲染 cube_all 方块）
///
/// `gui`可选，渲染期间按已处理的候选模型数上报进度
pub async fn load_blocks(gui: gui_hook::ProgressGui) -> CoreResult<()> {
    // 版本清单（本地缓存，缺了才在线拉）
    let versions = version_path::get_version_obj_online().await?;
    let last = versions.latest.release.clone();

    if block_render::mcml_tex_draw_id() == last {
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

    // 打开jar并渲染全部方块
    let archive = BaseArchive::open(&item.file)?;
    block_render::render_blocks(&archive, gui)?;

    blocks_write().id = last;
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

static BLOCKS: LazyLock<RwLock<BlocksObj>> = LazyLock::new(|| RwLock::new(BlocksObj::default()));

/// 初始化
pub fn init<P: AsRef<Path>>(path: P) -> CoreResult<()> {
    BLOCK_FILE.get_or_init(|| path.as_ref().join(names::BLOCK_FILE));

    let dir = BLOCK_DIR.get_or_init(|| path.as_ref().join(names::BLOCK_DIR));
    if !dir.exists() {
        path_helper::create_dir_all(dir)?;
    }

    let dir = LANG_DIR.get_or_init(|| dir.join(names::BLOCK_LANGS_DIR));
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

    Ok(())
}

/// 获取方块图片路径
pub fn get_block_path(id: &str) -> Option<PathBuf> {
    let dir = BLOCK_DIR.get().unwrap();
    let binding = BLOCKS.read().unwrap();
    let file = binding.tex.get(id)?;

    Some(dir.join(file))
}

/// 获取方块数据目录
pub(crate) fn get_block_dir() -> Option<PathBuf> {
    BLOCK_DIR.get().cloned()
}

/// 获取方块语言文件目录
pub(crate) fn get_lang_dir() -> Option<PathBuf> {
    LANG_DIR.get().cloned()
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
