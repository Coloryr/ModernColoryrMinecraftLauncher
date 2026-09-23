use std::{
    collections::HashMap,
    fs,
    io::Cursor,
    path::{Path, PathBuf},
    str::FromStr,
    sync::{LazyLock, OnceLock, RwLock},
    time::{Duration, Instant},
};

use mml_auth::{UserKeyObj, auths};
use mml_base::hash_helper::{self, HashType};
use mml_game::player_skin;
use mml_names::names;
use mml_net::mojang_api;
use mml_skin::SkinType;
use mml_skin_draw::{cape_2d_draw, head_2d_draw, head_3d_draw, skin_2d_draw};
use mml_sys::path_helper;
use tiny_skia::Pixmap;
use tauri::{
    UriSchemeResponder,
    http::{Request, Response, StatusCode},
};
use uuid::Uuid;

use crate::{
    dtos::account_dto::auth_type_from_str,
    gui_config::{self, HeadType},
};

static INSTANCE_IMAGE: LazyLock<RwLock<HashMap<Uuid, Vec<u8>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));
static SKIN_IMAGE: LazyLock<RwLock<HashMap<UserKeyObj, Vec<u8>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));
static HEAD_IMAGE: LazyLock<RwLock<HashMap<UserKeyObj, Vec<u8>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));
static SKIN2D_IMAGE: LazyLock<RwLock<HashMap<(UserKeyObj, String), Vec<u8>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));
static SKINRAW_IMAGE: LazyLock<RwLock<HashMap<UserKeyObj, Vec<u8>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));
static CAPERAW_IMAGE: LazyLock<RwLock<HashMap<UserKeyObj, Vec<u8>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));
static CAPE2D_IMAGE: LazyLock<RwLock<HashMap<UserKeyObj, Vec<u8>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));
static ICON_IMAGE: LazyLock<RwLock<HashMap<String, IconCache>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));
/// 图标磁盘缓存目录（`<运行目录>/image`）
static ICON_DIR: OnceLock<PathBuf> = OnceLock::new();

/// 图标内存缓存存活时长，超时后删除并回源重新下载
const ICON_TIMEOUT: Duration = Duration::from_secs(60);

/// 图标磁盘缓存的新鲜窗口（按文件修改时间判断）。
/// 过期后不再直接删掉重下，而是带 If-None-Match 去服务器校验：
/// 304 就续用本地（重写文件刷新时间戳），200 才真正重新下载
const DISK_ICON_TIMEOUT: Duration = Duration::from_secs(24 * 3600);

/// 图标缓存条目
#[derive(Clone)]
struct IconCache {
    /// 图片数据
    data: Vec<u8>,
    /// 内容类型
    mime: &'static str,
    /// 写入时刻
    time: Instant,
}

/// 图标请求名（网址的 sha256）→ 远程网址
static URL_IMAGE: LazyLock<RwLock<HashMap<String, String>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

/// 把请求路径按 `/` 拆成非空片段
///
/// Tauri 传入的 path 形如 `/instance/<uuid>`，始终带前导 `/`，
/// 直接 `split('/')` 会在首位多出一个空串。
fn split_path(path: &str) -> Vec<&str> {
    path.split('/').filter(|item| !item.is_empty()).collect()
}

fn send_png(res: UriSchemeResponder, data: Vec<u8>) {
    res.respond(
        Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "image/png")
            .body(data)
            .unwrap(),
    );
}

// fn send_jpeg(res: UriSchemeResponder, data: Vec<u8>) {
//     res.respond(
//         Response::builder()
//             .status(StatusCode::OK)
//             .header("Content-Type", "image/jpeg")
//             .body(data)
//             .unwrap(),
//     );
// }

fn send_bad(res: UriSchemeResponder) {
    res.respond(
        Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(&[0u8; 0])
            .unwrap(),
    );
}

/// 图标内容：能解码的统一转 PNG，解不动的（gif / avif / 动图 webp / svg 等）
/// 按内容嗅探后原样透传，交给 webview 自己渲染
struct IconBytes {
    data: Vec<u8>,
    mime: &'static str,
}

