use std::path::Path;

use serde::{Deserialize, Serialize};

/// GUI 状态（gui_config.json）
#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct GuiConfig {
    /// 主题：dark / light
    pub theme: String,
    /// 语言：zh-CN / en-US
    pub locale: String,
    /// 窗口模式：multi / single
    pub window_mode: String,
}

impl Default for GuiConfig {
    fn default() -> Self {
        Self {
            theme: "dark".into(),
            locale: "zh-CN".into(),
            window_mode: "multi".into(),
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub enum GuiSide {
    Left,
    Right,
}

impl Default for GuiSide {
    fn default() -> Self {
        GuiSide::Left
    }
}

#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct GuiMainConfig {
    /// 侧栏位置：left / right
    pub sidebar_side: GuiSide,
    /// 侧栏是否收起
    pub sidebar_collapsed: bool,
}

impl Default for GuiMainConfig {
    fn default() -> Self {
        Self {
            sidebar_side: GuiSide.Left,
            sidebar_collapsed: false,
        }
    }
}

pub fn init<P: AsRef<Path>>(path: P) {}
