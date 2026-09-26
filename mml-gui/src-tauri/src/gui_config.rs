//! GUI 状态（gui_config.json）
//!
//! 配置文件使用 Rust 命名（snake_case 字段 / PascalCase 枚举 / locale 变体名）。
//! 仅当经 Tauri IPC 传输到前端时转成 `crate::dtos::GuiConfigDto`（TS 命名，camelCase）。
//! 读写统一走 [`init`] 初始化后的内存缓存，保存经 `config_save` 异步落盘。

use std::{
    path::{Path, PathBuf},
    sync::{
        Arc, OnceLock, RwLock,
        atomic::{AtomicBool, Ordering},
    },
};

use mml_base::serialize_tools;
use mml_config::config_save;
use mml_names::{Lang, names, uuids};
use serde::{Deserialize, Serialize};

/// 主题（serde 按变体名序列化，与前端同名）
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum Theme {
    /// 跟随系统深浅色
    #[default]
    System,
    /// 浅色
    Light,
    /// 深色
    Dark,
}

/// 窗口模式：多窗口 / 单窗口
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum WindowMode {
    /// 多窗口（默认）
    #[default]
    Multi,
    /// 单窗口
    Single,
}

/// 侧栏位置
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum SidebarSide {
    /// 左侧（默认）
    #[default]
    Left,
    /// 右侧
    Right,
}

/// 主窗口实例列表显示模式（wire 名小写，与前端一致）
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum ViewMode {
    /// 列表（默认）
    #[default]
    List,
    /// 分组
    Group,
    /// 平铺
    Grid,
}

/// 皮肤显示模式（serde 按变体名序列化，与前端同名）
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum SkinDisplay {
    /// 2D 平面图（TypeA，默认；兼容旧值的 "Skin2D" 别名）
    #[default]
    #[serde(alias = "Skin2D")]
    Skin2DA,
    /// 2D 大图（TypeB）
    Skin2DB,
    /// 3D 等距模型（Rust 侧 skin_3d_draw 渲染）
    Skin3D,
}

/// 头像类型（serde 按变体名序列化，与前端同名）
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum HeadType {
    /// 2D 平面头像（默认）
    #[default]
    Head2DA,
    /// 3D 头像，不旋转
    Head3DA,
    /// 3D 头像，按 X / Y 旋转
    Head3DB,
    /// 2D 大头像
    Head2DB,
}

/// 头像设置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct HeadConfig {
    /// 头像类型：Head2DA / Head3DA / Head3DB / Head2DB
    pub head_type: HeadType,
    /// 3D 旋转 X
    pub x: f32,
    /// 3D 旋转 Y
    pub y: f32,
}

impl Default for HeadConfig {
    fn default() -> Self {
        Self {
            head_type: HeadType::default(),
            x: 15.0,
            y: 65.0,
        }
    }
}

/// 主窗口配置（字段名即 wire 名，与前端同名）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct MainWindowConfig {
    /// 侧栏位置：Left / Right
    pub sidebar_side: SidebarSide,
    /// 侧栏是否收起
    pub sidebar_collapsed: bool,
    /// 实例列表显示模式：list（默认）/ group / grid
    pub view_mode: ViewMode,
    /// 当前选中实例 uuid（空串 = 未选中）
    pub selected_instance: String,
}

impl Default for MainWindowConfig {
    fn default() -> Self {
        Self {
            sidebar_side: Default::default(),
            // 默认收起侧栏（首次启动 / 无配置时）
            sidebar_collapsed: true,
            // 默认列表模式（用户改过后用用户的值）
            view_mode: ViewMode::default(),
            // 默认未选中
            selected_instance: String::new(),
        }
    }
}

/// 收藏界面设置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CollectConfig {
    /// 显示整合包
    pub modpack: bool,
    /// 显示模组
    pub show_mod: bool,
    /// 显示资源包
    pub resource_pack: bool,
    /// 显示光影包
    pub shaderpack: bool,
}

impl Default for CollectConfig {
    fn default() -> Self {
        Self {
            modpack: true,
            show_mod: true,
            resource_pack: true,
            shaderpack: true,
        }
    }
}

