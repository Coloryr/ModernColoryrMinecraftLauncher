//! 方块列表窗口：方块贴图渲染状态 + IPC 命令（渲染 / 设图标 / 皮肤方块）
//!
//! 从主窗口拆出为独立窗口；渲染进度通过 `block-render` 事件广播，
//! 前端三态视图（未渲染 / 渲染中 / 已渲染）由状态驱动。

use std::sync::{
    atomic::{AtomicBool, AtomicU32, Ordering},
    Arc, LazyLock, Mutex,
};

use tauri::{AppHandle, Emitter};

use crate::dtos::{BlockItemDto, BlockStatusDto, IconSourceDto};
use crate::{image_manager, listens};
use mml_base::hash_helper;
use mml_game::gui_hook::IProgressGui;
use mml_names::{Lang, i18_items::error_type::ErrorType, names};

use super::main::{core_instance, emit_instance_change};

/// 方块贴图渲染状态（渲染进度 / 结果，驱动前端三态视图）
struct BlockRenderState {
    /// 渲染中（防重入）
    running: AtomicBool,
    /// 进度：已处理数
    now: AtomicU32,
    /// 进度：总数
    total: AtomicU32,
    /// 进度文字
    text: Mutex<Option<String>>,
    /// 上次渲染失败的错误信息
    error: Mutex<Option<String>>,
}

/// 方块贴图渲染进度状态（内存态，`render_status` 查询 / 事件推送共用）
static BLOCK_RENDER: LazyLock<BlockRenderState> = LazyLock::new(|| BlockRenderState {
    running: AtomicBool::new(false),
    now: AtomicU32::new(0),
    total: AtomicU32::new(0),
    text: Mutex::new(None),
    error: Mutex::new(None),
});

/// 方块渲染状态快照（内存进度 + 渲染结果合并；命令 `block_status` 同名，故叫 render_status）
fn render_status() -> BlockStatusDto {
    BlockStatusDto {
        rendered: !mml_tex_draw::blocks().is_empty() || !mml_tex_draw::items().is_empty(),
        version: mml_tex_draw::block_version(),
        running: BLOCK_RENDER.running.load(Ordering::Acquire) || mml_tex_draw::is_loading(),
        now: BLOCK_RENDER.now.load(Ordering::Acquire),
        total: BLOCK_RENDER.total.load(Ordering::Acquire),
        text: BLOCK_RENDER.text.lock().unwrap().clone(),
        error: BLOCK_RENDER.error.lock().unwrap().clone(),
    }
}

/// 方块渲染进度回调：写状态并广播事件（回调全同步，直接 emit）
struct BlockRenderGui {
    app: AppHandle,
}

impl IProgressGui for BlockRenderGui {
    /// 更新进度文字并广播
    fn set_progress_text(&self, text: Option<String>) {
        *BLOCK_RENDER.text.lock().unwrap() = text;
        emit_block_render(&self.app, render_status());
    }

    /// 更新步数进度并广播
    fn set_progress_now(&self, value: usize, all: Option<usize>) {
        BLOCK_RENDER.now.store(value as u32, Ordering::Release);
        if let Some(all) = all {
            BLOCK_RENDER.total.store(all as u32, Ordering::Release);
        }
        emit_block_render(&self.app, render_status());
    }
}

/// 方块渲染状态事件（进度变化 / 结束时推给方块窗口）
#[gui_macros::emit]
pub fn emit_block_render(app: &AppHandle, data: BlockStatusDto) {
    let _ = app.emit(listens::BLOCK_RENDER, data);
}

/// 渲染是否进行中（窗口关闭保护用）
///
/// 已取消时算「不在渲染」：渲染循环还要几毫秒才退出，若仍拦着，
/// 前端「取消 → 关窗」会撞上这段窗口期又弹一次确认
pub fn render_running() -> bool {
    (BLOCK_RENDER.running.load(Ordering::Acquire) || mml_tex_draw::is_loading())
        && !mml_tex_draw::is_cancelled()
}

