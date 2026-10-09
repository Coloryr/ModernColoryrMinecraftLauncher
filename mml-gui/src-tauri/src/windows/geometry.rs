//! 窗口几何：口径换算 + `window_save.json` 读写
//!
//! 从 `windows/mod.rs` 拆出来的。这里是全仓**最贵的一段经验**（原注释一字未删）：
//! 外框 / 客户区两种口径、逻辑 / 物理两种单位、以及"摘掉原生装饰会把标题栏并进
//! 客户区"导致的高度漂移 —— 都集中在这个文件里，改之前先读注释。
//!
//! 口径速记（详见 `WindowState` 的文档）：
//! - 存盘 / 恢复一律按**客户区**（`inner_size()`），且折算回**带装饰**时的口径；
//! - `WINDOWS_INFO` 里的最小尺寸是**客户区 + 逻辑像素**（见 `super::registry`）；
//! - 只有下发给 Windows 的外框最小尺寸才需要加上那圈边框带（见 [`reconcile_min_size`]）。

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{LazyLock, OnceLock, RwLock},
};

use mml_base::serialize_tools;
use mml_config::config_save;
use mml_names::{names, uuids};
use serde::{Deserialize, Serialize};
use tauri::WebviewWindow;
use uuid::Uuid;

use super::registry::min_size_of;

/// 窗口几何状态（window_save.json）
///
/// 全仓统一这一套口径：
/// - `x / y`：**外框**位置，物理像素（与 `outer_position()` 同源）
/// - `width / height`：**客户区**尺寸，物理像素（与 `inner_size()` 同源），
///   且不小于注册表里的最小客户区
///
/// 注册表 `WINDOWS_INFO` 里的最小 / 默认尺寸则是**客户区 + 逻辑像素**，两者之间只差显示器的
/// 缩放系数一次换算（见 [`min_inner_physical`]）；只有下发给 Windows 的外框最小尺寸
/// 才需要再加上那圈边框带（见 [`reconcile_min_size`]）。
///
/// ⚠ `inner_size()` 的含义**随装饰状态变化**：有原生 frame 时是客户区，
/// 插件 `set_decorations(false)` 之后同一块窗口会把标题栏并进客户区（外框不变）。
/// 而保存动作在两种状态下都会发生，直接存它会让窗口高度每次开关都涨一截。
/// 所以存盘前统一折算回**带装饰口径**，见 [`decoration_growth`]。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub(super) struct WindowState {
    /// 外框 X（`outer_position()`）
    pub x: i32,
    /// 外框 Y（`outer_position()`）
    pub y: i32,
    /// 客户区宽（`inner_size()`）
    pub width: u32,
    /// 客户区高（`inner_size()`）
    pub height: u32,
    /// 上次退出时是否处于最大化（全屏也算）
    ///
    /// 为 true 时 `x / y / width / height` 保留的是**最大化之前**的几何：开窗先按它开、
    /// 再最大化，这样既回到原来那块屏，也能恢复最大化状态。
    pub maximized: bool,
}

/// 窗口几何状态（uuid → 几何），内存中的唯一数据源
pub(super) static WINDOWS_STATE: LazyLock<RwLock<HashMap<Uuid, WindowState>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

/// 窗口状态文件路径（window_save.json）
pub(super) static STATE_FILE: OnceLock<PathBuf> = OnceLock::new();

/// 「带装饰」状态下「外框 − 客户区」的差值缓存（uuid → (宽, 高)，物理像素）
///
/// 用途见 [`decoration_growth`]：存盘要把无装饰时被撑大的客户区折算回带装饰口径，
/// 需要知道标题栏有多高；而关窗保存时窗口已无装饰、量不到它，只能在**建窗那一刻**
/// （仍是 `decorations(true)`）先量下来。
pub(super) static DECORATED_INSET: LazyLock<RwLock<HashMap<Uuid, (u32, u32)>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

/// 量并缓存「带装饰」状态下的外框 − 客户区差值（建窗后立即调用）
pub(super) fn remember_decorated_inset(uuid: &Uuid, window: &WebviewWindow) {
    let (Ok(inner), Ok(outer)) = (window.inner_size(), window.outer_size()) else {
        return;
    };
    let inset = (
        outer.width.saturating_sub(inner.width),
        outer.height.saturating_sub(inner.height),
    );
    DECORATED_INSET.write().unwrap().insert(*uuid, inset);
}

