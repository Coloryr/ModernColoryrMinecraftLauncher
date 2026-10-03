//! 图像管理：`mml-image` 自定义协议的后端
//!
//! 前端用 `image_base_url()` 拼地址，请求经 [`url_image`] 按首段路由：
//! `instance`（实例图标）/
//! `head`（账户头像）/ `skin`（皮肤全身图）/ `cape`（披风正面图）/ `capeback`（披风背面图）
//! ——渲染方式由后端读配置决定，
//! 前端只挑"要哪种图"，不挑"怎么渲染"；
//! `skinraw` / `caperaw`（原始皮肤/披风贴图，供前端 skinview3d 数据用）/
//! `icon`（远程图片，带内存 + 磁盘 + ETag 缓存）/
//! `block`（方块贴图）/ `item`（物品贴图）/ `screenshot`（实例截图）。
//!
//! 皮肤 / 披风纹理一律按**内容 SHA1** 取（与游戏 assets objects 同款布局，
//! 同一份贴图跨账户只存一份）：`skinraw/<sha1>`、`caperaw/<sha1>`、
//! `skin/<sha1>[/<skin_type>]`、`cape/<sha1>`、`capeback/<sha1>`。
//! 账户型 URI（`head/skin/cape*/<账户类型>/<uuid>`）保留给账户列表的
//! "当前选中"预览：解析账户 → 下载定 sha1 → 转调 sha1 型渲染。
//! 渲染缓存键也用内容 sha1，同一贴图的渲染结果跨账户共享。

use std::{
    collections::HashMap,
    fs,
    io::Cursor,
    path::{Path, PathBuf},
    str::FromStr,
    sync::{LazyLock, OnceLock, RwLock},
    time::{Duration, Instant},
};

use mml_auth::auths;
use mml_base::hash_helper::{self, HashType};
use mml_game::{
    launcher_path::assets_path,
    player_skin::{self, TextureFile},
};
use mml_names::names;
use mml_net::{get_work_client, mojang_api};
use mml_skin::{SkinType, skin_type_checker};
use mml_skin_draw::{cape_2d_draw, head_2d_draw, head_3d_draw, skin_2d_draw, skin_3d_draw};
use mml_sys::path_helper;
use tauri::{
    UriSchemeResponder,
    http::{Request, Response, StatusCode},
};
use tiny_skia::Pixmap;
use uuid::Uuid;

use crate::{
    dtos::account_dto::auth_type_from_str,
    gui_config::{self, HeadType, SkinDisplay},
};

/// 实例图标内存缓存
static INSTANCE_IMAGE: LazyLock<RwLock<HashMap<Uuid, Vec<u8>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));
/// 账户头像（按配置类型渲染）内存缓存（键：皮肤内容 sha1 + 头像渲染配置；
/// 同一皮肤的头像跨账户共享）
static HEAD_IMAGE: LazyLock<RwLock<HashMap<(String, String), Vec<u8>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));
/// 皮肤全身渲染图内存缓存（键：内容 sha1 + 显示模式 + 皮肤类型段）
static SKIN_IMAGE: LazyLock<RwLock<HashMap<(String, String), Vec<u8>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));
/// 皮肤原图 PNG 内存缓存（键：内容 sha1）
static SKINRAW_IMAGE: LazyLock<RwLock<HashMap<String, Vec<u8>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));
/// 披风原图 PNG 内存缓存（键：内容 sha1）
static CAPERAW_IMAGE: LazyLock<RwLock<HashMap<String, Vec<u8>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));
/// 披风渲染图内存缓存（键：内容 sha1）
static CAPE_IMAGE: LazyLock<RwLock<HashMap<String, Vec<u8>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));
/// 披风背面渲染图内存缓存（键：内容 sha1）
static CAPE_BACK_IMAGE: LazyLock<RwLock<HashMap<String, Vec<u8>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));
/// 纹理内容 sha1 → 源 URL：本地文件被清后按登记地址重新下载
static TEXTURE_URLS: LazyLock<RwLock<HashMap<String, String>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));
/// 远程图标内存缓存（键为请求名 = 网址 sha256）
static ICON_IMAGE: LazyLock<RwLock<HashMap<String, IconCache>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));
/// 图标磁盘缓存目录（`<运行目录>/image`）
static ICON_DIR: OnceLock<PathBuf> = OnceLock::new();

