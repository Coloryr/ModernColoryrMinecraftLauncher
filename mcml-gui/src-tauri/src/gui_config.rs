//! GUI 状态（gui_config.json）
//!
//! 配置文件使用 Rust 命名（snake_case 字段 / PascalCase 枚举 / locale 变体名）。
//! 仅当经 Tauri IPC 传输到前端时转成 `crate::dtos::GuiConfigDto`（TS 命名，camelCase）。
//! 读写统一走 [`init`] 初始化后的内存缓存，保存经 `config_save` 异步落盘。

use std::{
    path::{Path, PathBuf},
    sync::{Arc, OnceLock, RwLock},
};

use mcml_base::serialize_tools;
use mcml_config::config_save;
use mcml_names::{Lang, names, uuids};
use serde::{Deserialize, Serialize};

/// 主题（serde 按变体名序列化，与前端同名）
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum Theme {
    #[default]
    Dark,
    Light,
}

/// 窗口模式：多窗口 / 单窗口
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum WindowMode {
    #[default]
    Multi,
    Single,
}

/// 侧栏位置
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum SidebarSide {
    #[default]
    Left,
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
            x: 0.0,
            y: 0.0,
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
        }
    }
}

static FILE: OnceLock<PathBuf> = OnceLock::new();
static CONFIG: OnceLock<Arc<RwLock<GuiConfig>>> = OnceLock::new();

/// 读取配置（启动时调用）
pub fn init<P: AsRef<Path>>(path: P) {
    let file = FILE.get_or_init(|| path.as_ref().join(names::GUI_CONFIG_FILE));

    let config = if file.exists()
        && file.is_file()
        && let Ok(data) = serialize_tools::json_from_file::<GuiConfig>(file)
    {
        data
    } else {
        GuiConfig::default()
    };
    let _ = CONFIG.set(Arc::new(RwLock::new(config)));
}

/// 当前配置
pub fn get() -> GuiConfig {
    CONFIG
        .get()
        .map(|c| c.read().unwrap().clone())
        .unwrap_or_default()
}

/// 更新配置并保存
pub fn set(config: GuiConfig) {
    if let Some(c) = CONFIG.get() {
        *c.write().unwrap() = config;
    }
    save();
}

/// 保存配置（异步写入 gui_config.json）
pub fn save() {
    let Some(file) = FILE.get() else {
        return;
    };
    let data = CONFIG.get().unwrap().read().unwrap();
    config_save::save(uuids::GUI_CONFIG_FILE_UUID, &*data, file);
}