/// 按文件头嗅探图片类型
fn sniff_mime(data: &[u8]) -> Option<&'static str> {
    if data.starts_with(&[0x89, b'P', b'N', b'G']) {
        Some("image/png")
    } else if data.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some("image/jpeg")
    } else if data.starts_with(b"GIF8") {
        Some("image/gif")
    } else if data.len() > 12 && &data[0..4] == b"RIFF" && &data[8..12] == b"WEBP" {
        Some("image/webp")
    } else if data.len() > 12 && &data[4..8] == b"ftyp" && (&data[8..12] == b"avif" || &data[8..12] == b"avis")
    {
        Some("image/avif")
    } else if data.starts_with(b"<?xml") || data.starts_with(b"<svg") {
        Some("image/svg+xml")
    } else if data.starts_with(b"BM") {
        Some("image/bmp")
    } else {
        None
    }
}

/// 归一化图标：优先转 PNG，解不动再原样透传
fn normalize_icon(data: &[u8]) -> Option<IconBytes> {
    if let Some(png) = decode_as_png(data) {
        return Some(IconBytes {
            data: png,
            mime: "image/png",
        });
    }

    let mime = sniff_mime(data)?;
    Some(IconBytes {
        data: data.to_vec(),
        mime,
    })
}

fn send_icon(res: UriSchemeResponder, icon: &IconBytes) {
    res.respond(
        Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", icon.mime)
            .body(icon.data.clone())
            .unwrap(),
    );
}

fn read_as_png<P: AsRef<Path>>(file: P) -> Option<Vec<u8>> {
    decode_as_png(&fs::read(file).ok()?)
}

fn load_instance_image(uri: &[&str], res: UriSchemeResponder) {
    if uri.len() != 2 {
        send_bad(res);
        return;
    }
    let uuid = Uuid::from_str(uri[1]);
    if uuid.is_err() {
        send_bad(res);
        return;
    }
    let uuid = uuid.unwrap();
    let image = INSTANCE_IMAGE.read().unwrap().get(&uuid).cloned();
    if let Some(data) = image {
        send_png(res, data.clone());
        return;
    }

    let instance = mml_game::get_instance(&uuid);
    if instance.is_none() {
        send_bad(res);
        return;
    }

    let instance = instance.unwrap();

    let icon = instance.read().unwrap().get_icon_file();
    if !icon.exists() || !icon.is_file() {
        send_bad(res);
        return;
    }
    let image = read_as_png(icon);
    if image.is_none() {
        send_bad(res);
        return;
    }

    let image = image.unwrap();
    INSTANCE_IMAGE
        .write()
        .unwrap()
        .insert(uuid.clone(), image.clone());
    send_png(res, image);
}

async fn load_skin_image(uri: &[&str], res: UriSchemeResponder) {
    if uri.len() != 3 {
        send_bad(res);
        return;
    }

    let Some(files) = resolve_skin_files(uri).await else {
        send_bad(res);
        return;
    };
    let key = files.key;
    let Some(file) = files.skin else {
        send_bad(res);
        return;
    };

    {
        let image = HEAD_IMAGE.read().unwrap().get(&key).cloned();
        if let Some(data) = image {
            send_png(res, data);
            return;
        }
    }

    let Some(bitmap) = mml_skin::open_bitmap(&file) else {
        send_bad(res);
        return;
    };

    let Some(head) = gen_head_image(&bitmap) else {
        send_bad(res);
        return;
    };

    let Ok(data) = head.encode_png() else {
        send_bad(res);
        return;
    };

    HEAD_IMAGE.write().unwrap().insert(key, data.clone());

    send_png(res, data);
}

/// 账户皮肤与披风的本地缓存文件
struct SkinFiles {
    key: UserKeyObj,
    skin: Option<PathBuf>,
    cape: Option<PathBuf>,
}

