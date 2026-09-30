//! 自定义主页面：服主导入的整页 HTML 替换启动器主页
//!
//! 页面本体解包在 `base_dir/custom_home/`，经自定义协议 `mml-home` 供主窗口的
//! iframe 加载（`http://mml-home.localhost/...`，见 [`home_base_url`]）。
//!
//! | 部分 | 说明 |
//! | --- | --- |
//! | [`custom_home_import`] | 导入 zip：解到 `custom_home.tmp/` → 校验入口 → 原子替换 |
//! | [`custom_home_status`] | 现算状态（目录 mtime 即破缓存版本号，磁盘上不存派生状态） |
//! | [`custom_home_remove`] | 删除 `custom_home/` |
//! | [`custom_home_open_dir`] | 在文件管理器里打开该目录，方便服主调试 |
//! | [`url_custom_home`] | `mml-home` 协议 handler（路径穿越防护的唯一强制安全项） |
//!
//! 桥接脚本（[`BRIDGE_JS`]）是内置资源，不在 `custom_home/` 里：返回 HTML 时注入
//! `<script src="/__mml_bridge.js">`，页面据此拿到 `window.mml`（invoke / on / ready）。
//!
//! **安全边界**：自定义页面通过 `invoke` 能调任意已注册命令，这与「服主给的整合包
//! 本身就是代码」的信任级别一致，是确认过的设计；唯一的强制项是协议层的路径穿越防护。

use std::{
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::UNIX_EPOCH,
};

use mml_base::archives::{BaseArchive, IBaseArchiveGui};
use tauri::{
    AppHandle, Emitter, UriSchemeResponder,
    http::{Request, Response, StatusCode},
};

use crate::dtos::{CustomHomeInfoDto, CustomHomeProgressDto};
use crate::listens;

/// 解包目录名（运行根目录下）
const DIR_NAME: &str = "custom_home";
/// 解包中转目录名：与成品目录同盘，`rename` 才不跨卷复制
const TMP_DIR_NAME: &str = "custom_home.tmp";
/// 入口页文件名
const INDEX_FILE: &str = "index.html";
/// 桥接脚本的请求路径（内置资源，zip 里的同名文件不生效）
const BRIDGE_PATH: &str = "/__mml_bridge.js";
/// 桥接脚本本体（独立 js 文件，别塞成 Rust 字符串字面量）
const BRIDGE_JS: &str = include_str!("custom_home_bridge.js");
/// 注入到 HTML 里的桥接脚本标签
const BRIDGE_TAG: &str = "<script src=\"/__mml_bridge.js\"></script>";
/// 找不到入口页时的错误键（前端 i18n 的 `err.customHomeNoIndex`）
const ERR_NO_INDEX: &str = "err.customHomeNoIndex";

// ================= 路径 =================

/// 自定义主页面根目录
fn home_dir() -> PathBuf {
    mml_base::get_base_dir().join(DIR_NAME)
}

/// 导入中转目录（导入中途失败时残留，下次导入先清掉）
fn tmp_dir() -> PathBuf {
    mml_base::get_base_dir().join(TMP_DIR_NAME)
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

// ================= 状态 =================

/// 递归统计目录下的文件数
fn count_files(dir: &Path) -> u32 {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return 0;
    };
    let mut count = 0u32;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            count = count.saturating_add(count_files(&path));
        } else if path.is_file() {
            count = count.saturating_add(1);
        }
    }
    count
}

/// 内容版本号：取目录 mtime（毫秒时间戳）
///
/// 导入是整目录替换，顶层 mtime 必然变化，所以不需要额外的版本字段。
fn dir_version(dir: &Path) -> String {
    std::fs::metadata(dir)
        .and_then(|meta| meta.modified())
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|dur| dur.as_millis().to_string())
        .unwrap_or_default()
}

/// 现算当前状态（入口 URL 由这里拼好，前端不重算 scheme 前缀）
fn current_info() -> CustomHomeInfoDto {
    let dir = home_dir();
    let installed = dir.join(INDEX_FILE).is_file();
    let version = if installed { dir_version(&dir) } else { String::new() };
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
        file_count: if installed { count_files(&dir) } else { 0 },
    }
}

/// 查询已导入情况与入口 URL
#[tauri::command]
pub fn custom_home_status() -> Result<CustomHomeInfoDto, String> {
    Ok(current_info())
}

// ================= 导入 =================