/// 读取窗口状态文件（启动时调用，位于运行路径下）
pub fn init<P: AsRef<Path>>(path: P) {
    let file = STATE_FILE.get_or_init(|| path.as_ref().join(names::WINDOW_SAVE_FILE));

    if file.exists()
        && file.is_file()
        && let Ok(data) = serialize_tools::json_from_file::<HashMap<Uuid, WindowState>>(file)
    {
        WINDOWS_STATE.write().unwrap().extend(data);
    }
}

/// 保存窗口状态
pub(super) fn save() {
    let Some(file) = STATE_FILE.get() else {
        return;
    };
    let data = WINDOWS_STATE.read().unwrap();
    config_save::save(uuids::WINDOW_FILE_UUID, &*data, file);
}

/// 读取指定 uuid 的窗口几何（用于启动时恢复位置大小）
pub(super) fn window_state_for(uuid: &Uuid) -> Option<WindowState> {
    WINDOWS_STATE.read().unwrap().get(uuid).cloned()
}

/// 设置窗口状态
pub(super) fn window_state_set(uuid: &Uuid, state: WindowState) {
    WINDOWS_STATE.write().unwrap().insert(*uuid, state);
    save();
}

/// 注册表里的最小客户区尺寸（逻辑像素）→ 物理像素
///
/// 存盘与恢复都按客户区口径，夹取时用它。**不要在这里加边框带**：那圈只影响下发给 Windows 的
/// **外框**最小尺寸（见 [`reconcile_min_size`]），客户区本身不含它。
pub(super) fn min_inner_physical(min: (f64, f64), scale: f64) -> (u32, u32) {
    let scale = if scale > 0.0 { scale } else { 1.0 };
    (
        (min.0 * scale).round().max(1.0) as u32,
        (min.1 * scale).round().max(1.0) as u32,
    )
}

/// 把注册表里的最小尺寸（客户区口径）换算成外框口径，再重设一次
///
/// Windows 的最小尺寸走 `WM_GETMINMAXINFO`，它约束的是**外框**；而我们关心的其实是客户区
/// （webview 能拿到多少排版空间）。无边框 + 系统阴影时「外框 − 客户区」就是那圈 Windows
/// 边框带（96 DPI 下左右下各 8px、顶部 1px，且只有 Windows 有）。
/// 这里量出真实差值（物理像素，随 DPI 自动变化、不用自己算 SM_CXSIZEFRAME ——
/// 那双指标也不是按 DPI 倍数缩的：96→8、120→9、144→11）加到最小尺寸上。
/// 没有这个带时差值为 0，等于什么都没做（关掉阴影或换到别的平台都安全）。
pub(super) fn reconcile_min_size(win: &WebviewWindow, min_w: f64, min_h: f64) {
    let (Ok(inner), Ok(outer)) = (win.inner_size(), win.outer_size()) else {
        return;
    };
    let dx = outer.width.saturating_sub(inner.width);
    let dy = outer.height.saturating_sub(inner.height);
    if dx == 0 && dy == 0 {
        return;
    }
    // 量到的差值是物理像素，换算回逻辑像素后与 min_w / min_h 一起设回去。
    //
    // 必须用**逻辑单位**：tauri/tao 把它存成 `PixelUnit::Logical`，换到别的 DPI 显示器时
    // 会跟着缩放，与建窗时 `min_inner_size(min_w, min_h)` 的口径一致；
    // 用物理像素（`PhysicalSize`）则不会缩放，高 DPI 屏上"最小 770"就不再成立。
    let scale = win.scale_factor().unwrap_or(1.0);
    if scale <= 0.0 {
        return;
    }
    let width = min_w + dx as f64 / scale;
    let height = min_h + dy as f64 / scale;
    let _ = win.set_min_size(Some(tauri::LogicalSize::new(width, height)));
}