/// 图标内存缓存存活时长，超时后删除并回源重新下载
const ICON_TIMEOUT: Duration = Duration::from_secs(60);

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

/// 以 `image/png` 响应
fn send_png(res: UriSchemeResponder, data: Vec<u8>) {
    res.respond(
        Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "image/png")
            // skinview3d 取 skinraw/caperaw 上传 WebGL 纹理要过 CORS，必须带本头
            .header("Access-Control-Allow-Origin", "*")
            .body(data)
            .unwrap(),
    );
}

/// 以 400 响应（参数不合法 / 资源不存在）
fn send_bad(res: UriSchemeResponder) {
    res.respond(
        Response::builder()
            .status(StatusCode::BAD_REQUEST)
            // 失败响应同样带 CORS 头，前端 fetch 才能读到失败状态而不是一律 ERR_FAILED
            .header("Access-Control-Allow-Origin", "*")
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
    } else if data.len() > 12
        && &data[4..8] == b"ftyp"
        && (&data[8..12] == b"avif" || &data[8..12] == b"avis")
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
            // 与 send_png 同理：webview 里跨源取图（canvas 绘制等）需要 CORS 头
            .header("Access-Control-Allow-Origin", "*")
            .body(icon.data.clone())
            .unwrap(),
    );
}

/// 读文件并解码转 PNG
///
/// # 参数
///
/// - `file`: 图片文件路径
///
/// # 返回值
///
/// 返回 PNG 数据；读取或解码失败返回 `None`
fn read_as_png<P: AsRef<Path>>(file: P) -> Option<Vec<u8>> {
    decode_as_png(&fs::read(file).ok()?)
}

/// 实例图标（`instance/<uuid>`）：内存缓存命中直接回，未命中按图标来源取图
///
/// 图标与「方块 ID」二选一（见 `InstanceSettingObj::get_icon_file`）：
/// 设了方块 ID 就按 ID 现渲染（方块贴图未渲染完时拿不到图，返回占位失败），
/// 否则读 `icon.png`（上传的自定义图片、以及老实例都走这条）。
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

    // 分支一：设了方块 ID → 用方块渲染结果
    let block_id = {
        let inst = instance.read().unwrap().clone();
        crate::gui_setting::block_icon_id(&inst)
    };
    if let Some(id) = block_id {
        let file = mml_tex_draw::get_block_path(&id).or_else(|| mml_tex_draw::get_item_path(&id));
        let Some(file) = file else {
            // 方块贴图还没渲染 / 该 id 不存在：此时没有可用的图标
            send_bad(res);
            return;
        };
        let Some(image) = read_as_png(file) else {
            send_bad(res);
            return;
        };
        cache_and_send(uuid, image, res);
        return;
    }

    // 分支二：没有方块 ID → 读图标文件（Icon 已清空的实例按默认 icon.png 找）
    let icon = instance.read().unwrap().get_icon_file_or_default();
    if !icon.exists() || !icon.is_file() {
        send_bad(res);
        return;
    }
    let image = read_as_png(icon);
    if image.is_none() {
        send_bad(res);
        return;
    }

    cache_and_send(uuid, image.unwrap(), res);
}

/// 把实例图标写进内存缓存并回给请求方
fn cache_and_send(uuid: Uuid, image: Vec<u8>, res: UriSchemeResponder) {
    INSTANCE_IMAGE
        .write()
        .unwrap()
        .insert(uuid.clone(), image.clone());
    send_png(res, image);
}

/// 账户头像（`head/<账户类型>/<uuid>`）：解析账户当前皮肤后按配置的头像类型渲染
///
/// 保持账户型：账户列表高频渲染，不能每个头像先查一次纹理列表
async fn load_head_image(uri: &[&str], res: UriSchemeResponder) {
    if uri.len() != 3 {
        send_bad(res);
        return;
    }

    // 设置页样例预览（`head/preview/<uuid>`）：内置样例皮肤按当前配置渲染，不查账户
    if uri[1] == "preview" {
        send_head_preview(res);
        return;
    }

    let Some(files) = resolve_skin_files(uri).await else {
        send_bad(res);
        return;
    };
    let Some(tex) = files.skin else {
        send_bad(res);
        return;
    };

    send_head_image(&tex.sha1, res).await;
}