/// 注册内核侧的渲染进度回调（启动时调用一次）
///
/// 供内核侧自行发起的渲染（调用 `load_blocks` 时没传回调）上报事件与进度；
/// 界面触发的渲染自带回调，这里注册的只是兜底
pub fn set_gui_handel(app: &AppHandle) {
    let gui: Arc<dyn IProgressGui> = Arc::new(BlockRenderGui { app: app.clone() });
    mml_tex_draw::set_gui_handel(Some(gui));
}

/// 获取方块与物品列表（按请求语言翻译；顺序＝游戏创造栏顺序）
///
/// 顺序完全由 crate 给的合并序列决定（`mml_tex_draw::ordered_entries`）：
/// **不要**在这里按分类名 / 方块名排序——显示名是翻译后的文字，按它排会随界面语言变化，
/// 而且与游戏内顺序无关（这正是改之前的问题）。
#[tauri::command]
pub fn block_list(lang: String) -> Vec<BlockItemDto> {
    let lang = if lang == "en_us" {
        Lang::en_us
    } else {
        Lang::zh_cn
    };
    let base = image_manager::image_base_url();
    let version = mml_tex_draw::block_version();
    let item_version = mml_tex_draw::item_version();

    // 分组名从游戏语言文件翻译（itemGroup.<cat>键，与方块名同一来源）；
    // miss（如自定义皮肤分组 playerSkin 不是游戏键）时回退原值，前端兜底
    let cat_name = |cat: String| -> String {
        mml_tex_draw::get_lang(lang, &format!("itemGroup.{cat}")).unwrap_or(cat)
    };

    mml_tex_draw::ordered_entries()
        .into_iter()
        .map(|(kind, id)| {
            // 方块与物品只差：分类来源、名称语言键、图标目录与版本号
            let (cat, name_key, dir, ver) = match kind {
                mml_tex_draw::EntryKind::Block => (
                    mml_tex_draw::block_cat(&id),
                    mml_tex_draw::block_name_key(&id),
                    "block",
                    &version,
                ),
                mml_tex_draw::EntryKind::Item => (
                    mml_tex_draw::item_cat(&id),
                    mml_tex_draw::item_name_key(&id),
                    "item",
                    &item_version,
                ),
            };
            // 显示名：语言键翻译，miss 回退 id 尾段（如 minecraft:stone → stone）
            let name = name_key
                .and_then(|key| mml_tex_draw::get_lang(lang, &key))
                .unwrap_or_else(|| id.rsplit(':').next().unwrap_or(&id).to_string());
            BlockItemDto {
                name,
                cat: cat_name(cat.unwrap_or_default()),
                image: format!("{base}/{dir}/{id}?v={ver}"),
                id,
            }
        })
        .collect()
}

/// 获取方块贴图渲染状态
#[tauri::command]
pub fn block_status() -> BlockStatusDto {
    render_status()
}

/// 开始渲染方块贴图（force = 忽略版本短路全量重渲染）
///
/// 渲染一律由界面触发（启动时不再自动补渲染）。
/// 返回是否成功启动（已在跑返回 false，前端据此提示）
#[tauri::command]
pub async fn block_render_start(app: AppHandle, force: bool) -> Result<bool, String> {
    // 防重入查两处：GUI 自己的 running 标志 + mml-tex-draw 本体的 LOADING
    //（内核侧可能自发起一轮，不走 GUI 标志，不查会并发再起一轮、同一 jar 下两遍）。
    // is_loading 必须先于 swap 判断：它命中时直接返回，若已把 running 置 true
    // 而本轮没有 spawn 任务去复位，标志会卡死，之后永远“已在渲染中”
    if mml_tex_draw::is_loading() || BLOCK_RENDER.running.swap(true, Ordering::AcqRel) {
        return Ok(false);
    }

    // 新一轮渲染：清掉上次的错误与进度
    *BLOCK_RENDER.error.lock().unwrap() = None;
    *BLOCK_RENDER.text.lock().unwrap() = None;
    BLOCK_RENDER.now.store(0, Ordering::Release);
    BLOCK_RENDER.total.store(0, Ordering::Release);
    emit_block_render(&app, render_status());

    let gui: Arc<dyn IProgressGui> = Arc::new(BlockRenderGui { app: app.clone() });
    tauri::async_runtime::spawn(async move {
        let result = mml_tex_draw::load_blocks(Some(gui), force).await;
        // 主动取消不算失败（渲染循环随后就退出，状态复位交给下面统一做）
        if let Err(err) = result && !matches!(err, ErrorType::TaskCancel) {
            *BLOCK_RENDER.error.lock().unwrap() = Some(err.to_string());
        }
        BLOCK_RENDER.running.store(false, Ordering::Release);
        emit_block_render(&app, render_status());
    });

    Ok(true)
}

