//! 自定义主页的整包句柄：打开 / 校验 / 常驻 / 重载 / 导入
//!
//! 从 `custom_home/mod.rs` 拆出来的。整包是 zip（`index.json` + 资源），句柄常驻在
//! `HANDLE` 里以免每次请求重开；`reload` 在配置变更后被 `lib.rs` 调用。

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock};

use crate::dtos::CustomHomeInfoDto;

use mml_base::archives::BaseArchive;

use super::{current_info, zip_path};

/// 入口页文件名
pub(crate) const INDEX_FILE: &str = "index.html";

/// 找不到入口页时的错误键（前端 i18n 的 `err.customHomeNoIndex`）
pub(super) const ERR_NO_INDEX: &str = "err.customHomeNoIndex";

/// 已打开的压缩包：条目名 → 内容
///
/// `BaseArchive` 内部是 `Mutex<Box<dyn ArchiveHandle>>` 且 `Send + Sync`，
/// 所以能常驻一份给协议 handler 反复按条目读；条目表只在打开时读一次。
pub(super) struct HomeArchive {
    /// 条目索引：规范化后的名字（正斜杠）→ 包内原始条目名
    ///
    /// 单独建索引是因为 zip 里的名字可能用反斜杠（PowerShell `Compress-Archive`
    /// 就是这么写的），而 URL 路径一定是正斜杠。
    index: HashMap<String, String>,
    archive: BaseArchive,
}

impl HomeArchive {
    /// 按规范化名字取包内原始条目名（不存在返回 None）
    ///
    /// 依次尝试：原名 → 补 `./` 前缀 → 反斜杠版本，兼容各种打包工具写出的条目名。
    pub(crate) fn entry_name(&self, name: &str) -> Option<&str> {
        if let Some(found) = self.index.get(name) {
            return Some(found.as_str());
        }
        let dot = format!("./{name}");
        if let Some(found) = self.index.get(&dot) {
            return Some(found.as_str());
        }
        self.index.get(&name.replace('/', "\\")).map(String::as_str)
    }

    /// 读一个条目（全部读进内存；条目不存在或解压失败返回 None）
    pub(crate) fn read(&self, name: &str) -> Option<Vec<u8>> {
        let entry = self.entry_name(name)?;
        self.archive.read(entry).ok()
    }

    /// 包内文件数（不含目录条目）
    pub(crate) fn file_count(&self) -> u32 {
        self.index.len() as u32
    }

    /// 唯一顶层目录名（包根就散放着文件时返回 None）
    ///
    /// 用来兜底「整包套一层目录」的布局：入口页在 `<目录>/index.html` 时，
    /// URL 里的 `/index.html` 也能命中。
    pub(crate) fn single_top_dir(&self) -> Option<String> {
        let mut first: Option<&str> = None;
        for name in self.index.keys() {
            let top = name.split('/').next()?;
            // 根目录下的文件（名字里没有 /）说明不是「整包套一层」
            if !name.contains('/') || top.is_empty() {
                return None;
            }
            match first {
                None => first = Some(top),
                Some(seen) if seen == top => {}
                Some(_) => return None,
            }
        }
        first.map(str::to_string)
    }

    /// 入口页在不在
    pub(crate) fn has_entry(&self) -> bool {
        self.entry_name(INDEX_FILE).is_some()
            || self
                .single_top_dir()
                .is_some_and(|dir| self.entry_name(&format!("{dir}/{INDEX_FILE}")).is_some())
    }
}

/// 进程内常驻的包句柄（None = 没导入过）
pub(super) static HANDLE: OnceLock<Mutex<Option<Arc<HomeArchive>>>> = OnceLock::new();

pub(super) fn handle_cell() -> &'static Mutex<Option<Arc<HomeArchive>>> {
    HANDLE.get_or_init(|| Mutex::new(None))
}

/// 取当前句柄（没导入 / 打不开时为 None）
pub(crate) fn open_handle() -> Option<Arc<HomeArchive>> {
    handle_cell().lock().ok()?.clone()
}