/// 头像渲染（sha1 入口；账户型请求解析出当前皮肤后也转调这里）
///
/// 缓存键带上头像渲染配置：切换头像模式 / 旋转角度后立即出新图而不是旧缓存；
/// 同一皮肤的头像跨账户共享
async fn send_head_image(skin_sha1: &str, res: UriSchemeResponder) {
    let config = gui_config::get().head;
    let key = (
        skin_sha1.to_string(),
        format!("{:?}/{}/{}", config.head_type, config.x, config.y),
    );

    {
        let image = HEAD_IMAGE.read().unwrap().get(&key).cloned();
        if let Some(data) = image {
            send_png(res, data);
            return;
        }
    }

    let Some(bytes) = load_texture_bytes(skin_sha1).await else {
        send_bad(res);
        return;
    };
    let Ok(bitmap) = Pixmap::decode_png(&bytes) else {
        send_bad(res);
        return;
    };

    let Some(image) = gen_head_image(&bitmap) else {
        send_bad(res);
        return;
    };

    let Ok(data) = image.encode_png() else {
        send_bad(res);
        return;
    };

    HEAD_IMAGE.write().unwrap().insert(key, data.clone());

    send_png(res, data);
}

/// 设置页头像样例皮肤（纤细模型，取自 mml-skin 的测试资源）
const PREVIEW_SKIN: &[u8] = include_bytes!("../../../mml-core/mml-skin/tests/skin_slim.png");

/// 设置页头像样例预览：内置皮肤按当前头像配置渲染
///
/// 不查账户、不落缓存（设置页切换配置后靠前端 URL 版本号重取，渲染开销可忽略）
fn send_head_preview(res: UriSchemeResponder) {
    let Some(bitmap) = Pixmap::decode_png(PREVIEW_SKIN).ok() else {
        send_bad(res);
        return;
    };
    let Some(head) = gen_head_image(&bitmap) else {
        send_bad(res);
        return;
    };
    match head.encode_png() {
        Ok(data) => send_png(res, data),
        Err(_) => send_bad(res),
    }
}

/// 设置页皮肤样例预览：内置皮肤按当前皮肤显示模式渲染
///
/// `skin_type` 直接采用 URI 段（样例皮肤是纤细模型，前端固定传 `slim`），不查账户不落缓存
fn send_skin_preview(skin_type: &str, res: UriSchemeResponder) {
    let Some(st) = parse_skin_type(skin_type) else {
        send_bad(res);
        return;
    };
    let Some(bitmap) = Pixmap::decode_png(PREVIEW_SKIN).ok() else {
        send_bad(res);
        return;
    };
    let display = gui_config::get().skin_display;
    let Some(image) = render_skin_display(&bitmap, st, display) else {
        send_bad(res);
        return;
    };
    match image.encode_png() {
        Ok(data) => send_png(res, data),
        Err(_) => send_bad(res),
    }
}

/// 账户皮肤与披风的本地缓存纹理
struct SkinFiles {
    skin: Option<TextureFile>,
    cape: Option<TextureFile>,
    /// 会话服务器 metadata 的纤细标记（权威值；离线等查不到档案时为 false）
    is_new_slim: bool,
}

/// 皮肤类URI账户型（`skin*/cape*/<账户类型>/<uuid>`）解析出账户并取回皮肤/披风纹理
///
/// 账户不存在返回None；皮肤/披风某一项可能没有（如离线账户都没有）；
/// 下载结果由player_skin层按内容SHA1缓存到本地皮肤目录，
/// 同时把源URL登记进 [`TEXTURE_URLS`]，sha1型请求在文件被清后能回源
async fn resolve_skin_files(uri: &[&str]) -> Option<SkinFiles> {
    let user_type = auth_type_from_str(uri.get(1)?);
    let user = auths::get(uri.get(2)?, user_type)?;
    let res = player_skin::download_skin(&user).await;
    if let Some(tex) = &res.skin {
        push_texture_url(&tex.sha1, &tex.url);
    }
    if let Some(tex) = &res.cape {
        push_texture_url(&tex.sha1, &tex.url);
    }
    Some(SkinFiles {
        is_new_slim: res.is_new_slim,
        skin: res.skin,
        cape: res.cape,
    })
}