/// 皮肤类URI（`skin*/cape*/<账户类型>/<uuid>`）解析出账户并取回皮肤/披风文件
///
/// 账户不存在返回None；皮肤/披风某一项可能没有（如离线账户都没有）；
/// 下载结果由player_skin层缓存到本地皮肤目录
async fn resolve_skin_files(uri: &[&str]) -> Option<SkinFiles> {
    let user_type = auth_type_from_str(uri.get(1)?);
    let user = auths::get(uri.get(2)?, user_type)?;
    let res = player_skin::download_skin(&user).await;
    Some(SkinFiles {
        key: user.get_key(),
        skin: res.skin,
        cape: res.cape,
    })
}

/// 皮肤全身2D平面图（skin_2d_draw_typea，128×256）
///
/// 第4段为皮肤类型：`auto`（自动检测，可省略）/ `old`（1.7旧版）/ `new`（1.8新版）/ `slim`（纤细）
async fn load_skin2d_image(uri: &[&str], res: UriSchemeResponder) {
    if uri.len() != 3 && uri.len() != 4 {
        send_bad(res);
        return;
    }
    let skin_type = uri.get(3).copied().unwrap_or("auto");
    if parse_skin_type(skin_type).is_none() {
        send_bad(res);
        return;
    }

    let Some(files) = resolve_skin_files(uri).await else {
        send_bad(res);
        return;
    };
    let key = (files.key, skin_type.to_string());
    let Some(file) = files.skin else {
        send_bad(res);
        return;
    };

    {
        let image = SKIN2D_IMAGE.read().unwrap().get(&key).cloned();
        if let Some(data) = image {
            send_png(res, data);
            return;
        }
    }

    let Some(bitmap) = mml_skin::open_bitmap(&file) else {
        send_bad(res);
        return;
    };
    let Some(image) = skin_2d_draw::skin_2d_draw_typea(&bitmap, parse_skin_type(skin_type).flatten())
    else {
        send_bad(res);
        return;
    };
    let Ok(data) = image.encode_png() else {
        send_bad(res);
        return;
    };

    SKIN2D_IMAGE.write().unwrap().insert(key, data.clone());

    send_png(res, data);
}

/// URI里的皮肤类型段 → SkinType（None = 自动检测）
fn parse_skin_type(s: &str) -> Option<Option<SkinType>> {
    match s {
        "auto" => Some(None),
        "old" => Some(Some(SkinType::Old)),
        "new" => Some(Some(SkinType::New)),
        "slim" => Some(Some(SkinType::NewSlim)),
        _ => None,
    }
}

/// 原始皮肤贴图PNG（供前端 skinview3d 做3D渲染）
async fn load_skinraw_image(uri: &[&str], res: UriSchemeResponder) {
    if uri.len() != 3 {
        send_bad(res);
        return;
    }

    let Some(files) = resolve_skin_files(uri).await else {
        send_bad(res);
        return;
    };
    let key = files.key;
    let Some(file) = files.skin else {
        send_bad(res);
        return;
    };

    {
        let image = SKINRAW_IMAGE.read().unwrap().get(&key).cloned();
        if let Some(data) = image {
            send_png(res, data);
            return;
        }
    }

    let Ok(data) = path_helper::read_byte(&file) else {
        send_bad(res);
        return;
    };

    SKINRAW_IMAGE.write().unwrap().insert(key, data.clone());

    send_png(res, data);
}

/// 原始披风贴图PNG（供前端 skinview3d 挂载）
async fn load_caperaw_image(uri: &[&str], res: UriSchemeResponder) {
    if uri.len() != 3 {
        send_bad(res);
        return;
    }

    let Some(files) = resolve_skin_files(uri).await else {
        send_bad(res);
        return;
    };
    let key = files.key;
    let Some(file) = files.cape else {
        send_bad(res);
        return;
    };

    {
        let image = CAPERAW_IMAGE.read().unwrap().get(&key).cloned();
        if let Some(data) = image {
            send_png(res, data);
            return;
        }
    }

    let Ok(data) = path_helper::read_byte(&file) else {
        send_bad(res);
        return;
    };

    CAPERAW_IMAGE.write().unwrap().insert(key, data.clone());

    send_png(res, data);
}