/// 取消正在进行的渲染
///
/// 渲染循环与在途下载都是协作式的，取消后几毫秒内退出本轮，
/// 残留的半成品 PNG 不会被登记（下次全量渲染覆盖）。
/// 返回是否有渲染在进行（没有可取消的返回 false）
#[tauri::command]
pub fn block_render_cancel(app: AppHandle) -> bool {
    if !render_running() {
        return false;
    }
    mml_tex_draw::cancel();
    emit_block_render(&app, render_status());
    true
}

/// 把方块/物品贴图设为实例图标
#[tauri::command]
pub async fn block_set_icon(app: AppHandle, uuid: String, id: String) -> Result<bool, String> {
    // 方块优先，物品（未作为方块出现的 id）回退物品贴图
    let Some(file) = mml_tex_draw::get_block_path(&id).or_else(|| mml_tex_draw::get_item_path(&id))
    else {
        return Err("err.fileNotFound".to_string());
    };

    // 锁内只取需要的路径，drop 后再 await（std 锁跨 await 破坏 Send）
    let Some((id, instance)) = core_instance(&uuid) else {
        return Err("err.gameNotFound".to_string());
    };
    let dest = instance.read().unwrap().get_icon_file();

    tokio::fs::copy(&file, &dest)
        .await
        .map_err(|err| err.to_string())?;

    image_manager::clear_instance_image(&id);
    emit_instance_change(&app, "edit");
    Ok(true)
}

/// 图标边长的默认值（px）
///
/// 图标框是圆角方块，显示最大 84px；256 已足够清晰，需要更锐利（如给整合包做缩略图）
/// 时由前端在 [`ICON_SIZE_MIN`] ~ [`ICON_SIZE_MAX`] 之间另选。
const ICON_SIZE_DEFAULT: u32 = 256;
/// 图标边长下限（再小就糊了）
const ICON_SIZE_MIN: u32 = 16;
/// 图标边长上限（太大只是浪费磁盘与解码时间）
const ICON_SIZE_MAX: u32 = 1024;

/// 裁剪预览的最长边（px）
///
/// 预览只用于选范围，**不参与最终裁剪**：选区坐标会按「原图 / 预览」比例换算回原图，
/// 裁的仍是原图，清晰度不受影响。降采样是为了不把十几 MB 的原图 PNG 塞进 IPC 与 `<img>`
/// —— 那正是"打开弹窗就卡死"的原因。
const PREVIEW_MAX: u32 = 1024;

/// 解码前的像素数上限（防解压炸弹）
///
/// 文件头里声明一个 5 万见方的尺寸是几 KB 的事，真去解码却要 10GB 内存。
/// 64MP 已高于常见手机照片（12~50MP），只挡病态输入。
const MAX_SOURCE_PIXELS: u64 = 64 * 1024 * 1024;

/// 把前端给的图标边长夹到合法范围（0 或越界都回落到默认值）
fn clamp_icon_size(size: u32) -> u32 {
    if size == 0 {
        ICON_SIZE_DEFAULT
    } else {
        size.clamp(ICON_SIZE_MIN, ICON_SIZE_MAX)
    }
}