/// 判断片段是否为内容 SHA1（40 位十六进制）
///
/// sha1 型与账户型 URI 的区分依据：账户类型关键字与账户 uuid 都不是 40 位十六进制
fn is_sha1(s: &str) -> bool {
    s.len() == 40 && s.chars().all(|c| c.is_ascii_hexdigit())
}

/// 按内容 sha1 取纹理 PNG 字节
///
/// 本地皮肤目录有文件直接读；没有则按 [`TEXTURE_URLS`] 登记的源 URL 重新下载
/// （下载结果仍按内容 sha1 落盘，与请求的 sha1 一致才返回）
async fn load_texture_bytes(sha1: &str) -> Option<Vec<u8>> {
    let file = assets_path::get_skin_object(sha1);
    if file.is_file() {
        return path_helper::read_byte(&file).ok();
    }

    let url = TEXTURE_URLS.read().unwrap().get(sha1).cloned()?;
    let data = mojang_api::get_assets(&url).await.ok()?;
    let saved = assets_path::save_skin_object(&data).ok()?;
    (saved == sha1).then_some(data)
}

/// 皮肤全身渲染图（`skin/...`）
///
/// 片段形态（首段 `skin` 已剥）：
/// - `skin/<sha1>`、`skin/<sha1>/<skin_type>`：sha1 型（40 位十六进制），
///   直接按内容取皮肤——皮肤窗口等已知具体贴图的场景用
/// - `skin/<账户类型>/<uuid>[/<skin_type>]`：账户型，解析该账户**当前选中**的皮肤
///   后转 sha1 型渲染——账户列表预览用
/// - `skin/preview/<uuid>/<skin_type>`：账户型的特例（设置页样例），先判 preview
///
/// 渲染风格由后端读 gui_config 的 `skin_display` 决定（Skin2DA / Skin2DB / Skin3D / Skin3DD），
/// 前端不选风格。皮肤型号段：`auto`（自动检测，可省略）/ `old`（1.7旧版）/
/// `new`（1.8新版）/ `slim`（纤细）——这是皮肤版型信息，不是渲染风格
async fn load_skin_image(uri: &[&str], res: UriSchemeResponder) {
    // sha1 型：2-3 段且第 2 段是 40 位十六进制
    if (uri.len() == 2 || uri.len() == 3) && is_sha1(uri[1]) {
        let skin_type = uri.get(2).copied().unwrap_or("auto");
        if parse_skin_type(skin_type).is_none() {
            send_bad(res);
            return;
        }
        send_skin_display(uri[1], skin_type, None, res).await;
        return;
    }

    if uri.len() != 3 && uri.len() != 4 {
        send_bad(res);
        return;
    }
    let skin_type = uri.get(3).copied().unwrap_or("auto");
    if parse_skin_type(skin_type).is_none() {
        send_bad(res);
        return;
    }

    // 设置页样例预览（`skin/preview/<uuid>/<skin_type>`）：内置样例皮肤按当前配置渲染
    if uri[1] == "preview" {
        send_skin_preview(skin_type, res);
        return;
    }

    let Some(files) = resolve_skin_files(uri).await else {
        send_bad(res);
        return;
    };
    let Some(tex) = files.skin else {
        send_bad(res);
        return;
    };
    send_skin_display(&tex.sha1, skin_type, Some(files.is_new_slim), res).await;
}

