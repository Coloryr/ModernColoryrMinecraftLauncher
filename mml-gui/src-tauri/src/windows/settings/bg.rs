//! 设置窗口的背景图：设置 / 缩放 / 清除 / 读取
//!
//! 从 `settings/mod.rs` 拆出来的。背景原图在内存里留一份（`BG_ORIGINAL`），
//! 缩放时以它为源、避免反复解码已缩放的图。命令带 `#[gui_macros::ipc_group("settings")]`
//! 把组键钉回 `settings`（AGENTS.md §4）。

use std::sync::Mutex;

use tauri::{AppHandle, Emitter};

use crate::dtos::BgInfoDto;
use crate::listens;

/// 最近一次下载的原图缓存（网址来源，仅内存）：调「原始大小」时按来源命中缓存，
/// 直接用原图重新缩放，不重新走网络；换来源或重启后失效（重新下载即可）
pub(super) static BG_ORIGINAL: Mutex<Option<(String, Vec<u8>, String)>> = Mutex::new(None);

/// 背景图来源加载：本地文件路径 / 网址 → 原始字节 + mime
///
/// - 网址（http/https）：走 mml-net 客户端下载，mime 取响应的 Content-Type
/// - 其余按本地文件路径处理，mime 由扩展名推断（仅支持 png / jpeg / webp）
pub(super) async fn load_bg_bytes(source: &str) -> Result<(Vec<u8>, String), String> {
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
pub(super) fn resize_bg_image(
    bytes: Vec<u8>,
    mime: &str,
    percent: u32,
) -> Result<(Vec<u8>, String), String> {
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
pub(super) async fn store_bg(
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
pub(super) fn cache_bg_original(source: &str, bytes: &[u8], mime: &str) {
    *BG_ORIGINAL.lock().unwrap() = Some((source.to_string(), bytes.to_vec(), mime.to_string()));
}

/// 设置背景图：每次都按来源重新加载（随机图网址每次拿到的是新图，不能吃缓存）→
/// 缩放到 `percent` → 落盘 `bg_image` → 记入配置；下载结果进 `BG_ORIGINAL`
/// 供之后「调原始大小」复用
///
/// 返回处理后的 dataURL 供前端立即显示；其他窗口经 `bg-change` 事件刷新
#[gui_macros::ipc_group("settings")]
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
#[gui_macros::ipc_group("settings")]
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
#[gui_macros::ipc_group("settings")]
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
#[gui_macros::ipc_group("settings")]
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
pub(super) fn sniff_bg_mime(bytes: &[u8]) -> &'static str {
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
pub(super) fn emit_bg_change(app: &AppHandle) {
    let _ = app.emit(listens::BG_CHANGE, ());
}
