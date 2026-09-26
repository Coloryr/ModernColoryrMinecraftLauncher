//! GUI 配置前端 DTO（TS 命名，camelCase wire）
//!
//! 配置文件 `gui_config.json` 落盘用 Rust 命名（见 `../gui_config.rs`），
//! 仅当跨 Tauri IPC 传输到前端时转成此结构：
//! - 字段 camelCase = 前端 `mml-vue/src/lib/guiConfig.ts` 的类型名
//! - 枚举值 = Rust 变体名（"Dark" / "Light" / "Multi" / "Single" / "Left" / "Right"）
//! - locale 为字符串（core `Lang` 未实现 serde）

use serde::{Deserialize, Serialize};

use mml_names::Lang;

use crate::gui_config::{
    CollectConfig, GuiConfig, HeadConfig, HeadType, MainWindowConfig, SidebarSide, SkinDisplay,
    Theme, ViewMode, WindowMode,
};

/// 头像设置（前端 wire：camelCase，字段即 `head`）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct HeadConfigDto {
    /// Head2DA / Head3DA / Head3DB / Head2DB
    pub head_type: HeadType,
    /// 3D 旋转 X
    pub x: f32,
    /// 3D 旋转 Y
    pub y: f32,
}

/// 主窗口配置（前端 wire：camelCase，字段即 `mainWindow`）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct MainWindowConfigDto {
    /// 侧栏位置：Left / Right
    pub sidebar_side: SidebarSide,
    /// 侧栏是否收起
    pub sidebar_collapsed: bool,
    /// 实例列表显示模式：list（默认）/ group / grid
    pub view_mode: ViewMode,
    /// 当前选中实例 uuid（空串 = 未选中）
    pub selected_instance: String,
}

/// 收藏界面设置（前端 wire：camelCase，字段即 `collect`）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct CollectConfigDto {
    /// 显示整合包
    pub modpack: bool,
    /// 显示模组
    pub show_mod: bool,
    /// 显示资源包
    pub resource_pack: bool,
    /// 显示光影包
    pub shaderpack: bool,
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
    /// 头像设置
    pub head: HeadConfigDto,
    /// 收藏界面设置
    pub collect: CollectConfigDto,
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
    /// 背景图原始分辨率（%，10–100）
    pub bg_native_size: u32,
}

impl From<GuiConfig> for GuiConfigDto {
    /// 磁盘配置 → 前端 wire 形态
    fn from(c: GuiConfig) -> Self {
        Self {
            theme: c.theme,
            locale: c.locale,
            window_mode: c.window_mode,
            main_window: MainWindowConfigDto {
                sidebar_side: c.main_window.sidebar_side,
                sidebar_collapsed: c.main_window.sidebar_collapsed,
                view_mode: c.main_window.view_mode,
                selected_instance: c.main_window.selected_instance,
            },
            head: HeadConfigDto {
                head_type: c.head.head_type,
                x: c.head.x,
                y: c.head.y,
            },
            collect: CollectConfigDto {
                modpack: c.collect.modpack,
                show_mod: c.collect.show_mod,
                resource_pack: c.collect.resource_pack,
                shaderpack: c.collect.shaderpack,
            },
            font: c.font,
            skin_display: c.skin_display,
            bg_source: c.bg_source,
            bg_opacity: c.bg_opacity,
            bg_blur: c.bg_blur,
            bg_native_size: c.bg_native_size,
        }
    }
}

impl From<GuiConfigDto> for GuiConfig {
    /// 前端 wire 形态 → 磁盘配置
    fn from(d: GuiConfigDto) -> Self {
        Self {
            theme: d.theme,
            locale: d.locale,
            window_mode: d.window_mode,
            main_window: MainWindowConfig {
                sidebar_side: d.main_window.sidebar_side,
                sidebar_collapsed: d.main_window.sidebar_collapsed,
                view_mode: d.main_window.view_mode,
                selected_instance: d.main_window.selected_instance,
            },
            head: HeadConfig {
                head_type: d.head.head_type,
                x: d.head.x,
                y: d.head.y,
            },
            collect: CollectConfig {
                modpack: d.collect.modpack,
                show_mod: d.collect.show_mod,
                resource_pack: d.collect.resource_pack,
                shaderpack: d.collect.shaderpack,
            },
            font: d.font,
            skin_display: d.skin_display,
            bg_source: d.bg_source,
            bg_opacity: d.bg_opacity,
            bg_blur: d.bg_blur,
            bg_native_size: d.bg_native_size,
        }
    }
}
