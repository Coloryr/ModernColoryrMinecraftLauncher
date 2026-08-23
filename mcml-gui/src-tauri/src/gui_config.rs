use std::{path::{Path, PathBuf}, sync::{Arc, OnceLock, RwLock}};

use mcml_names::names;
use serde::{Deserialize, Serialize};

/// GUI 状态（gui_config.json）
#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct GuiConfig {
    /// 主题
    pub theme: Theme,
    /// 窗口模式
    pub window_mode: WindowModel,
    /// 主窗口配置
    pub main: GuiMainConfig,
}

#[derive(Serialize, Deserialize)]
pub enum Theme {
    Light,
    Dark,
}

#[derive(Serialize, Deserialize)]
pub enum WindowModel {
    Mulit,
    Single,
}

impl Default for GuiConfig {
    fn default() -> Self {
        Self {
            theme: Theme::Light,
            window_mode: WindowModel::Mulit,
            main: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct GuiMainConfig {
    /// 侧栏位置
    pub sidebar_side: GuiSide,
    /// 侧栏是否收起
    pub sidebar_collapsed: bool,
}

#[derive(Serialize, Deserialize)]
pub enum GuiSide {
    Left,
    Right,
}

impl Default for GuiMainConfig {
    fn default() -> Self {
        Self {
            sidebar_side: GuiSide::Left,
            sidebar_collapsed: false,
        }
    }
}

static FILE: OnceLock<PathBuf> = OnceLock::new();
static CONFIG: OnceLock<Arc<RwLock<GuiConfig>>> = OnceLock::new();

pub fn init<P: AsRef<Path>>(path: P) {
    path.as_ref().join(names::GUI_CONFIG_FILE)
}
