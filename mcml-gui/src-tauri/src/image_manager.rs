use std::{
    collections::HashMap,
    fs,
    io::Cursor,
    path::{Path, PathBuf},
    str::FromStr,
    sync::{LazyLock, OnceLock, RwLock},
    time::{Duration, Instant},
};

use mcml_auth::{AuthType, UserKeyObj, auths};
use mcml_base::hash_helper::{self, HashType};
use mcml_game::player_skin;
use mcml_names::names;
use mcml_net::mojang_api;
use mcml_skin_draw::{head_2d_draw, head_3d_draw};
use mcml_sys::path_helper;
use tiny_skia::Pixmap;
use tauri::{
    UriSchemeResponder,
    http::{Request, Response, StatusCode},
};
use uuid::Uuid;

use crate::{
    gui_config::{self, HeadType},
    windows::add_resource::SourceInfo,
};

static INSTANCE_IMAGE: LazyLock<RwLock<HashMap<Uuid, Vec<u8>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));
static SKIN_IMAGE: LazyLock<RwLock<HashMap<UserKeyObj, Vec<u8>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));
static HEAD_IMAGE: LazyLock<RwLock<HashMap<UserKeyObj, Vec<u8>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));
static ICON_IMAGE: LazyLock<RwLock<HashMap<String, IconCache>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));
/// 图标磁盘缓存目录（`<运行目录>/image`）
static ICON_DIR: OnceLock<PathBuf> = OnceLock::new();

/// 图标缓存存活时长，超时后删除并回源重新下载
const ICON_TIMEOUT: Duration = Duration::from_secs(60);

/// 图标缓存条目
struct IconCache {
    /// PNG 数据
    data: Vec<u8>,
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

    let instance = mcml_game::get_instance(&uuid);
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
    let user_type = AuthType::from_str(uri[1]);
    let uuid = uri[2];

    let user = auths::get(uuid, user_type);
    if user.is_none() {
        send_bad(res);
        return;
    }

    let user = user.unwrap();
    let key = user.get_key();

    {
        let image = HEAD_IMAGE.read().unwrap().get(&key).cloned();
        if let Some(data) = image {
            send_png(res, data);
            return;
        }
    }

    let skin = player_skin::download_skin(&user).await;
    let Some(file) = skin.skin else {
        send_bad(res);
        return;
    };

    let Some(bitmap) = mcml_skin::open_bitmap(&file) else {
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
    if let Some(data) = get_icon_image(name) {
        send_png(res, data);
        return;
    }

    let file = icon_file(&url);

    // 磁盘缓存
    if let Some(data) = read_icon_file(&file) {
        set_icon_image(name, data.clone());
        send_png(res, data);
        return;
    }

    // 回源下载并统一转成 PNG，再写入内存与磁盘
    let Ok(data) = mojang_api::get_assets(&url).await else {
        send_bad(res);
        return;
    };

    let Some(data) = decode_as_png(&data) else {
        send_bad(res);
        return;
    };

    write_icon_file(&file, &data);
    set_icon_image(name, data.clone());

    send_png(res, data);
}

/// 取图标缓存，已过期的条目会被删除并返回 `None`（由调用方回源）
fn get_icon_image(info: &str) -> Option<Vec<u8>> {
    let mut lock = ICON_IMAGE.write().unwrap();
    let alive = lock
        .get(info)
        .is_some_and(|item| item.time.elapsed() < ICON_TIMEOUT);

    if alive {
        return lock.get(info).map(|item| item.data.clone());
    }

    lock.remove(info);

    None
}

/// 写入图标缓存并记录时刻
fn set_icon_image(info: &str, data: Vec<u8>) {
    ICON_IMAGE.write().unwrap().insert(
        info.to_string(),
        IconCache {
            data,
            time: Instant::now(),
        },
    );
}

/// 初始化图标磁盘缓存目录（启动时调用）
pub fn init<P: AsRef<Path>>(path: P) {
    ICON_DIR.get_or_init(|| path.as_ref().join(names::IMAGE_DIR));
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
        .map(|elapsed| elapsed >= ICON_TIMEOUT)
        .unwrap_or(false)
}

/// 读取磁盘缓存的图标，过期或内容损坏时删掉该文件
fn read_icon_file(file: &Path) -> Option<Vec<u8>> {
    if !file.is_file() {
        return None;
    }

    if is_icon_expired(file) {
        let _ = path_helper::delete(file);
        return None;
    }

    match path_helper::read_byte(file)
        .ok()
        .and_then(|data| decode_as_png(&data))
    {
        Some(data) => Some(data),
        None => {
            // 内容损坏：删掉让调用方重新获取
            let _ = path_helper::delete(file);
            None
        }
    }
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
    } else if image_type == "icon" {
        load_icon_image(&uri, res).await;
    } else {
        send_bad(res);
    }
}

/// `mcml-image` 自定义协议的访问前缀
///
/// Windows / Android 上自定义协议被映射到 `http://<scheme>.localhost`，
/// 其余平台是 `<scheme>://localhost`（与 Tauri `convertFileSrc` 的规则一致）。
fn image_base_url() -> &'static str {
    if cfg!(windows) || cfg!(target_os = "android") {
        "http://mcml-image.localhost"
    } else {
        "mcml-image://localhost"
    }
}

/// 登记远程图片地址并返回可直接用于 `src` 的网址
///
/// 请求名取**网址的 sha256**，所以返回值形如
/// `http://mcml-image.localhost/icon/<sha256>`，已含协议前缀，前端无需再拼接；
/// 同一个网址每次返回同一个地址，重复渲染不会产生新地址。
pub fn push_image_url(url: &str) -> String {
    let name = icon_name(url);

    URL_IMAGE
        .write()
        .unwrap()
        .insert(name.clone(), url.to_string());

    format!("{}/icon/{}", image_base_url(), name)
}
