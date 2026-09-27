//! 设置窗口：规格 + IPC 命令（系统字体枚举 / 网络设置 / 启动设置）
//!
//! 网络设置落在 core `config.json` 的 `HttpObj`，启动设置落在 `RunArgObj`
//! 与 Java 列表（mml-jvms，增删即时持久化）。

use std::path::Path;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use mml_base::archives::IBaseArchiveGui;
use mml_jvms::java_helper;
use tauri::{AppHandle, Emitter};

use crate::dtos::{
    BgInfoDto, DnsSettingDto, GameCheckSettingDto, JavaImportProgressDto, JavaInfoDto,
    LaunchSettingDto, NetworkSettingDto,
};
use crate::listens;

// ================= 界面字体 =================

/// 枚举系统已安装字体的族名（系统直接给出已去重的列表，这里再排序一次）
#[tauri::command]
pub fn settings_get_system_fonts() -> Vec<String> {
    use font_kit::source::SystemSource;

    let mut families = SystemSource::new().all_families().unwrap_or_default();
    families.sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()));
    families
}

// ================= 背景图 =================

/// 最近一次下载的原图缓存（网址来源，仅内存）：调「原始大小」时按来源命中缓存，
/// 直接用原图重新缩放，不重新走网络；换来源或重启后失效（重新下载即可）
static BG_ORIGINAL: Mutex<Option<(String, Vec<u8>, String)>> = Mutex::new(None);

/// 背景图来源加载：本地文件路径 / 网址 → 原始字节 + mime
///
/// - 网址（http/https）：走 mml-net 客户端下载，mime 取响应的 Content-Type
/// - 其余按本地文件路径处理，mime 由扩展名推断（仅支持 png / jpeg / webp）
async fn load_bg_bytes(source: &str) -> Result<(Vec<u8>, String), String> {
    if source.starts_with("http://") || source.starts_with("https://") {
        let client = mml_net::Client::new(mml_config::config_obj::ProxyState::Auto);
        let resp = client.get(source).await.map_err(|err| err.to_string())?;
        let status = resp.status();
        if !status.is_success() {
            return Err(format!("HTTP {status}"));
        }
        let mime = resp
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("image/png")
            .split(';')
            .next()
            .unwrap_or("image/png")
            .trim()
            .to_string();
        // 文本响应基本是错误页（服务挂了返回 HTML），别再往下走解码报不出所以然
        if mime.starts_with("text/") {
            return Err(format!("not an image (mime: {mime})"));
        }
        let bytes = resp.bytes().await.map_err(|err| err.to_string())?;
        Ok((bytes.to_vec(), mime))
    } else {
        let mime = match source
            .rsplit('.')
            .next()
            .map(|e| e.to_lowercase())
            .as_deref()
        {
            Some("png") => "image/png",
            Some("jpg") | Some("jpeg") => "image/jpeg",
            Some("webp") => "image/webp",
            _ => return Err("unsupported image type".to_string()),
        };
        let path = source.to_string();
        let bytes = tauri::async_runtime::spawn_blocking(move || {
            std::fs::read(&path).map_err(|err| err.to_string())
        })
        .await
        .map_err(|err| err.to_string())??;
        Ok((bytes, mime.to_string()))
    }
}