/// 保存窗口几何到状态表
///
/// **位置按外框、尺寸按客户区**（各自与恢复端同源）：
/// - `.position()` 设置的是外框位置 → 存 `outer_position()`
/// - 尺寸存 `inner_size()`（客户区），再**折算回「带装饰」口径**存下去
///   （折算量见 [`decoration_growth`]）
///
/// 为什么尺寸不用 `outer_size()`：注册表里的最小尺寸、恢复端的 `.inner_size()`
/// 都是**客户区**口径，存外框的话每一处都得再减一次边框带；而 `inner_size()` 的
/// **含义会随装饰状态变化** —— 有原生 frame 时它是"客户区"，插件激活调
/// `set_decorations(false)` 之后，同一块窗口的客户区会多出标题栏那一段。本函数在
/// 两种状态下都会被调用（建窗后立刻存一次、关窗时再存一次），于是存进去的高度会在
/// 600 / 621 / 630 之间来回漂，用户看到的就是"每次开关高度都变"
/// （实测主窗口注册表定义 600，文件里出现过 621 与 630）。折算就是为了消掉这个漂移。
///
/// 两条统一规则：
/// - 尺寸夹到不小于注册表里的最小**客户区**（见 [`min_inner_physical`]）；
/// - `maximized`（最大化 / 全屏）为真时**不覆盖几何**，只记下这个标志。最大化窗口的
///   `outer_position()` 带着框外偏移（常见是 −8），当成普通位置存下来，下次开窗
///   `monitor_from_point` 可能解析到**另一块显示器**，而且开出来还不是最大化。
pub(super) fn save_window_state(
    uuid: &Uuid,
    window: &WebviewWindow,
    maximized: bool,
) -> Result<(), String> {
    let pos = window.outer_position().map_err(|err| err.to_string())?;
    let size = window.inner_size().map_err(|err| err.to_string())?;

    let mut geom = window_state_for(uuid).unwrap_or_default();

    let scale = window.scale_factor().unwrap_or(1.0);
    // 夹到不小于注册表里的最小客户区：文件里出现比最小值还小的尺寸，下次开窗就会以非法尺寸
    // 创建，而且会被反复写回、一直不收敛（真实例子：窗口停在「外框 = 最小尺寸」上时，
    // 客户区比最小值小了一圈带宽，主窗口 920×600 → 904×600）。
    let (min_w, min_h) = min_inner_physical(min_size_of(uuid).unwrap_or((0.0, 0.0)), scale);

    if !maximized {
        // ==== 高度漂移的修复：统一折算到「带装饰」口径再存 ====
        //
        // 实测同一窗口的两种状态：
        //   inner=968x639  外框−客户区 = 16x39  ← 带装饰（含边框带 + 标题栏）
        //   inner=968x669  外框−客户区 = 16x9   ← 摘装饰后，外框不变、客户区 +30
        // 也就是说 `set_decorations(false)` 会**撑大客户区**（把标题栏那 30px 并进去），
        // 而保存动作在两种状态下都会发生（建窗后立刻存一次、关窗时再存一次），于是：
        //   存 639 → 下次按 639 建窗 → 激活后客户区变 669 → 存 669 → 再 +30 …
        // 每轮净增 30，就是"高度持续变高"。
        //
        // 修法：把无装饰状态下的 inner 折算回带装饰口径（与建窗时 `.inner_size()`
        // 一致），循环即收敛。折算量见 [`decoration_growth`]。
        let (dw, dh) = decoration_growth(uuid, window);
        geom.x = pos.x;
        geom.y = pos.y;
        geom.width = size.width.saturating_sub(dw).max(min_w);
        geom.height = size.height.saturating_sub(dh).max(min_h);
    }
    geom.maximized = maximized;
    window_state_set(uuid, geom);

    Ok(())
}

/// 当前窗口因「摘掉原生装饰」而多出来、需要从 `inner` 里减掉的客户区尺寸（物理像素）
///
/// 原理：`set_decorations(false)` 会**保持外框不变**、把标题栏那一段并进客户区
/// （实测：同一窗口外框恒为 984×678，`inset` 从 16×39 变成 16×9，客户区 +30）。
/// 要存的口径是**带装饰时的客户区**（与建窗时 `.inner_size()` 一致），
/// 所以无装饰时得把这个增量减掉。
///
/// 增量 = 当前 `(外框 − 客户区)` 与**带装饰时** `(外框 − 客户区)` 的差：
/// 带装饰时该差值含标题栏，无装饰时不含，两者相减正好是标题栏那一段。
/// 带装饰的值从 [`DECORATED_INSET`] 取（建窗那一刻量的）。
///
/// 所以：
/// - 带装饰时：当前 == 缓存 → 增量 0（本来就是对的口径，不动）
/// - 无装饰时：当前比缓存小一个标题栏 → 增量 = 标题栏高度（减掉它）
///
/// 不用写死 30：标题栏与边框带都随 DPI 变，这里全部现量。
pub(super) fn decoration_growth(uuid: &Uuid, window: &WebviewWindow) -> (u32, u32) {
    let (Ok(inner), Ok(outer)) = (window.inner_size(), window.outer_size()) else {
        return (0, 0);
    };
    let now_inset = (
        outer.width.saturating_sub(inner.width),
        outer.height.saturating_sub(inner.height),
    );

    let cached = DECORATED_INSET.read().unwrap().get(uuid).copied();
    let Some(decorated) = cached else {
        // 没缓存到（本进程没经过建窗路径）：不折算，保持原值
        return (0, 0);
    };

    (
        decorated.0.saturating_sub(now_inset.0),
        decorated.1.saturating_sub(now_inset.1),
    )
}