/// 打开压缩包并建好条目索引
pub(super) fn load_archive(zip: &Path) -> Result<HomeArchive, String> {
    let archive = BaseArchive::open_readonly(zip).map_err(|err| err.to_string())?;
    let mut index = HashMap::with_capacity(archive.entries().len());
    for entry in archive.entries() {
        if entry.is_dir {
            continue;
        }
        // 正斜杠化 + 去掉前导 ./，让 URL 路径能直接查到（包内原名照旧用于读取）
        let normalized = entry.name.replace('\\', "/");
        let normalized = normalized.trim_start_matches("./").to_string();
        index.insert(normalized, entry.name.clone());
    }
    Ok(HomeArchive { index, archive })
}

/// （重）打开常驻句柄：zip 在就打开，不在就清空
///
/// 导入 / 删除 / 启动时各调一次。打不开（包损坏）时句柄保持为 None，
/// 协议层自然回 404，主窗口回落到内置主页。
pub(crate) fn reload() {
    let zip = zip_path();
    let loaded = if zip.is_file() {
        match load_archive(&zip) {
            Ok(archive) => {
                // 打开时就把入口定位算出来：包不合法（没有 index.html）也当作没导入，
                // 免得主窗口拿到一个必然 404 的 entry_url
                if archive.entry_name(INDEX_FILE).is_some() {
                    Some(Arc::new(archive))
                } else {
                    eprintln!("[custom_home] 压缩包里没有 {INDEX_FILE}");
                    None
                }
            }
            Err(err) => {
                eprintln!("[custom_home] 打开压缩包失败：{err}");
                None
            }
        }
    } else {
        None
    };
    if let Ok(mut cell) = handle_cell().lock() {
        *cell = loaded;
    }
}

/// 校验待导入的包：能打开、且有入口页（整包套一层目录时也认）
///
/// 入口定位结果**不留状态**：读取时用 [`HomeArchive::entry_name`] 依次尝试
/// `index.html` 与「唯一顶层目录/index.html」，所以这里只做「能不能用」的判断。
pub(super) fn validate_archive(path: &Path) -> Result<(), String> {
    let archive = BaseArchive::open_readonly(path).map_err(|err| err.to_string())?;
    if archive.contains(INDEX_FILE) {
        return Ok(());
    }
    let wrapped = archive
        .single_top_dir()
        .map(|dir| archive.contains(&format!("{dir}/{INDEX_FILE}")))
        .unwrap_or(false);
    if wrapped {
        Ok(())
    } else {
        Err(ERR_NO_INDEX.to_string())
    }
}

/// 导入 zip：**不解压、不落任何目录**，只校验入口后把压缩包存成 `custom_home.zip`
///
/// 导入语义仍是全量替换，且中途失败不能破坏服主已有的包：
/// 1. 先打开压缩包校验入口，不合法直接返回 `err.customHomeNoIndex`（现有包一动不动）；
/// 2. 复制到 `custom_home.zip.tmp`（复制失败也不会碰到成品文件）；
/// 3. 删旧 `custom_home.zip` → `rename` 上架；
/// 4. 重开常驻句柄，让后续请求读新包。
///
/// 之所以在导入时就把整包读一遍：入口校验必须发生在替换之前，否则一个没有
/// `index.html` 的包会把服主原来能用的主页顶掉。
pub(crate) fn import_archive(path: &Path) -> Result<CustomHomeInfoDto, String> {
    validate_archive(path)?;

    let zip = zip_path();
    let tmp = zip.with_extension("zip.tmp");
    if tmp.exists() {
        std::fs::remove_file(&tmp).map_err(|err| err.to_string())?;
    }
    std::fs::copy(path, &tmp).map_err(|err| err.to_string())?;

    if zip.exists() {
        std::fs::remove_file(&zip).map_err(|err| err.to_string())?;
    }
    std::fs::rename(&tmp, &zip).map_err(|err| err.to_string())?;

    // 句柄换成新包（旧句柄的 Arc 还活着也不会影响：它指向的是已被替换的旧文件）
    reload();

    Ok(current_info())
}
