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

/// 主窗口配置（字段名即 wire 名，与前端同名）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct MainWindowConfig {
    /// 侧栏位置：Left / Right
    pub sidebar_side: SidebarSide,
    /// 侧栏是否收起
    pub sidebar_collapsed: bool,
}

impl Default for MainWindowConfig {
    fn default() -> Self {
        Self {
            sidebar_side: Default::default(),
            sidebar_collapsed: Default::default(),
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
}

impl Default for GuiConfig {
    fn default() -> Self {
        Self {
            theme: Theme::default(),
            locale: Lang::zh_cn,
            window_mode: WindowMode::default(),
            main_window: MainWindowConfig::default(),
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