/// 源图字节 → 处理后的图片字节 + 实际 mime
///
/// - **始终重新编码，不透传原图**：落盘的永远是处理过的版本，percent=100 只压缩不缩放
/// - JPEG 保持 JPEG（q90，编码快）；PNG 保持 PNG；WebP 解码后转 PNG
///   （image 的 WebP 编码只支持无损，又大又慢）
///
/// CPU 密集，调用方放阻塞线程池
fn resize_bg_image(bytes: Vec<u8>, mime: &str, percent: u32) -> Result<(Vec<u8>, String), String> {
    let format = match mime {
        "image/png" => image::ImageFormat::Png,
        "image/jpeg" => image::ImageFormat::Jpeg,
        "image/webp" => image::ImageFormat::WebP,
        other => return Err(format!("unsupported mime: {other}")),
    };
    let img = image::load_from_memory_with_format(&bytes, format).map_err(|err| err.to_string())?;
    let scaled = if percent >= 100 {
        img
    } else {
        // 最小 1px 防止归零
        let scale = percent as f64 / 100.0;
        let w = ((img.width() as f64) * scale).round().max(1.0) as u32;
        let h = ((img.height() as f64) * scale).round().max(1.0) as u32;
        img.resize_exact(w, h, image::imageops::FilterType::CatmullRom)
    };
    let mut out: Vec<u8> = Vec::new();
    let out_mime = match format {
        image::ImageFormat::Jpeg => {
            image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 90)
                .encode_image(&image::DynamicImage::ImageRgb8(scaled.to_rgb8()))
                .map_err(|err| err.to_string())?;
            "image/jpeg"
        }
        // PNG 原样保持；WebP 等转 PNG
        _ => {
            scaled
                .write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png)
                .map_err(|err| err.to_string())?;
            "image/png"
        }
    };
    Ok((out, out_mime.to_string()))
}

/// 加载来源 → 缩放 → 落盘 → 记配置 → 广播刷新，共用的后半段流程
///
/// 返回处理后的 dataURL 供前端立即显示；其他窗口经 `bg-change` 事件刷新
async fn store_bg(
    app: &AppHandle,
    source: String,
    bytes: Vec<u8>,
    mime: String,
    percent: u32,
) -> Result<String, String> {
    use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

    let (data, mime) =
        tauri::async_runtime::spawn_blocking(move || resize_bg_image(bytes, &mime, percent))
            .await
            .map_err(|err| err.to_string())
            .inspect_err(|err| mml_log::error(format!("bg resize join failed: {err}")))?
            .inspect_err(|err| mml_log::error(format!("bg resize failed: {err}")))?;

    // 处理结果落盘（配置目录下，文件名不带扩展名，读取时按魔数识别格式），
    // 各窗口启动时从这里取图
    let Some(dir) = crate::gui_config::dir() else {
        mml_log::error("set_bg failed: config dir not initialized".to_string());
        return Err("config dir not initialized".to_string());
    };
    let write_data = data.clone();
    tauri::async_runtime::spawn_blocking(move || {
        std::fs::write(dir.join("bg_image"), write_data).map_err(|err| err.to_string())
    })
    .await
    .map_err(|err| format!("set_bg write join failed: {err}"))
    .inspect_err(|err| mml_log::error(err.to_string()))?
    .inspect_err(|err| mml_log::error(format!("set_bg write failed: {err}")))?;

    // 来源与分辨率记入配置（set 会异步落盘 gui_config.json）
    let mut config = crate::gui_config::get();
    config.bg_source = source;
    config.bg_native_size = percent;
    crate::gui_config::set(config);

    emit_bg_change(app);
    Ok(format!("data:{mime};base64,{}", BASE64.encode(&data)))
}

/// 下载结果写入原图缓存（仅网址来源）
fn cache_bg_original(source: &str, bytes: &[u8], mime: &str) {
    *BG_ORIGINAL.lock().unwrap() = Some((source.to_string(), bytes.to_vec(), mime.to_string()));
}

/// 设置背景图：每次都按来源重新加载（随机图网址每次拿到的是新图，不能吃缓存）→
/// 缩放到 `percent` → 落盘 `bg_image` → 记入配置；下载结果进 `BG_ORIGINAL`
/// 供之后「调原始大小」复用
///
/// 返回处理后的 dataURL 供前端立即显示；其他窗口经 `bg-change` 事件刷新
#[tauri::command]
pub async fn settings_set_bg(
    app: AppHandle,
    source: String,
    percent: u32,
) -> Result<String, String> {
    let percent = percent.clamp(10, 100);
    mml_log::info(format!("set_bg: percent={percent} source={source}"));

    let (bytes, mime) = load_bg_bytes(&source).await.inspect_err(|err| {
        mml_log::error(format!("set_bg load failed: {err}"));
    })?;
    if source.starts_with("http://") || source.starts_with("https://") {
        cache_bg_original(&source, &bytes, &mime);
    }
    store_bg(&app, source, bytes, mime, percent).await
}