/// 自定义主页面导入进度事件（解包推进时发）
#[gui_macros::emit]
pub fn emit_custom_home_progress(app: &AppHandle, dto: CustomHomeProgressDto) {
    let _ = app.emit(listens::CUSTOM_HOME_PROGRESS, dto);
}

/// 自定义主页面内容变更事件（导入 / 删除成功后广播）
///
/// 主窗口的 iframe 地址只在导入 / 删除时才变，而这两件事都不会改
/// `gui_config.client`（`client-config-change` 不会发），所以单独立一个事件，
/// 主窗口收到后重拉一次 `custom_home_status`。
#[gui_macros::emit]
pub fn emit_custom_home_change(app: &AppHandle) {
    let _ = app.emit(listens::CUSTOM_HOME_CHANGE, ());
}

/// 解包进度回调 → `custom-home-progress` 事件桥接
///
/// 与 `JavaArchiveGui` 同构：`start` 下发的 total 记在内部，`update` 携带它一起转发，
/// 当前文件名放进 `subText`；命令返回即导入完成，`done` 无需再发。
struct CustomHomeArchiveGui {
    app: AppHandle,
    /// `start` 下发的总数，`update` 时回读
    total: AtomicUsize,
}

impl CustomHomeArchiveGui {
    fn new(app: &AppHandle) -> Arc<Self> {
        Arc::new(Self {
            app: app.clone(),
            total: AtomicUsize::new(0),
        })
    }
}

impl IBaseArchiveGui for CustomHomeArchiveGui {
    fn start(&self, total: usize) {
        self.total.store(total, Ordering::Relaxed);
        emit_custom_home_progress(
            &self.app,
            CustomHomeProgressDto {
                state: "extract".to_string(),
                now: 0,
                total: total as u32,
                sub_text: String::new(),
            },
        );
    }

    fn update(&self, filename: Option<String>, current: usize) {
        emit_custom_home_progress(
            &self.app,
            CustomHomeProgressDto {
                state: "extract".to_string(),
                now: current as u32,
                total: self.total.load(Ordering::Relaxed) as u32,
                sub_text: filename.unwrap_or_default(),
            },
        );
    }

    fn done(&self) {}

    /// 导入场景无需询问，非法字符文件名直接同意替换
    fn file_rename(&self, _name: &str) -> bool {
        true
    }
}

/// 解包到中转目录并原子替换 `custom_home/`
///
/// 导入语义是**全量替换**，且中途失败不能留下半个包：
/// 1. 清掉可能残留的 `custom_home.tmp/`，重新解包进去；
/// 2. 定位入口：包内根目录有 `index.html` 就用它；否则整包被单层目录包裹时
///    剥掉那层（`strip_dir`，解出来就是平铺的，不留前缀信息）；
/// 3. 找不到入口 → 删掉中转目录返回 `err.customHomeNoIndex`，**现有包保持不变**；
/// 4. 校验通过 → 删旧目录 → `rename` 上架。
fn import_archive(path: &Path, gui: Arc<CustomHomeArchiveGui>) -> Result<CustomHomeInfoDto, String> {
    let archive = BaseArchive::open_readonly(path).map_err(|err| err.to_string())?;

    // 入口定位（只读压缩包条目表，不解包）
    let strip_dir = if archive.contains(INDEX_FILE) {
        None
    } else {
        match archive.single_top_dir().map(str::to_string) {
            Some(dir) if archive.contains(&format!("{dir}/{INDEX_FILE}")) => Some(dir),
            _ => return Err(ERR_NO_INDEX.to_string()),
        }
    };

    let tmp = tmp_dir();
    if tmp.exists() {
        std::fs::remove_dir_all(&tmp).map_err(|err| err.to_string())?;
    }
    std::fs::create_dir_all(&tmp).map_err(|err| err.to_string())?;

    if let Err(err) = archive.extract_all(&tmp, None, strip_dir, Some(gui)) {
        let _ = std::fs::remove_dir_all(&tmp);
        return Err(err.to_string());
    }
    // 条目表里有不代表真落盘成功（解包中途被拒绝等），再确认一次入口在
    if !tmp.join(INDEX_FILE).is_file() {
        let _ = std::fs::remove_dir_all(&tmp);
        return Err(ERR_NO_INDEX.to_string());
    }

    // 校验通过：删旧目录 → 改名上架（同盘 rename）
    let dir = home_dir();
    if dir.exists() {
        std::fs::remove_dir_all(&dir).map_err(|err| err.to_string())?;
    }
    std::fs::rename(&tmp, &dir).map_err(|err| err.to_string())?;

    Ok(current_info())
}

