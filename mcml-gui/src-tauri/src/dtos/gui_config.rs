//! GUI 配置前端 DTO（TS 命名，camelCase wire）
//!
//! 配置文件 `gui_config.json` 落盘用 Rust 命名（见 `../gui_config.rs`），
//! 仅当跨 Tauri IPC 传输到前端时转成此结构：
//! - 字段 camelCase = 前端 `mcml-vue/src/lib/guiConfig.ts` 的类型名
//! - 枚举值 = Rust 变体名（"Dark" / "Light" / "Multi" / "Single" / "Left" / "Right"）
//! - locale 为字符串（core `Lang` 未实现 serde）

use serde::{Deserialize, Serialize};

use mcml_names::Lang;

use crate::gui_config::{GuiConfig, MainWindowConfig, SidebarSide, Theme, WindowMode};

/// 主窗口配置（前端 wire：camelCase，字段即 `mainWindow`）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct MainWindowConfigDto {
    /// 侧栏位置：Left / Right
    pub sidebar_side: SidebarSide,
    /// 侧栏是否收起
    pub sidebar_collapsed: bool,
}

/// GUI 配置（前端 wire：camelCase，即 `window_get_gui_config` 返回值）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct GuiConfigDto {
    /// 主题：Dark / Light
    pub theme: Theme,
    /// 语言：zh_cn / en_us
    pub locale: Lang,
    /// 窗口模式：Multi / Single
    pub window_mode: WindowMode,
    /// 主窗口配置
    pub main_window: MainWindowConfigDto,
}

impl From<GuiConfig> for GuiConfigDto {
    fn from(c: GuiConfig) -> Self {
        Self {
            theme: c.theme,
            locale: c.locale,
            window_mode: c.window_mode,
            main_window: MainWindowConfigDto {
                sidebar_side: c.main_window.sidebar_side,
                sidebar_collapsed: c.main_window.sidebar_collapsed,
            },
        }
    }
}

impl From<GuiConfigDto> for GuiConfig {
    fn from(d: GuiConfigDto) -> Self {
        Self {
            theme: d.theme,
            locale: d.locale,
            window_mode: d.window_mode,
            main_window: MainWindowConfig {
                sidebar_side: d.main_window.sidebar_side,
                sidebar_collapsed: d.main_window.sidebar_collapsed,
            },
        }
    }
}