/// 调整原始大小（点「应用」）：只按新 `percent` 重新缩放已设置的背景图，不改来源
///
/// - 网址来源：用 `BG_ORIGINAL` 里缓存的原图（就是当前显示这张）重新缩放，
///   不重新下载；缓存未命中（重启后）才重新下载——随机图网址此时会换一张图
/// - 本地文件来源：直接重读文件（同一文件每次内容一致）
#[tauri::command]
pub async fn settings_resize_bg(app: AppHandle, percent: u32) -> Result<String, String> {
    let percent = percent.clamp(10, 100);
    let source = crate::gui_config::get().bg_source;
    if source.is_empty() {
        return Err("no background image".to_string());
    }
    mml_log::info(format!("resize_bg: percent={percent} source={source}"));

    let is_url = source.starts_with("http://") || source.starts_with("https://");
    let cached = if is_url {
        BG_ORIGINAL
            .lock()
            .unwrap()
            .as_ref()
            .filter(|(src, _, _)| *src == source)
            .map(|(_, bytes, mime)| (bytes.clone(), mime.clone()))
    } else {
        None
    };
    let (bytes, mime) = match cached {
        Some(hit) => hit,
        None => {
            let (bytes, mime) = load_bg_bytes(&source).await.inspect_err(|err| {
                mml_log::error(format!("resize_bg load failed: {err}"));
            })?;
            if is_url {
                cache_bg_original(&source, &bytes, &mime);
            }
            (bytes, mime)
        }
    };
    store_bg(&app, source, bytes, mime, percent).await
}

/// 清除背景图：删除落盘文件并清空配置里的来源
#[tauri::command]
pub async fn settings_clear_bg(app: AppHandle) -> Result<(), String> {
    if let Some(dir) = crate::gui_config::dir() {
        let _ = tauri::async_runtime::spawn_blocking(move || {
            std::fs::remove_file(dir.join("bg_image"))
        })
        .await;
    }
    let mut config = crate::gui_config::get();
    config.bg_source = String::new();
    crate::gui_config::set(config);
    emit_bg_change(&app);
    Ok(())
}

/// 读取背景图（窗口启动 / `bg-change` 时调用）：源地址 + 处理后的图 + 显示参数；
/// 未设置背景图时返回 None
#[tauri::command]
pub fn settings_get_bg() -> Result<Option<BgInfoDto>, String> {
    use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

    let config = crate::gui_config::get();
    if config.bg_source.is_empty() {
        return Ok(None);
    }
    let Some(dir) = crate::gui_config::dir() else {
        return Ok(None);
    };
    let bytes = std::fs::read(dir.join("bg_image"))
        .inspect_err(|err| mml_log::error(format!("get_bg read failed: {err}")))
        .map_err(|err| err.to_string())?;
    let mime = sniff_bg_mime(&bytes);
    Ok(Some(BgInfoDto {
        source: config.bg_source,
        data_url: format!("data:{mime};base64,{}", BASE64.encode(bytes)),
        opacity: config.bg_opacity,
        blur: config.bg_blur,
        native_size: config.bg_native_size,
    }))
}

/// 从文件头嗅探图片 mime（PNG / JPEG / WebP 魔数；未识别按 PNG 处理）
fn sniff_bg_mime(bytes: &[u8]) -> &'static str {
    if bytes.starts_with(&[0x89, b'P', b'N', b'G']) {
        "image/png"
    } else if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        "image/jpeg"
    } else if bytes.len() > 12 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WEBP" {
        "image/webp"
    } else {
        "image/png"
    }
}