/// 披风2D平面图（cape_2d_draw 正面）
async fn load_cape2d_image(uri: &[&str], res: UriSchemeResponder) {
    if uri.len() != 3 {
        send_bad(res);
        return;
    }

    let Some(files) = resolve_skin_files(uri).await else {
        send_bad(res);
        return;
    };
    let key = files.key;
    let Some(file) = files.cape else {
        send_bad(res);
        return;
    };

    {
        let image = CAPE2D_IMAGE.read().unwrap().get(&key).cloned();
        if let Some(data) = image {
            send_png(res, data);
            return;
        }
    }

    let Some(bitmap) = mml_skin::open_bitmap(&file) else {
        send_bad(res);
        return;
    };
    let Some(image) = cape_2d_draw::draw_cape_2d(&bitmap) else {
        send_bad(res);
        return;
    };
    let Ok(data) = image.encode_png() else {
        send_bad(res);
        return;
    };

    CAPE2D_IMAGE.write().unwrap().insert(key, data.clone());

    send_png(res, data);
}

/// 按配置的头像类型渲染头像
fn gen_head_image(bitmap: &Pixmap) -> Option<Pixmap> {
    let config = gui_config::get().head;

    match config.head_type {
        HeadType::Head3DA => head_3d_draw::draw_head_3d_typea(bitmap),
        HeadType::Head3DB => head_3d_draw::draw_head_3d_typeb(bitmap, config.x, config.y),
        HeadType::Head2DB => head_2d_draw::head_2d_draw_typeb(bitmap),
        HeadType::Head2DA => head_2d_draw::head_2d_draw_typea(bitmap),
    }
}

async fn load_icon_image(uri: &[&str], res: UriSchemeResponder) {
    if uri.len() != 2 {
        send_bad(res);
        return;
    }

    let name = uri[1];

    let url = URL_IMAGE.read().unwrap().get(name).cloned();
    let Some(url) = url else {
        send_bad(res);
        return;
    };

    // 内存缓存
    if let Some(icon) = get_icon_image(name) {
        send_icon(
            res,
            &IconBytes {
                data: icon.data,
                mime: icon.mime,
            },
        );
        return;
    }

    let file = icon_file(&url);
    let etag_file = icon_etag_file(&url);

    // 磁盘缓存的本地副本（有文件但过期时保留，供 304 续用 / 网络失败兜底）
    let cached = read_icon_bytes(&file);
    if cached.is_none() && file.is_file() {
        // 内容损坏：删掉重新下载
        let _ = path_helper::delete(&file);
    }

    // 磁盘缓存仍新鲜：直接用，不发请求
    if let Some(icon) = &cached {
        if !is_icon_expired(&file) {
            set_icon_image(name, icon.data.clone(), icon.mime);
            send_icon(res, icon);
            return;
        }
    }

    // 过期 / 缺失：回源。存有 ETag（两家 CDN 的 ETag 都是图片内容的 MD5）就带
    // If-None-Match 校验，图没变时 304 直接续用本地，不用重新下载
    let etag = read_icon_etag(&etag_file);
    match mojang_api::get_assets_with_check(&url, etag.as_deref()).await {
        Ok(check) if check.not_modified => {
            if let Some(icon) = &cached {
                // 重写一份刷新修改时间，把新鲜窗口清零
                write_icon_file(&file, &icon.data);
                set_icon_image(name, icon.data.clone(), icon.mime);
                send_icon(res, icon);
            } else {
                // 只有 ETag 没有图片（异常状态），重新下载
                let _ = path_helper::delete(&etag_file);
                send_bad(res);
            }
        }
        Ok(check) => {
            let Some(icon) = normalize_icon(&check.data) else {
                send_bad(res);
                return;
            };

            if let Some(etag) = check.etag {
                write_icon_etag(&etag_file, &etag);
            }
            write_icon_file(&file, &icon.data);
            set_icon_image(name, icon.data.clone(), icon.mime);

            send_icon(res, &icon);
        }
        Err(_) => {
            // 网络失败：有过期的本地副本就先用着
            if let Some(icon) = &cached {
                set_icon_image(name, icon.data.clone(), icon.mime);
                send_icon(res, icon);
            } else {
                send_bad(res);
            }
        }
    }
}