/// 读图 → 生成预览 data URL + 原图尺寸（**阻塞**，调用方负责放到阻塞线程上跑）
///
/// 只把降采样后的预览编成 PNG：原图直接编 PNG 又慢又大（12MP 照片要几秒、出来几十 MB），
/// 这一步以前跑在主线程上，界面会整片冻住。
fn read_icon_preview(path: &str) -> Result<IconSourceDto, String> {
    let bytes = std::fs::read(path).map_err(|_| "err.fileNotFound".to_string())?;
    let format = image::guess_format(&bytes).map_err(|_| "err.imageInvalid".to_string())?;

    // 先只读文件头拿尺寸：解压炸弹不必真解码就能挡掉（`into_dimensions` 不读像素）
    let (width, height) = image::ImageReader::with_format(std::io::Cursor::new(&bytes), format)
        .into_dimensions()
        .map_err(|_| "err.imageInvalid".to_string())?;
    if u64::from(width) * u64::from(height) > MAX_SOURCE_PIXELS {
        return Err("err.imageInvalid".to_string());
    }

    let image = image::load_from_memory_with_format(&bytes, format)
        .map_err(|_| "err.imageInvalid".to_string())?;
    // thumbnail 先整数抽样再插值，比 resize 快得多；等比缩放到 PREVIEW_MAX 以内
    let preview = if width > PREVIEW_MAX || height > PREVIEW_MAX {
        image.thumbnail(PREVIEW_MAX, PREVIEW_MAX)
    } else {
        image
    };

    let mut out = std::io::Cursor::new(Vec::new());
    preview
        .write_to(&mut out, image::ImageFormat::Png)
        .map_err(|err| err.to_string())?;
    Ok(IconSourceDto {
        src: format!(
            "data:image/png;base64,{}",
            hash_helper::gen_base64_bytes(out.get_ref())
        ),
        width,
        height,
    })
}

/// 读本地图片的**预览图**与原图尺寸（供"修改图标"的裁剪界面用）
///
/// 与资源图标同一套编码（`data:image/png;base64,`），前端可直接放进 `<img src>`。
/// 只认 `image` crate 已启用特性的格式（png / jpeg / webp），解不出来的按无效图片处理。
///
/// 必须是 `async` 命令 + 阻塞线程池：同步命令会直接在**主线程**上解码 / 编码，
/// 大图会让界面完全没响应（这个坑踩过一次）。
#[tauri::command]
pub async fn block_read_icon_source(path: String) -> Result<IconSourceDto, String> {
    tauri::async_runtime::spawn_blocking(move || read_icon_preview(&path))
        .await
        .map_err(|err| err.to_string())?
}

/// 按选区裁出图标 PNG（**阻塞**，调用方负责放到阻塞线程上跑）
///
/// - `w == 0 || h == 0`：不裁，整图等比缩放到 `size` 见方，四周补透明（整图都保留）；
/// - 否则按 `(x, y, w, h)`（**原图像素坐标**）裁剪后再缩放到 `size` 见方。
fn render_icon_png(
    path: &str,
    x: u32,
    y: u32,
    w: u32,
    h: u32,
    size: u32,
) -> Result<Vec<u8>, String> {
    let bytes = std::fs::read(path).map_err(|_| "err.fileNotFound".to_string())?;
    let image = image::load_from_memory(&bytes).map_err(|_| "err.imageInvalid".to_string())?;
    let (img_w, img_h) = (image.width(), image.height());

    let icon = if w == 0 || h == 0 {
        // 不裁（"使用原图"）：等比缩放进方框，四周补透明，整图都保留
        let scaled = image.resize(size, size, image::imageops::FilterType::Lanczos3);
        let mut canvas = image::RgbaImage::new(size, size);
        let ox = (size - scaled.width()) / 2;
        let oy = (size - scaled.height()) / 2;
        image::imageops::overlay(&mut canvas, &scaled.to_rgba8(), ox as i64, oy as i64);
        image::DynamicImage::ImageRgba8(canvas)
    } else {
        // 选区按原图边界夹紧（前端已夹过，这里兜底，避免越界 panic）
        let cx = x.min(img_w.saturating_sub(1));
        let cy = y.min(img_h.saturating_sub(1));
        let cw = w.min(img_w - cx);
        let ch = h.min(img_h - cy);
        // 选区：等比缩放后居中裁掉多余的边 → 正好是正方形图标
        image
            .crop_imm(cx, cy, cw, ch)
            .resize_to_fill(size, size, image::imageops::FilterType::Lanczos3)
    };

    let mut out = std::io::Cursor::new(Vec::new());
    icon.write_to(&mut out, image::ImageFormat::Png)
        .map_err(|err| err.to_string())?;
    Ok(out.into_inner())
}