/// 背景图变更事件（跨窗口同步刷新背景层）
#[gui_macros::emit]
fn emit_bg_change(app: &AppHandle) {
    let _ = app.emit(listens::BG_CHANGE, ());
}

// ================= 网络设置 =================

/// 读取网络与下载设置（core config.json 的 HttpObj + DnsObj + GameCheckObj）
#[tauri::command]
pub fn settings_get_network() -> NetworkSettingDto {
    let config = mml_config::read_config();
    let mut dto = NetworkSettingDto::from(&config.http);
    dto.dns = DnsSettingDto::from(&config.dns);
    dto.check = GameCheckSettingDto::from(&config.check);
    dto
}

/// 保存网络与下载设置（Http / DNS / 游戏文件检查一次落盘）
#[tauri::command]
pub fn settings_save_network(dto: NetworkSettingDto) {
    let mut config = mml_config::write_config();
    config.http = dto.clone().into();
    config.dns = dto.dns.into();
    config.check = dto.check.into();
    drop(config);
    mml_config::save();
}

// ================= 游戏启动设置 =================

/// 读取启动设置（Java 列表 + 内存 + 自定义参数）
#[tauri::command]
pub fn settings_get_launch() -> LaunchSettingDto {
    let mut dto = LaunchSettingDto::from(&mml_config::read_config().jvm_arg);
    dto.java_list = java_list();
    dto
}

/// 保存内存与自定义参数（Java 列表走独立命令增删）
#[tauri::command]
pub fn settings_save_launch(min_memory: u32, max_memory: u32, jvm_args: String, game_args: String) {
    let mut config = mml_config::write_config();
    let run = &mut config.jvm_arg;
    run.min_memory = Some(min_memory);
    run.max_memory = Some(max_memory);
    run.jvm_args = Some(jvm_args);
    run.game_args = Some(game_args);
    drop(config);
    mml_config::save();
}

/// 扫描系统已安装的 Java 并持久化到配置
#[tauri::command]
pub fn settings_scan_java() -> Vec<JavaInfoDto> {
    mml_jvms::scan_java();
    java_list()
}

/// 手动添加一个 Java（路径无效返回错误）
#[tauri::command]
pub fn settings_add_java(name: Option<String>, path: String) -> Result<JavaInfoDto, String> {
    if let Some(added) = mml_jvms::add_item(name, path) {
        Ok(JavaInfoDto {
            name: added.name.clone(),
            path: added.path.to_string_lossy().to_string(),
            version: added.version.clone(),
            major: added.major_version.max(0),
            java_type: added.java_type.clone(),
            arch: added.arch.to_string(),
        })
    } else {
        Err("err.javaInvalid".to_string())
    }
}

/// 移除一个 Java（按名称）
#[tauri::command]
pub fn settings_remove_java(name: String) {
    mml_jvms::remove(&name);
}

/// 移除全部 Java，返回刷新后的列表
#[tauri::command]
pub fn settings_remove_all_java() {
    mml_jvms::remove_all();
}

/// Java 压缩包导入进度事件（解包 / 识别阶段推进时发）
#[gui_macros::emit]
pub fn emit_settings_java_progress(app: &AppHandle, dto: JavaImportProgressDto) {
    let _ = app.emit(listens::SETTINGS_JAVA_PROGRESS, dto);
}

/// 解压进度回调 → `settings-java-progress` 事件桥接
///
/// 给 `mml_jvms::unzip_java` 的 `BaseArchiveGui` 参数用：
/// `start` 下发的 total 记在内部，`update` 携带它一起转发成进度事件，
/// 当前文件名放进 `subText`；命令返回即导入完成，`done` 无需再发。
struct JavaArchiveGui {
    app: AppHandle,
    /// `start` 下发的总数，`update` 时回读
    total: AtomicUsize,
}

