//! 自定义主页面：服主导入的整页 HTML 替换启动器主页
//!
//! **压缩包是唯一产物，全程不落盘解压**：服主导入的 zip 存放在 `base_dir/custom_home.zip`，
//! 启动时打开成一个常驻句柄（[`HomeArchive`]），`mml-home` 协议的每个请求
//! **直接从包内按条目读取**（[`BaseArchive::read`]，内部 seek 到条目偏移解压到内存，
//! 不会把文件铺到磁盘上）。页面经自定义协议 `mml-home` 供主窗口的 iframe 加载
//! （`http://mml-home.localhost/...`，见 [`home_base_url`]）。
//!
//! | 部分 | 说明 |
//! | --- | --- |
//! | [`custom_home_import`] | 导入 zip：校验入口后原子替换 `custom_home.zip`，并重开句柄 |
//! | [`HomeArchive`] | 常驻的只读句柄（`BaseArchive` 内部 `Mutex`，逐条目读取） |
//! | [`custom_home_status`] | 现算状态（zip 的 mtime 即破缓存版本号，磁盘上不存派生状态） |
//! | [`custom_home_remove`] | 删除 zip 并释放句柄 |
//! | [`custom_home_open_dir`] | 在文件管理器里定位压缩包（便于服主替换 / 备份） |
//! | [`url_custom_home`] | `mml-home` 协议 handler（路径穿越防护的唯一强制安全项） |
//!
//! 桥接脚本（[`BRIDGE_JS`]，源码在 `src-tauri/resources/custom_home_bridge.js`）是内置资源，
//! 不在包里：返回 HTML 时注入 `<script src="/__mml_bridge.js">`，
//! 页面据此拿到 `window.mml`（invoke / on / ready）。
//!
//! **安全边界**：自定义页面通过 `invoke` 能调任意已注册命令，这与「服主给的整合包
//! 本身就是代码」的信任级别一致，是确认过的设计；唯一的强制项是协议层的路径穿越防护。

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, OnceLock},
    time::UNIX_EPOCH,
};

use mml_base::archives::BaseArchive;
use tauri::{
    AppHandle, Emitter, UriSchemeResponder,
    http::{Request, Response, StatusCode},
};

use crate::dtos::CustomHomeInfoDto;
use crate::listens;

/// 持久压缩包名（运行根目录下）；前缀统一，改名时两处一起改
const ZIP_PREFIX: &str = "custom_home";
/// 入口页文件名
const INDEX_FILE: &str = "index.html";
/// 桥接脚本的请求路径（内置资源，zip 里的同名文件不生效）
const BRIDGE_PATH: &str = "/__mml_bridge.js";
/// 桥接脚本本体（独立 js 文件放 `src-tauri/resources/`，别塞成 Rust 字符串字面量）
/// - `include_str!` 编译期嵌入二进制：运行时不需要分发这个文件，
///   因此**不要**把它加进 tauri.conf.json 的 `bundle.resources`（那会多打一份没人读的副本）
const BRIDGE_JS: &str = include_str!("../../resources/custom_home_bridge.js");
/// 注入到 HTML 里的桥接脚本标签
const BRIDGE_TAG: &str = "<script src=\"/__mml_bridge.js\"></script>";
/// 找不到入口页时的错误键（前端 i18n 的 `err.customHomeNoIndex`）
const ERR_NO_INDEX: &str = "err.customHomeNoIndex";

// ================= 路径 =================

/// 运行根目录下的绝对路径
fn base_path(name: &str) -> PathBuf {
    mml_base::get_base_dir().join(name)
}

/// 持久压缩包的路径（带扩展名，`zip` / `7z` 都存成这个）
fn zip_path() -> PathBuf {
    base_path(&format!("{ZIP_PREFIX}.zip"))
}

/// `mml-home` 自定义协议的访问前缀
///
/// 与 `image_base_url()` 同规则：Windows / Android 上自定义协议被映射到
/// `http://<scheme>.localhost`，其余平台是 `<scheme>://localhost`。
pub fn home_base_url() -> &'static str {
    if cfg!(windows) || cfg!(target_os = "android") {
        "http://mml-home.localhost"
    } else {
        "mml-home://localhost"
    }
}

// ================= 常驻句柄 =================