/// 取图标缓存，已过期的条目会被删除并返回 `None`（由调用方回源）
fn get_icon_image(info: &str) -> Option<IconCache> {
    let mut lock = ICON_IMAGE.write().unwrap();
    let alive = lock
        .get(info)
        .is_some_and(|item| item.time.elapsed() < ICON_TIMEOUT);

    if alive {
        return lock.get(info).cloned();
    }

    lock.remove(info);

    None
}

/// 写入图标缓存并记录时刻
fn set_icon_image(info: &str, data: Vec<u8>, mime: &'static str) {
    ICON_IMAGE.write().unwrap().insert(
        info.to_string(),
        IconCache {
            data,
            mime,
            time: Instant::now(),
        },
    );
}

/// 初始化图标磁盘缓存目录（启动时调用）
pub fn init<P: AsRef<Path>>(path: P) {
    ICON_DIR.get_or_init(|| path.as_ref().join(names::IMAGE_DIR));
}

/// 方块贴图（`mml-image/block/<方块ID>`）：
/// PNG 直接读盘透传，不进内存缓存——重渲染后立即生效；
/// webview 自身的缓存由 URL 上的 `?v=<版本>` 破掉
fn load_block_image(uri: &[&str], res: UriSchemeResponder) {
    if uri.len() != 2 {
        send_bad(res);
        return;
    }
    let Some(file) = mml_tex_draw::get_block_path(uri[1]) else {
        send_bad(res);
        return;
    };
    match path_helper::read_byte(&file) {
        Ok(data) => send_png(res, data),
        Err(_) => send_bad(res),
    }
}

/// 实例图标内存缓存失效（实例图标被外部更换后调用，让下次请求重读磁盘）
pub fn clear_instance_image(uuid: &Uuid) {
    INSTANCE_IMAGE.write().unwrap().remove(uuid);
}

/// 实例截图（`mml-image/screenshot/<实例uuid>/<文件名>`）：
/// PNG 读盘透传，不进内存缓存——游戏运行中新增截图下次请求即可见
fn load_screenshot_image(uri: &[&str], res: UriSchemeResponder) {
    if uri.len() != 3 {
        send_bad(res);
        return;
    }
    let Ok(uuid) = Uuid::from_str(uri[1]) else {
        send_bad(res);
        return;
    };
    // 文件名必须是纯文件名（防路径穿越）
    let name = Path::new(uri[2]);
    if name.file_name() != Some(name.as_os_str()) {
        send_bad(res);
        return;
    }

    let Some(instance) = mml_game::get_instance(&uuid) else {
        send_bad(res);
        return;
    };
    // 锁内只取目录，drop 后再做文件操作
    let dir = instance.read().unwrap().get_screenshots_path();
    let file = dir.join(name);
    if !file.starts_with(&dir) {
        send_bad(res);
        return;
    }
    match path_helper::read_byte(&file) {
        Ok(data) => send_png(res, data),
        Err(_) => send_bad(res),
    }
}

/// 图标请求名：网址的 sha256（十六进制小写，与旧启动器的缓存命名一致）
///
/// 同一个网址每次都得到同一个名字，前端拿到的地址因此是稳定的，
/// 浏览器缓存与磁盘缓存都能直接命中。
fn icon_name(url: &str) -> String {
    hash_helper::gen_hash_from_string(HashType::Sha256, url)
}

/// 远程图片的磁盘缓存位置（`<网址 sha256>.png`）
fn icon_file(url: &str) -> PathBuf {
    ICON_DIR.get().unwrap().join(format!("{}.png", icon_name(url)))
}

/// 图标的 ETag 旁车文件（`<网址 sha256>.etag`）
fn icon_etag_file(url: &str) -> PathBuf {
    ICON_DIR.get().unwrap().join(format!("{}.etag", icon_name(url)))
}