/// 皮肤全身渲染（sha1 入口；账户型请求解析出当前皮肤后也转调这里）
///
/// `server_slim` 为账户型解析得到的会话服务器纤细标记（权威值，用于自动型号纠偏）；
/// sha1 型请求传 `None`，自动型号仅按贴图检测
async fn send_skin_display(
    sha1: &str,
    skin_type: &str,
    server_slim: Option<bool>,
    res: UriSchemeResponder,
) {
    let display = gui_config::get().skin_display;
    let key = (sha1.to_string(), format!("{display:?}/{skin_type}"));

    {
        let image = SKIN_IMAGE.read().unwrap().get(&key).cloned();
        if let Some(data) = image {
            send_png(res, data);
            return;
        }
    }

    let Some(bytes) = load_texture_bytes(sha1).await else {
        send_bad(res);
        return;
    };
    let Ok(bitmap) = Pixmap::decode_png(&bytes) else {
        send_bad(res);
        return;
    };
    let st = match parse_skin_type(skin_type).flatten() {
        Some(st) => Some(st),
        None => {
            // 自动检测：带服务器纤细标记时用于纠偏——服务器说是纤细就直接按纤细；
            // 说不是而贴图检测出纤细（手臂宽度含糊时可能误判）按普通新版处理，
            // 其余沿用检测结果（Old / New）
            let detected = skin_type_checker::get_skin_type(&bitmap);
            Some(match server_slim {
                Some(true) => SkinType::NewSlim,
                Some(false) if detected == SkinType::NewSlim => SkinType::New,
                _ => detected,
            })
        }
    };
    let Some(image) = render_skin_display(&bitmap, st, display) else {
        send_bad(res);
        return;
    };
    let Ok(data) = image.encode_png() else {
        send_bad(res);
        return;
    };

    SKIN_IMAGE.write().unwrap().insert(key, data.clone());

    send_png(res, data);
}