/// 从 zip 导入自定义主页面（全量替换），返回新的状态
///
/// 解包与替换过程中经 `custom-home-progress` 事件上报进度。
#[tauri::command]
pub async fn custom_home_import(app: AppHandle, path: String) -> Result<CustomHomeInfoDto, String> {
    let gui = CustomHomeArchiveGui::new(&app);
    let info =
        tauri::async_runtime::spawn_blocking(move || import_archive(Path::new(&path), gui))
            .await
            .map_err(|err| err.to_string())??;
    emit_custom_home_change(&app);
    Ok(info)
}

/// 删除已导入的 `custom_home/`（配置里的启用开关不动：删掉后主窗口自动回落到内置主页）
#[tauri::command]
pub fn custom_home_remove(app: AppHandle) -> Result<(), String> {
    let dir = home_dir();
    if dir.exists() {
        std::fs::remove_dir_all(&dir).map_err(|err| err.to_string())?;
    }
    // 中转目录一起清掉（上次导入中途失败可能留下）
    let tmp = tmp_dir();
    if tmp.exists() {
        let _ = std::fs::remove_dir_all(&tmp);
    }
    emit_custom_home_change(&app);
    Ok(())
}

/// 在文件管理器里打开 `custom_home/`（没导入过就先建出来，方便服主放文件）
#[tauri::command]
pub fn custom_home_open_dir() -> Result<(), String> {
    let dir = home_dir();
    std::fs::create_dir_all(&dir).map_err(|err| err.to_string())?;
    mml_sys::open_helper::open_file(&dir);
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

/// 请求路径 → 磁盘上的真实文件路径（未导入 / 越界 / 不存在一律 `None`）
///
/// 路径穿越防护（本次唯一强制安全项）叠三道：
/// 1. 原始串与百分号解码后都要过 [`is_safe_rel`]；
/// 2. 词法拼接的结果必须仍在 `custom_home/` 下；
/// 3. 命中真实文件后再 `canonicalize` 比对一次前缀（挡符号链接）。
fn resolve_file(raw_path: &str) -> Option<PathBuf> {
    let root = home_dir();
    // 未导入：整个协议一律 404
    if !root.is_dir() {
        return None;
    }

    let rel = if raw_path.is_empty() || raw_path == "/" {
        INDEX_FILE
    } else {
        raw_path
    };
    if !is_safe_rel(rel) {
        return None;
    }
    let decoded = percent_decode(rel);
    if !is_safe_rel(&decoded) {
        return None;
    }

    let mut file = root.clone();
    for seg in decoded.split('/') {
        if seg.is_empty() || seg == "." {
            continue;
        }
        file.push(seg);
    }
    if file == root || !file.starts_with(&root) {
        return None;
    }

    let (real, root_real) = (
        std::fs::canonicalize(&file).ok()?,
        std::fs::canonicalize(&root).ok()?,
    );
    if !real.starts_with(&root_real) || !real.is_file() {
        return None;
    }
    Some(real)
}

/// 按扩展名给 MIME（未知扩展名一律 `application/octet-stream`）
fn mime_of(file: &Path) -> &'static str {
    match file
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

/// `mml-home` 协议入口：把 `custom_home/` 下的文件按 MIME 返回
///
/// - `/` 或空路径 → `index.html`；其余去掉前导 `/` 后拼到根目录下
/// - `/__mml_bridge.js` 是内置资源，直接返回桥接脚本
/// - query string（`?v=...` 只是破缓存）与 fragment 由 `uri().path()` 天然滤掉
pub async fn url_custom_home(req: Request<Vec<u8>>, res: UriSchemeResponder) {
    let path = req.uri().path();

    if path == BRIDGE_PATH {
        send_ok(res, BRIDGE_JS.as_bytes().to_vec(), "application/javascript");
        return;
    }

    let Some(file) = resolve_file(path) else {
        send_not_found(res);
        return;
    };
    let Ok(data) = std::fs::read(&file) else {
        send_not_found(res);
        return;
    };

    let mime = mime_of(&file);
    let data = if mime == "text/html" { inject_bridge(data) } else { data };
    send_ok(res, data, mime);
}