/// 读取记录的 ETag
fn read_icon_etag(file: &Path) -> Option<String> {
    let text = fs::read_to_string(file).ok()?;
    let text = text.trim().to_string();
    (!text.is_empty()).then_some(text)
}

/// 记录 ETag，供下次回源时做 If-None-Match 校验
fn write_icon_etag(file: &Path, etag: &str) {
    let _ = fs::write(file, etag);
}

/// 把图片数据解码后统一转成 PNG，无法解码视为损坏
///
/// 用 `image` 按内容嗅探格式（png / jpeg / webp）
/// skia-safe 的预编译包只带 jpeg / png 解码，Modrinth 的图标是 webp，解不出来。
fn decode_as_png(data: &[u8]) -> Option<Vec<u8>> {
    let image = image::load_from_memory(data).ok()?;
    let mut out = Cursor::new(Vec::new());
    image.write_to(&mut out, image::ImageFormat::Png).ok()?;

    Some(out.into_inner())
}

/// 磁盘缓存是否已过期（按文件修改时间判断）
fn is_icon_expired(file: &Path) -> bool {
    let Ok(time) = fs::metadata(file).and_then(|meta| meta.modified()) else {
        return true;
    };

    // 修改时间晚于当前时间（时钟回拨）时按未过期处理，避免反复重新下载
    time.elapsed()
        .map(|elapsed| elapsed >= DISK_ICON_TIMEOUT)
        .unwrap_or(false)
}

/// 读取磁盘缓存的图标内容（不管新鲜度，过期判断由调用方做）
fn read_icon_bytes(file: &Path) -> Option<IconBytes> {
    if !file.is_file() {
        return None;
    }

    path_helper::read_byte(file)
        .ok()
        .and_then(|data| normalize_icon(&data))
}

/// 写入磁盘缓存的图标
fn write_icon_file(file: &Path, data: &[u8]) {
    let _ = path_helper::write_bytes(file, data);
}

pub async fn url_image(req: Request<Vec<u8>>, res: UriSchemeResponder) {
    let uri = split_path(req.uri().path());
    let image_type = uri.first().copied().unwrap_or_default();

    if image_type == "instance" {
        load_instance_image(&uri, res);
    } else if image_type == "skin" {
        load_skin_image(&uri, res).await;
    } else if image_type == "skin2d" {
        load_skin2d_image(&uri, res).await;
    } else if image_type == "skinraw" {
        load_skinraw_image(&uri, res).await;
    } else if image_type == "caperaw" {
        load_caperaw_image(&uri, res).await;
    } else if image_type == "cape2d" {
        load_cape2d_image(&uri, res).await;
    } else if image_type == "icon" {
        load_icon_image(&uri, res).await;
    } else if image_type == "block" {
        load_block_image(&uri, res);
    } else if image_type == "screenshot" {
        load_screenshot_image(&uri, res);
    } else {
        send_bad(res);
    }
}

/// `mml-image` 自定义协议的访问前缀
///
/// Windows / Android 上自定义协议被映射到 `http://<scheme>.localhost`，
/// 其余平台是 `<scheme>://localhost`（与 Tauri `convertFileSrc` 的规则一致）。
pub fn image_base_url() -> &'static str {
    if cfg!(windows) || cfg!(target_os = "android") {
        "http://mml-image.localhost"
    } else {
        "mml-image://localhost"
    }
}

/// 登记远程图片地址并返回可直接用于 `src` 的网址
///
/// 请求名取**网址的 sha256**，所以返回值形如
/// `http://mml-image.localhost/icon/<sha256>`，已含协议前缀，前端无需再拼接；
/// 同一个网址每次返回同一个地址，重复渲染不会产生新地址。
pub fn push_image_url(url: &str) -> String {
    let name = icon_name(url);

    URL_IMAGE
        .write()
        .unwrap()
        .insert(name.clone(), url.to_string());

    format!("{}/icon/{}", image_base_url(), name)
}
