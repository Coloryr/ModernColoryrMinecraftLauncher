//! 设置窗口：规格 + IPC 命令（系统字体枚举 / 网络设置 / 启动设置）
//!
//! 网络设置落在 core `config.json` 的 `HttpObj`，启动设置落在 `RunArgObj`
//! 与 Java 列表（mml-jvms，增删即时持久化）。

use std::path::Path;

use tauri::{AppHandle, Emitter};

use crate::dtos::{
    BgInfoDto, DnsSettingDto, GameCheckSettingDto, JavaInfoDto, LaunchSettingDto, NetworkSettingDto,
};
use crate::listens;

/// mml-jvms 内存列表 → 前端 DTO（与 windows::main 的 java_list 同构）
fn java_list() -> Vec<JavaInfoDto> {
    mml_jvms::get_all_java()
        .iter()
        .map(|j| JavaInfoDto {
            name: j.name.clone(),
            path: j.path.to_string_lossy().to_string(),
            version: j.version.clone(),
            // 遗留占位条目（Java 失效）主版本号为 -1，前端用 0 表示未知
            major: j.major_version.max(0) as u32,
            java_type: j.java_type.clone(),
            arch: j.arch.to_string(),
        })
        .collect()
}

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

/// 背景图来源加载：本地文件路径 / 网址 → 原始字节 + mime
///
/// - 网址（http/https）：走 mml-net 客户端下载，mime 取响应的 Content-Type
/// - 其余按本地文件路径处理，mime 由扩展名推断（仅支持 png / jpeg / webp）
async fn load_bg_bytes(source: &str) -> Result<(Vec<u8>, String), String> {
    if source.starts_with("http://") || source.starts_with("https://") {
        let client = mml_net::Client::new(mml_config::config_obj::ProxyState::Auto);
        let resp = client.get(source).await.map_err(|err| err.to_string())?;
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
        let bytes = resp.bytes().await.map_err(|err| err.to_string())?;
        Ok((bytes.to_vec(), mime))
    } else {
        let mime = match source.rsplit('.').next().map(|e| e.to_lowercase()).as_deref() {
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

/// 源图字节 → 按 `percent` 缩放后的图片字节 + 实际 mime
///
/// - 100% 不解码不重编码，直接透传原始字节（大图编 PNG 很慢，能免则免）
/// - JPEG 保持 JPEG（q90，编码快）；PNG 保持 PNG；WebP 解码后转 PNG
///   （image 的 WebP 编码只支持无损，又大又慢）
///
/// CPU 密集，调用方放阻塞线程池
fn resize_bg_image(bytes: Vec<u8>, mime: &str, percent: u32) -> Result<(Vec<u8>, String), String> {
    if percent >= 100 {
        return Ok((bytes, mime.to_string()));
    }
    let format = match mime {
        "image/png" => image::ImageFormat::Png,
        "image/jpeg" => image::ImageFormat::Jpeg,
        "image/webp" => image::ImageFormat::WebP,
        other => return Err(format!("unsupported mime: {other}")),
    };
    let img = image::load_from_memory_with_format(&bytes, format).map_err(|err| err.to_string())?;
    // 最小 1px 防止归零
    let scale = percent as f64 / 100.0;
    let w = ((img.width() as f64) * scale).round().max(1.0) as u32;
    let h = ((img.height() as f64) * scale).round().max(1.0) as u32;
    let resized = img.resize_exact(w, h, image::imageops::FilterType::CatmullRom);
    let mut out: Vec<u8> = Vec::new();
    let out_mime = match format {
        image::ImageFormat::Jpeg => {
            image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 90)
                .encode_image(&image::DynamicImage::ImageRgb8(resized.to_rgb8()))
                .map_err(|err| err.to_string())?;
            "image/jpeg"
        }
        // PNG 原样保持；WebP 等转 PNG
        _ => {
            resized
                .write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png)
                .map_err(|err| err.to_string())?;
            "image/png"
        }
    };
    Ok((out, out_mime.to_string()))
}

/// 设置背景图：按来源加载 → 缩放到 `percent` → 落盘 `bg_image.png` → 记入配置
///
/// 返回处理后的 dataURL 供前端立即显示；其他窗口经 `bg-change` 事件刷新
#[tauri::command]
pub async fn settings_set_bg(
    app: AppHandle,
    source: String,
    percent: u32,
) -> Result<String, String> {
    use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

    let percent = percent.clamp(10, 100);
    mml_log::info(format!("set_bg: percent={percent} source={source}"));

    let (bytes, mime) = load_bg_bytes(&source).await.inspect_err(|err| {
        mml_log::error(format!("set_bg load failed: {err}"));
    })?;
    let (data, mime) =
        tauri::async_runtime::spawn_blocking(move || resize_bg_image(bytes, &mime, percent))
            .await
            .map_err(|err| err.to_string())
            .inspect_err(|err| mml_log::error(format!("set_bg resize join failed: {err}")))?
            .inspect_err(|err| mml_log::error(format!("set_bg resize failed: {err}")))?;

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

    emit_bg_change(&app);
    Ok(format!("data:{mime};base64,{}", BASE64.encode(&data)))
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
    persist_java();
    java_list()
}

/// 手动添加一个 Java（路径无效返回错误）
#[tauri::command]
pub fn settings_add_java(path: String) -> Result<JavaInfoDto, String> {
    // 名称缺省用可执行文件所在目录名（如 "jdk-17.0.2"）
    let name = Path::new(&path)
        .parent()
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "java".to_string());

    let Some(added) = mml_jvms::add_item(name, path) else {
        return Err("err.javaInvalid".to_string());
    };
    persist_java();
    mml_jvms::get_all_java()
        .into_iter()
        .find(|j| j.name == added)
        .map(|j| JavaInfoDto {
            name: j.name.clone(),
            path: j.path.to_string_lossy().to_string(),
            version: j.version.clone(),
            major: j.major_version.max(0) as u32,
            java_type: j.java_type.clone(),
            arch: j.arch.to_string(),
        })
        .ok_or_else(|| "err.javaInvalid".to_string())
}

/// 移除一个 Java（按名称）
#[tauri::command]
pub fn settings_remove_java(name: String) -> Vec<JavaInfoDto> {
    mml_jvms::remove(&name);
    java_list()
}

/// 把 mml-jvms 内存列表里尚未入配置的条目补进 config.json（scan_java 只写内存）
fn persist_java() {
    let all = mml_jvms::get_all_java();
    let mut dirty = false;
    {
        let mut config = mml_config::write_config();
        for j in &all {
            if !config.java_list.iter().any(|x| x.name == j.name) {
                config.java_list.push(mml_config::config_obj::JvmConfigObj {
                    name: j.name.clone(),
                    local: j.path.to_string_lossy().to_string(),
                });
                dirty = true;
            }
        }
    }
    if dirty {
        mml_config::save();
    }
}