impl JavaArchiveGui {
    fn new(app: &AppHandle) -> Arc<Self> {
        Arc::new(Self {
            app: app.clone(),
            total: AtomicUsize::new(0),
        })
    }
}

impl IBaseArchiveGui for JavaArchiveGui {
    fn start(&self, total: usize) {
        self.total.store(total, Ordering::Relaxed);
        emit_settings_java_progress(
            &self.app,
            JavaImportProgressDto {
                state: "extract".to_string(),
                now: 0,
                total: total as u32,
                sub_text: None,
            },
        );
    }

    fn update(&self, filename: Option<String>, current: usize) {
        emit_settings_java_progress(
            &self.app,
            JavaImportProgressDto {
                state: "extract".to_string(),
                now: current as u32,
                total: self.total.load(Ordering::Relaxed) as u32,
                sub_text: filename,
            },
        );
    }

    fn done(&self) {}

    /// 导入场景无需询问，非法字符文件名直接同意替换
    fn file_rename(&self, _name: &str) -> bool {
        true
    }
}

/// 从压缩包（zip / 7z / tar.gz 等）解包导入 Java，返回刷新后的列表。
/// 解包与识别过程中经 `settings-java-progress` 事件上报进度
#[tauri::command]
pub async fn settings_import_java(
    app: AppHandle,
    name: Option<String>,
    archive_path: String,
) -> Result<Option<JavaInfoDto>, String> {
    let data = mml_jvms::unzip_java(
        name,
        Path::new(&archive_path),
        Some(JavaArchiveGui::new(&app)),
    )
    .await
    .map_err(|err| err.to_string())?;
    if let Some(data) = data {
        Ok(Some(JavaInfoDto {
            name: data.name.clone(),
            path: data.path.to_string_lossy().to_string(),
            version: data.version.clone(),
            major: data.major_version,
            java_type: data.java_type.clone(),
            arch: data.arch.to_string(),
        }))
    } else {
        Ok(None)
    }
}

/// 扫描指定文件夹（含子目录）里的 Java 并注册，返回刷新后的列表
#[tauri::command]
pub async fn settings_scan_java_dir(path: String) -> Result<Option<JavaInfoDto>, String> {
    let data = mml_jvms::find_java_from_path(Path::new(&path));

    let Some(data) = data else {
        return Ok(None);
    };

    let data = tauri::async_runtime::spawn(async move { java_helper::test_java(Path::new(&data)) })
        .await
        .map_err(|err| err.to_string())?;

    if let Some(info) = data {
        let info = mml_jvms::add_info_item(info);
        return Ok(Some(JavaInfoDto {
            name: info.name.clone(),
            path: info.path.to_string_lossy().to_string(),
            version: info.version.clone(),
            major: info.major_version,
            java_type: info.java_type.clone(),
            arch: info.arch.to_string(),
        }));
    } else {
        return Ok(None);
    }
}

/// 测试获取 Java 信息（不注册）：探测路径指向的 Java，返回识别到的名称 / 版本等
#[tauri::command]
pub async fn settings_detect_java(path: String) -> Result<Option<String>, String> {
    let data = tauri::async_runtime::spawn(async move { java_helper::test_java(Path::new(&path)) })
        .await
        .map_err(|err| err.to_string())?;

    if let Some(data) = data {
        Ok(Some(data.name))
    } else {
        Ok(None)
    }
}

/// mml-jvms 内存列表 → 前端 DTO（与 windows::main 的 java_list 同构）
fn java_list() -> Vec<JavaInfoDto> {
    mml_jvms::get_all_java()
        .iter()
        .map(|j| JavaInfoDto {
            name: j.name.clone(),
            path: j.path.to_string_lossy().to_string(),
            version: j.version.clone(),
            major: j.major_version.max(0),
            java_type: j.java_type.clone(),
            arch: j.arch.to_string(),
        })
        .collect()
}