/// 按皮肤显示模式选渲染方式（2D 展开 / 2D 大图 / 3D 等距）
fn render_skin_display(
    bitmap: &Pixmap,
    st: Option<SkinType>,
    display: SkinDisplay,
) -> Option<Pixmap> {
    match display {
        SkinDisplay::Skin2DA => skin_2d_draw::skin_2d_draw_typea(bitmap, st),
        SkinDisplay::Skin2DB => skin_2d_draw::skin_2d_draw_typeb(bitmap, st),
        SkinDisplay::Skin3D => skin_3d_draw::draw_skin_3d_typea(bitmap, st),
        SkinDisplay::Skin3DD => skin_3d_draw::draw_skin_3d_typea_down(bitmap, st),
    }
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

/// 原始皮肤贴图PNG（`skinraw/<sha1>`，供前端 skinview3d 做3D渲染）
async fn load_skinraw_image(uri: &[&str], res: UriSchemeResponder) {
    if uri.len() != 2 {
        send_bad(res);
        return;
    }

    {
        let image = SKINRAW_IMAGE.read().unwrap().get(uri[1]).cloned();
        if let Some(data) = image {
            send_png(res, data);
            return;
        }
    }

    let Some(data) = load_texture_bytes(uri[1]).await else {
        send_bad(res);
        return;
    };

    SKINRAW_IMAGE
        .write()
        .unwrap()
        .insert(uri[1].to_string(), data.clone());

    send_png(res, data);
}

/// 原始披风贴图PNG（`caperaw/<sha1>`，供前端 skinview3d 挂载）
async fn load_caperaw_image(uri: &[&str], res: UriSchemeResponder) {
    if uri.len() != 2 {
        send_bad(res);
        return;
    }

    {
        let image = CAPERAW_IMAGE.read().unwrap().get(uri[1]).cloned();
        if let Some(data) = image {
            send_png(res, data);
            return;
        }
    }

    let Some(data) = load_texture_bytes(uri[1]).await else {
        send_bad(res);
        return;
    };

    CAPERAW_IMAGE
        .write()
        .unwrap()
        .insert(uri[1].to_string(), data.clone());

    send_png(res, data);
}

/// 披风渲染图（cape_2d_draw 正面）
///
/// - `cape/<sha1>`：sha1 型，按内容直接取
/// - `cape/<账户类型>/<uuid>`：账户型，解析该账户**当前选中**的披风后转 sha1 型
///   渲染——账户列表预览用
async fn load_cape_image(uri: &[&str], res: UriSchemeResponder) {
    if uri.len() == 2 {
        if is_sha1(uri[1]) {
            send_cape_image(uri[1], res).await;
        } else {
            send_bad(res);
        }
        return;
    }
    if uri.len() != 3 {
        send_bad(res);
        return;
    }

    let Some(files) = resolve_skin_files(uri).await else {
        send_bad(res);
        return;
    };
    let Some(tex) = files.cape else {
        send_bad(res);
        return;
    };
    send_cape_image(&tex.sha1, res).await;
}

/// 披风正面渲染（sha1 入口；账户型请求解析出当前披风后也转调这里）
async fn send_cape_image(sha1: &str, res: UriSchemeResponder) {
    {
        let image = CAPE_IMAGE.read().unwrap().get(sha1).cloned();
        if let Some(data) = image {
            send_png(res, data);
            return;
        }
    }

    let Some(bytes) = load_texture_bytes(sha1).await else {
        send_bad(res);
        return;
    };
    let Ok(bitmap) = Pixmap::decode_png(&bytes) else {
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

    CAPE_IMAGE
        .write()
        .unwrap()
        .insert(sha1.to_string(), data.clone());

    send_png(res, data);
}

/// 披风背面渲染图（cape_2d_draw 背面，供账户列表悬浮大图与正面并排显示）
///
/// - `capeback/<sha1>`：sha1 型，按内容直接取
/// - `capeback/<账户类型>/<uuid>`：账户型，解析该账户**当前选中**的披风后转 sha1 型渲染
async fn load_cape_back_image(uri: &[&str], res: UriSchemeResponder) {
    if uri.len() == 2 {
        if is_sha1(uri[1]) {
            send_cape_back_image(uri[1], res).await;
        } else {
            send_bad(res);
        }
        return;
    }
    if uri.len() != 3 {
        send_bad(res);
        return;
    }

    let Some(files) = resolve_skin_files(uri).await else {
        send_bad(res);
        return;
    };
    let Some(tex) = files.cape else {
        send_bad(res);
        return;
    };
    send_cape_back_image(&tex.sha1, res).await;
}

/// 披风背面渲染（sha1 入口；账户型请求解析出当前披风后也转调这里）
async fn send_cape_back_image(sha1: &str, res: UriSchemeResponder) {
    {
        let image = CAPE_BACK_IMAGE.read().unwrap().get(sha1).cloned();
        if let Some(data) = image {
            send_png(res, data);
            return;
        }
    }

    let Some(bytes) = load_texture_bytes(sha1).await else {
        send_bad(res);
        return;
    };
    let Ok(bitmap) = Pixmap::decode_png(&bytes) else {
        send_bad(res);
        return;
    };
    let Some(image) = cape_2d_draw::draw_cape_back_2d(&bitmap) else {
        send_bad(res);
        return;
    };
    let Ok(data) = image.encode_png() else {
        send_bad(res);
        return;
    };

    CAPE_BACK_IMAGE
        .write()
        .unwrap()
        .insert(sha1.to_string(), data.clone());

    send_png(res, data);
}

/// 按配置的头像类型渲染头像
fn gen_head_image(bitmap: &Pixmap) -> Option<Pixmap> {
    let config = gui_config::get().head;

    match config.head_type {
        HeadType::Head3DA => head_3d_draw::draw_head_3d_typea_down(bitmap),
        HeadType::Head3DB => head_3d_draw::draw_head_3d_typeb(bitmap, config.x, config.y),
        HeadType::Head3DC => head_3d_draw::draw_head_3d_typea_up(bitmap),
        HeadType::Head2DB => head_2d_draw::head_2d_draw_typeb(bitmap),
        HeadType::Head2DA => head_2d_draw::head_2d_draw_typea(bitmap),
    }
}

/// 远程图标（`icon/<请求名>`）：内存 → 磁盘 + ETag 校验 → 网络
///
/// 内存未命中时读磁盘缓存，并带记录的 ETag 发一次条件请求回源校验：
/// 服务器 304（图没变）不拉图片体，直接用硬盘缓存；200 才重新拉图更新本地；
/// 网络失败时用本地副本兜底
async fn load_icon_image(uri: &[&str], res: UriSchemeResponder) {
    if uri.len() != 2 {
        send_bad(res);
        return;
    }

    let name = uri[1];

    let url = URL_IMAGE.read().unwrap().get(name).cloned();
    let Some(url) = url else {
        // 映射里没有 ≠ 图片没有：`sha → 远程网址` 只在内存里，重启后就空了，
        // 但图片字节是按 sha256 落在磁盘缓存上的 —— 先按 name 查盘（收藏夹里存下来的
        // 老地址就是这种情况），查不到才算真的没有
        let file = icon_file_by_name(name);
        match read_icon_bytes(&file) {
            Some(icon) => send_icon(res, &icon),
            None => send_bad(res),
        }
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

    // 磁盘缓存的本地副本（供 304 续用 / 网络失败兜底）
    let cached = read_icon_bytes(&file);
    if cached.is_none() && file.is_file() {
        // 内容损坏：删掉重新下载
        let _ = path_helper::delete(&file);
    }

    // 有磁盘缓存就带 ETag（两家 CDN 的 ETag 都是图片内容的 MD5）去校验，
    // 没有就直接下载——每次最多一个请求，图没变时 304 只回头部不带体
    let etag = read_icon_etag(&etag_file);
    match get_work_client().get_bytes_with_check(&url, etag.as_deref()).await {
        Ok(check) if check.not_modified => {
            if let Some(icon) = &cached {
                set_icon_image(name, icon.data.clone(), icon.mime);
                send_icon(res, icon);
            } else {
                // 只有 ETag 没有图片（异常状态），下次不带 ETag 重新下载
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
            // 网络失败：有本地副本就先用着
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
pub fn init() {
    ICON_DIR.get_or_init(|| mml_downloader::get_cache_path().join(names::IMAGE_DIR));
    // 登记表也读回来：没有它，收藏夹里存下来的 `.../icon/<sha>` 地址就找不回远程网址，
    // 既回不了源、也做不了过期校验，只能一直吃磁盘缓存
    load_url_image();
}

/// 登记表落盘文件（`<图标缓存目录>/urls.json`）
const ICON_URLS_FILE: &str = "urls.json";

fn url_image_file() -> Option<PathBuf> {
    ICON_DIR.get().map(|dir| dir.join(ICON_URLS_FILE))
}

/// 把「sha256 → 远程网址」登记表写到磁盘
fn save_url_image() {
    let Some(file) = url_image_file() else {
        return;
    };
    let Ok(json) = serde_json::to_vec(&*URL_IMAGE.read().unwrap()) else {
        return;
    };
    let _ = path_helper::write_bytes(&file, &json);
}

/// 启动时读回登记表（首次运行没有这个文件，忽略即可）
fn load_url_image() {
    let Some(file) = url_image_file() else {
        return;
    };
    let Ok(data) = path_helper::read_byte(&file) else {
        return;
    };
    let Ok(map) = serde_json::from_slice::<HashMap<String, String>>(&data) else {
        return;
    };

    URL_IMAGE.write().unwrap().extend(map);
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

/// 物品贴图（`mml-image/item/<物品ID>`）：与方块同策略，
/// PNG 直接读盘透传，不进内存缓存——重渲染后立即生效；
/// webview 自身的缓存由 URL 上的 `?v=<版本>` 破掉
fn load_item_image(uri: &[&str], res: UriSchemeResponder) {
    if uri.len() != 2 {
        send_bad(res);
        return;
    }
    let Some(file) = mml_tex_draw::get_item_path(uri[1]) else {
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

/// 清空全部纹理类内存缓存（账户列表"刷新皮肤"用）
///
/// 渲染缓存键是内容 sha1 而非账户维度，量小全清最稳（重渲染开销可忽略）；
/// 同时清掉 sha1→URL 登记表，下次取图重新走会话服务器下载并登记。
/// 本地皮肤文件按内容缓存，服务器换肤后贴图内容变化自动落新文件，无需删文件；
/// webview 侧的旧图由前端 bump 图片版本号破掉。
pub fn clear_texture_images() {
    HEAD_IMAGE.write().unwrap().clear();
    SKIN_IMAGE.write().unwrap().clear();
    SKINRAW_IMAGE.write().unwrap().clear();
    CAPERAW_IMAGE.write().unwrap().clear();
    CAPE_IMAGE.write().unwrap().clear();
    CAPE_BACK_IMAGE.write().unwrap().clear();
    TEXTURE_URLS.write().unwrap().clear();
}

/// 登记纹理的源 URL（内容 sha1 → URL），本地文件被清后 sha1 型请求能回源
pub fn push_texture_url(sha1: &str, url: &str) {
    if sha1.is_empty() || url.is_empty() {
        return;
    }
    TEXTURE_URLS
        .write()
        .unwrap()
        .insert(sha1.to_string(), url.to_string());
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
    icon_file_by_name(&icon_name(url))
}

/// 已知 sha256 名字时的磁盘缓存位置
///
/// 内存里的「sha → 远程网址」映射是重启就没的，而缓存文件就叫这个名字 ——
/// 所以拿到 name 就能直接查盘，不必先知道远程网址。
fn icon_file_by_name(name: &str) -> PathBuf {
    ICON_DIR.get().unwrap().join(format!("{name}.png"))
}

/// 图标的 ETag 旁车文件（`<网址 sha256>.etag`）
fn icon_etag_file(url: &str) -> PathBuf {
    ICON_DIR
        .get()
        .unwrap()
        .join(format!("{}.etag", icon_name(url)))
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

/// 读取磁盘缓存的图标内容（不管新鲜度，有效性由 ETag 校验）
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

/// `mml-image` 协议入口：按 URI 首段路由到各加载函数
///
/// # 参数
///
/// - `req`: 协议请求
/// - `res`: 响应回写句柄
pub async fn url_image(req: Request<Vec<u8>>, res: UriSchemeResponder) {
    let uri = split_path(req.uri().path());
    let image_type = uri.first().copied().unwrap_or_default();

    if image_type == "instance" {
        load_instance_image(&uri, res);
    } else if image_type == "head" {
        load_head_image(&uri, res).await;
    } else if image_type == "skin" {
        load_skin_image(&uri, res).await;
    } else if image_type == "skinraw" {
        load_skinraw_image(&uri, res).await;
    } else if image_type == "caperaw" {
        load_caperaw_image(&uri, res).await;
    } else if image_type == "cape" {
        load_cape_image(&uri, res).await;
    } else if image_type == "capeback" {
        load_cape_back_image(&uri, res).await;
    } else if image_type == "icon" {
        load_icon_image(&uri, res).await;
    } else if image_type == "block" {
        load_block_image(&uri, res);
    } else if image_type == "item" {
        load_item_image(&uri, res);
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

    // 登记表要落盘：只放内存里的话，重启后收藏夹存下来的 `.../icon/<sha>` 地址
    // 就查不回远程网址了（既回不了源、也走不了 ETag 过期校验）。
    // 同一个网址重复登记很常见（各窗口渲染），所以只在真的变了才写
    let changed = {
        let mut lock = URL_IMAGE.write().unwrap();
        lock.insert(name.clone(), url.to_string()).as_deref() != Some(url)
    };
    if changed {
        save_url_image();
    }

    format!("{}/icon/{}", image_base_url(), name)
}

/// 把"本地图片协议地址"还原成登记时的远程网址
///
/// [`push_image_url`] 返回的 `.../icon/<sha256>` 只是个**进程内**的映射键。
/// 凡是要落盘的地方都得先用这个换回原始网址 —— 存下来的话重启后就是个死地址
/// （既回不了源，也走不了 ETag 过期校验）。
pub fn remote_url(local: &str) -> Option<String> {
    // 去掉可能的 `?v=…` 查询串，再取最后一段（就是 sha256）
    let name = local
        .split('?')
        .next()?
        .trim_end_matches('/')
        .rsplit('/')
        .next()?;

    URL_IMAGE.read().unwrap().get(name).cloned()
}
