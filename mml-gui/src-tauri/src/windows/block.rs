//! 方块列表窗口：方块贴图渲染状态 + IPC 命令（渲染 / 设图标 / 皮肤方块）
//!
//! 从主窗口拆出为独立窗口；渲染进度通过 `block-render` 事件广播，
//! 前端三态视图（未渲染 / 渲染中 / 已渲染）由状态驱动。

use std::sync::{
    atomic::{AtomicBool, AtomicU32, Ordering},
    Arc, LazyLock, Mutex,
};

use tauri::{AppHandle, Emitter};

use crate::dtos::{BlockItemDto, BlockStatusDto};
use crate::{image_manager, listens};
use mml_game::gui_hook::IProgressGui;
use mml_names::{Lang, i18_items::error_type::ErrorType};

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

/// 获取方块与物品列表（按请求语言翻译，cat + name 排序）
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

    let block_ids: std::collections::HashSet<String> =
        mml_tex_draw::blocks().into_iter().collect();

    let mut list: Vec<BlockItemDto> = block_ids
        .iter()
        .map(|id| {
            // 显示名：语言键翻译，miss 回退 id 尾段（如 minecraft:stone → stone）
            let name = mml_tex_draw::block_name_key(id)
                .and_then(|key| mml_tex_draw::get_lang(lang, &key))
                .unwrap_or_else(|| id.rsplit(':').next().unwrap_or(id).to_string());
            BlockItemDto {
                name,
                cat: cat_name(mml_tex_draw::block_cat(id).unwrap_or_default()),
                image: format!("{base}/block/{id}?v={version}"),
                id: id.clone(),
            }
        })
        // 物品：跳过已作为方块出现的 id（如 minecraft:stone 方块/物品同 id），避免重复条目
        .chain(mml_tex_draw::items().into_iter().filter(|id| !block_ids.contains(id))
            .map(|id| {
                let name = mml_tex_draw::item_name_key(&id)
                    .and_then(|key| mml_tex_draw::get_lang(lang, &key))
                    .unwrap_or_else(|| id.rsplit(':').next().unwrap_or(&id).to_string());
                BlockItemDto {
                    name,
                    cat: cat_name(mml_tex_draw::item_cat(&id).unwrap_or_default()),
                    image: format!("{base}/item/{id}?v={item_version}"),
                    id,
                }
            }))
        .collect();
    list.sort_by(|a, b| a.cat.cmp(&b.cat).then_with(|| a.name.cmp(&b.name)));
    list
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