/// 已打开的压缩包：条目名 → 内容
///
/// `BaseArchive` 内部是 `Mutex<Box<dyn ArchiveHandle>>` 且 `Send + Sync`，
/// 所以能常驻一份给协议 handler 反复按条目读；条目表只在打开时读一次。
pub struct HomeArchive {
    /// 条目索引：规范化后的名字（正斜杠）→ 包内原始条目名
    ///
    /// 单独建索引是因为 zip 里的名字可能用反斜杠（PowerShell `Compress-Archive`
    /// 就是这么写的），而 URL 路径一定是正斜杠。
    index: HashMap<String, String>,
    archive: BaseArchive,
}

/// 进程内常驻的包句柄（None = 没导入过）
static HANDLE: OnceLock<Mutex<Option<Arc<HomeArchive>>>> = OnceLock::new();

fn handle_cell() -> &'static Mutex<Option<Arc<HomeArchive>>> {
    HANDLE.get_or_init(|| Mutex::new(None))
}

/// 取当前句柄（没导入 / 打不开时为 None）
fn open_handle() -> Option<Arc<HomeArchive>> {
    handle_cell().lock().ok()?.clone()
}

/// 打开压缩包并建好条目索引
fn load_archive(zip: &Path) -> Result<HomeArchive, String> {
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
pub fn reload() {
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

impl HomeArchive {
    /// 按规范化名字取包内原始条目名（不存在返回 None）
    ///
    /// 依次尝试：原名 → 补 `./` 前缀 → 反斜杠版本，兼容各种打包工具写出的条目名。
    fn entry_name(&self, name: &str) -> Option<&str> {
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
    fn read(&self, name: &str) -> Option<Vec<u8>> {
        let entry = self.entry_name(name)?;
        self.archive.read(entry).ok()
    }

    /// 包内文件数（不含目录条目）
    fn file_count(&self) -> u32 {
        self.index.len() as u32
    }

    /// 唯一顶层目录名（包根就散放着文件时返回 None）
    ///
    /// 用来兜底「整包套一层目录」的布局：入口页在 `<目录>/index.html` 时，
    /// URL 里的 `/index.html` 也能命中。
    fn single_top_dir(&self) -> Option<String> {
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
    fn has_entry(&self) -> bool {
        self.entry_name(INDEX_FILE).is_some()
            || self
                .single_top_dir()
                .is_some_and(|dir| self.entry_name(&format!("{dir}/{INDEX_FILE}")).is_some())
    }
}

// ================= 状态 =================

/// 内容版本号：取**压缩包**的 mtime（毫秒时间戳）
///
/// zip 只在服主重新导入时变化，正好是「内容变没变」的判据，用来给 iframe 破缓存。
fn zip_version(zip: &Path) -> String {
    std::fs::metadata(zip)
        .and_then(|meta| meta.modified())
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|dur| dur.as_millis().to_string())
        .unwrap_or_default()
}

/// 现算当前状态（入口 URL 由这里拼好，前端不重算 scheme 前缀）
fn current_info() -> CustomHomeInfoDto {
    let zip = zip_path();
    let handle = open_handle();
    // 句柄能读出来才算「已导入」：包损坏 / 没有入口页时这里就是 false，主窗口会回落
    let installed = handle.as_ref().is_some_and(|h| h.has_entry());
    let version = if installed {
        zip_version(&zip)
    } else {
        String::new()
    };
    let entry_url = if installed {
        format!("{}/{INDEX_FILE}?v={version}", home_base_url())
    } else {
        String::new()
    };
    CustomHomeInfoDto {
        installed,
        enabled: crate::gui_config::get().client.custom_home,
        entry_url,
        version,
        file_count: handle.as_ref().map(|h| h.file_count()).unwrap_or(0),
    }
}

/// 查询已导入情况与入口 URL
#[tauri::command]
pub fn custom_home_status() -> Result<CustomHomeInfoDto, String> {
    Ok(current_info())
}

// ================= 导入 =================

/// 自定义主页面内容变更事件（导入 / 删除成功后广播）
///
/// 主窗口的 iframe 地址只在导入 / 删除时才变，而这两件事都不会改
/// `gui_config.client`（`client-config-change` 不会发），所以单独立一个事件，
/// 主窗口收到后重拉一次 `custom_home_status`。
#[gui_macros::emit]
pub fn emit_custom_home_change(app: &AppHandle) {
    let _ = app.emit(listens::CUSTOM_HOME_CHANGE, ());
}

/// 校验待导入的包：能打开、且有入口页（整包套一层目录时也认）
///
/// 入口定位结果**不留状态**：读取时用 [`HomeArchive::entry_name`] 依次尝试
/// `index.html` 与「唯一顶层目录/index.html」，所以这里只做「能不能用」的判断。
fn validate_archive(path: &Path) -> Result<(), String> {
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
fn import_archive(path: &Path) -> Result<CustomHomeInfoDto, String> {
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

/// 从 zip 导入自定义主页面（全量替换），返回新的状态
///
/// 只校验 + 复制压缩包，**不落盘解压**：请求时直接从包内按条目读。
#[tauri::command]
pub async fn custom_home_import(app: AppHandle, path: String) -> Result<CustomHomeInfoDto, String> {
    let info = tauri::async_runtime::spawn_blocking(move || import_archive(Path::new(&path)))
        .await
        .map_err(|err| err.to_string())??;
    emit_custom_home_change(&app);
    Ok(info)
}

/// 删除已导入的包（配置里的启用开关不动：删掉后主窗口自动回落到内置主页）
#[tauri::command]
pub fn custom_home_remove(app: AppHandle) -> Result<(), String> {
    let zip = zip_path();
    if zip.exists() {
        std::fs::remove_file(&zip).map_err(|err| err.to_string())?;
    }
    // 句柄指向已删除的文件：直接丢掉，后续请求回 404
    reload();
    emit_custom_home_change(&app);
    Ok(())
}

/// 在文件管理器里定位压缩包（方便服主替换包或做备份）
///
/// 还没导入过时就选中运行根目录，让服主知道包该放哪。
#[tauri::command]
pub fn custom_home_open_dir() -> Result<(), String> {
    let zip = zip_path();
    let target = if zip.is_file() {
        zip
    } else {
        let dir = mml_base::get_base_dir();
        std::fs::create_dir_all(&dir).map_err(|err| err.to_string())?;
        dir
    };
    mml_sys::open_helper::open_file(&target);
    Ok(())
}

// ================= mml-home 协议 =================

/// 以 200 响应（带 CORS 头：iframe 是 opaque origin，取子资源要过 CORS）
fn send_ok(res: UriSchemeResponder, data: Vec<u8>, mime: &str) {
    res.respond(
        Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", mime)
            .header("Access-Control-Allow-Origin", "*")
            .body(data)
            .unwrap(),
    );
}

/// 以 404 响应（未导入 / 文件不存在 / 路径越界）
fn send_not_found(res: UriSchemeResponder) {
    res.respond(
        Response::builder()
            .status(StatusCode::NOT_FOUND)
            .header("Access-Control-Allow-Origin", "*")
            .body(Vec::new())
            .unwrap(),
    );
}

/// 十六进制半字节
fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

/// 百分号解码（`%XX`）
///
/// 非法转义（如真实文件名里的 `100%.png`）原样保留，交给后面的安全检查处理；
/// 但 `%2e%2e` 这类编码穿越会在解码后被下面的 `..` 检查挡掉。
fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(hi), Some(lo)) = (hex_val(bytes[i + 1]), hex_val(bytes[i + 2])) {
                out.push((hi << 4) | lo);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// 请求的相对路径是否安全
///
/// 拒绝反斜杠（Windows 分隔符）、`:`（盘符 / 协议）、NUL，以及任何 `..` 段。
fn is_safe_rel(rel: &str) -> bool {
    if rel.contains('\\') || rel.contains(':') || rel.contains('\0') {
        return false;
    }
    !rel.split('/').any(|seg| seg == "..")
}

/// 请求路径 → 包内条目名（未导入 / 越界 / 不存在一律 `None`）
///
/// 路径穿越防护（本次唯一强制安全项）：
/// 1. 原始串与百分号解码后都要过 [`is_safe_rel`]（挡 `..` / `\` / `:`）；
/// 2. 只接受**规范化后能精确命中包内条目**的结果（[`HomeArchive::entry_name`] 是精确匹配，
///    不是前缀拼接），所以不存在「拼出包外路径」的可能；
/// 3. 入口页允许两种布局：包根 `index.html`，或整包套一层目录时的 `<目录>/index.html`。
fn resolve_entry<'a>(archive: &'a HomeArchive, raw_path: &str) -> Option<&'a str> {
    let rel = if raw_path.is_empty() || raw_path == "/" {
        INDEX_FILE.to_string()
    } else {
        raw_path.to_string()
    };
    if !is_safe_rel(&rel) {
        return None;
    }
    let decoded = percent_decode(&rel);
    if !is_safe_rel(&decoded) {
        return None;
    }
    // 去掉前导 / 与 ./，拼成规范化条目名
    let name = decoded
        .trim_start_matches('/')
        .trim_start_matches("./")
        .to_string();
    if name.is_empty() {
        return None;
    }

    if let Some(found) = archive.entry_name(&name) {
        return Some(found);
    }
    // 整包套一层目录：把唯一顶层目录名的前缀试一遍（只在根请求时兜底，避免多花开销）
    if name == INDEX_FILE {
        if let Some(dir) = archive.single_top_dir() {
            let wrapped = format!("{dir}/{INDEX_FILE}");
            return archive.entry_name(&wrapped);
        }
    }
    None
}

/// 按扩展名给 MIME（未知扩展名一律 `application/octet-stream`）
fn mime_of(name: &str) -> &'static str {
    match Path::new(name)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase())
        .as_deref()
    {
        Some("html") | Some("htm") => "text/html",
        Some("css") => "text/css",
        Some("js") | Some("mjs") => "application/javascript",
        Some("json") => "application/json",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("ico") => "image/x-icon",
        Some("woff") => "font/woff",
        Some("woff2") => "font/woff2",
        Some("ttf") => "font/ttf",
        Some("otf") => "font/otf",
        Some("txt") => "text/plain",
        _ => "application/octet-stream",
    }
}

/// 大小写不敏感地找 ASCII 子串（`</head>` / `</HEAD>` 都要命中）
fn find_ignore_ascii_case(hay: &str, needle: &str) -> Option<usize> {
    let (h, n) = (hay.as_bytes(), needle.as_bytes());
    if n.is_empty() || h.len() < n.len() {
        return None;
    }
    (0..=h.len() - n.len()).find(|&i| h[i..i + n.len()].eq_ignore_ascii_case(n))
}

/// 在 HTML 的 `</head>` 之前插入桥接脚本；没有 `</head>` 就插在最前面
///
/// 响应体不是合法 UTF-8 时原样返回（页面仍能显示，只是拿不到 `window.mml`）。
fn inject_bridge(data: Vec<u8>) -> Vec<u8> {
    let text = match String::from_utf8(data) {
        Ok(text) => text,
        Err(err) => return err.into_bytes(),
    };
    let mut out = String::with_capacity(text.len() + BRIDGE_TAG.len());
    match find_ignore_ascii_case(&text, "</head>") {
        Some(pos) => {
            out.push_str(&text[..pos]);
            out.push_str(BRIDGE_TAG);
            out.push_str(&text[pos..]);
        }
        None => {
            out.push_str(BRIDGE_TAG);
            out.push_str(&text);
        }
    }
    out.into_bytes()
}

/// `mml-home` 协议入口：从常驻的压缩包里按条目读取，按 MIME 返回
///
/// - `/` 或空路径 → `index.html`（含「整包套一层目录」的兜底）
/// - `/__mml_bridge.js` 是内置资源，直接返回桥接脚本（不在包里）
/// - query string（`?v=...` 只是破缓存）与 fragment 由 `uri().path()` 天然滤掉
/// - 全程不落盘：内容是 [`BaseArchive::read`] 从 zip 条目现读出来的
pub async fn url_custom_home(req: Request<Vec<u8>>, res: UriSchemeResponder) {
    let path = req.uri().path();

    if path == BRIDGE_PATH {
        send_ok(res, BRIDGE_JS.as_bytes().to_vec(), "application/javascript");
        return;
    }

    let Some(archive) = open_handle() else {
        send_not_found(res);
        return;
    };
    let Some(name) = resolve_entry(&archive, path) else {
        send_not_found(res);
        return;
    };
    let Some(data) = archive.read(&name) else {
        send_not_found(res);
        return;
    };

    let mime = mime_of(&name);
    let data = if mime == "text/html" {
        inject_bridge(data)
    } else {
        data
    };
    send_ok(res, data, mime);
}