/// 按选区把本地图片裁成实例图标：写实例目录下的 `icon.png`，并把实例配置的 `Icon` 指过去
///
/// - `size`：图标边长（px），0 或越界都回落到 [`ICON_SIZE_DEFAULT`]（夹在
///   [`ICON_SIZE_MIN`] ~ [`ICON_SIZE_MAX`]）；
/// - `w == 0 || h == 0`：不裁，整图等比缩放到 `size` 见方；
/// - 否则按 `(x, y, w, h)`（**原图像素坐标**）裁剪后再缩放到 `size` 见方；
/// - 解码 / 裁剪 / 缩放（Lanczos3）都放阻塞线程池，不占 async 运行时；
/// - 写盘前先把终图编码好，不持锁 `await`（std 锁跨 await 破坏 `Send`）；
/// - `icon.png` 就是 [`InstanceSettingObj::get_icon_file`] 的默认名，
///   这里同时把配置里的 `Icon` 显式指过去，实例目录里一眼能看出图标文件是谁。
#[tauri::command]
pub async fn block_set_icon_area(
    app: AppHandle,
    uuid: String,
    path: String,
    x: u32,
    y: u32,
    w: u32,
    h: u32,
    size: u32,
) -> Result<bool, String> {
    let size = clamp_icon_size(size);
    let png =
        tauri::async_runtime::spawn_blocking(move || render_icon_png(&path, x, y, w, h, size))
            .await
            .map_err(|err| err.to_string())??;

    let Some((id, instance)) = core_instance(&uuid) else {
        return Err("err.gameNotFound".to_string());
    };
    let dest = instance.read().unwrap().get_icon_file();
    tokio::fs::write(&dest, &png)
        .await
        .map_err(|err| err.to_string())?;

    // 实例配置里把 Icon 指到 icon.png（与 get_icon_file 的默认值一致，显式写下来更清楚）
    {
        let mut inst = instance.write().unwrap();
        if inst.icon.as_deref() != Some(names::ICON_FILE) {
            inst.icon = Some(names::ICON_FILE.to_string());
            inst.save();
        }
    }

    image_manager::clear_instance_image(&id);
    emit_instance_change(&app, "edit");
    Ok(true)
}

/// 按用户名或UUID添加皮肤方块（拉取该玩家的皮肤，渲染成头颅图标），返回方块ID
///
/// 同名玩家重复添加即覆盖（刷新图标）
#[tauri::command]
pub async fn block_skin_add(input: String) -> Result<String, String> {
    let (name, skin) = mml_game::player_skin::fetch_skin_by_input(&input)
        .await
        .map_err(|e| e.to_string())?;

    // 锁内只取路径，文件读取在drop后（std锁跨await破坏Send同理，这里没锁）
    let data = tokio::fs::read(&skin).await.map_err(|e| e.to_string())?;
    mml_tex_draw::add_skin_block(&name, &data).map_err(|e| e.to_string())
}

/// 删除皮肤方块（名字即皮肤方块显示名）
#[tauri::command]
pub fn block_skin_remove(name: String) -> Result<(), String> {
    mml_tex_draw::remove_skin_block(&name).map_err(|e| e.to_string())
}