/// 界面设置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GuiConfig {
    /// 主题：Dark / Light
    pub theme: Theme,
    /// 语言：zh_cn / en_us（core Lang）
    pub locale: Lang,
    /// 窗口模式：Multi / Single
    pub window_mode: WindowMode,
    /// 主窗口配置
    pub main_window: MainWindowConfig,
    /// 头像设置
    pub head: HeadConfig,
    /// 收藏界面设置
    pub collect: CollectConfig,
    /// 界面字体族名（空串 = 默认字体栈）
    pub font: String,
    /// 皮肤显示模式：Skin2D / Skin3D
    pub skin_display: SkinDisplay,
    /// 背景图来源（文件路径 / 网址，空串 = 无背景图）
    pub bg_source: String,
    /// 背景图不透明度（%，5–100）
    pub bg_opacity: u32,
    /// 背景图模糊（px，0–40）
    pub bg_blur: u32,
    /// 背景图原始分辨率（%，10–100，后端按此把源图缩放后落盘）
    pub bg_native_size: u32,
}

impl Default for GuiConfig {
    fn default() -> Self {
        Self {
            theme: Default::default(),
            locale: Lang::zh_cn,
            window_mode: Default::default(),
            main_window: Default::default(),
            head: Default::default(),
            collect: Default::default(),
            font: String::new(),
            skin_display: Default::default(),
            bg_source: String::new(),
            bg_opacity: 100,
            bg_blur: 0,
            bg_native_size: 100,
        }
    }
}

/// 配置文件路径（init 时设置）
static FILE: OnceLock<PathBuf> = OnceLock::new();
/// 内存配置缓存（init 时设置）
static CONFIG: OnceLock<Arc<RwLock<GuiConfig>>> = OnceLock::new();
/// 启动时没有配置文件（用户还没做过任何显式设置，默认值应跟随系统主题）
static FRESH_CONFIG: AtomicBool = AtomicBool::new(false);

/// 读取配置（启动时调用）
///
/// # 参数
///
/// - `path`: 运行路径（配置文件放在其下）
pub fn init<P: AsRef<Path>>(path: P) {
    let file = FILE.get_or_init(|| path.as_ref().join(names::GUI_CONFIG_FILE));

    let config = if file.exists()
        && file.is_file()
        && let Ok(data) = serialize_tools::json_from_file::<GuiConfig>(file)
    {
        data
    } else {
        FRESH_CONFIG.store(true, Ordering::Release);
        GuiConfig::default()
    };
    let _ = CONFIG.set(Arc::new(RwLock::new(config)));
}

/// 启动时是否没有配置文件（前端据此让默认主题跟随系统）
///
/// # 返回值
///
/// 无配置文件返回 `true`
pub fn is_fresh() -> bool {
    FRESH_CONFIG.load(Ordering::Acquire)
}

/// 当前配置
///
/// # 返回值
///
/// 返回配置快照；未初始化返回默认值
pub fn get() -> GuiConfig {
    CONFIG
        .get()
        .map(|c| c.read().unwrap().clone())
        .unwrap_or_default()
}

/// 配置目录（背景图等派生文件放这里；未初始化返回 None）
pub fn dir() -> Option<PathBuf> {
    FILE.get().and_then(|p| p.parent()).map(Path::to_path_buf)
}

/// 更新配置并保存
///
/// # 参数
///
/// - `config`: 新配置
pub fn set(config: GuiConfig) {
    if let Some(c) = CONFIG.get() {
        *c.write().unwrap() = config;
    }
    // 用户做过显式设置：配置已落盘，不再视为首次启动
    FRESH_CONFIG.store(false, Ordering::Release);
    save();
}

/// 保存配置（异步写入 gui_config.json；未初始化时忽略）
pub fn save() {
    let Some(file) = FILE.get() else {
        return;
    };
    let data = CONFIG.get().unwrap().read().unwrap();
    config_save::save(uuids::GUI_CONFIG_FILE_UUID, &*data, file);
}
