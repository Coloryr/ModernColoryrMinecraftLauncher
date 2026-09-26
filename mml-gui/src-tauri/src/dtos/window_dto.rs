//! 窗口尺寸 DTO——前端 JS 回退路径创建窗口时从后端取默认宽高。
//!
//! 窗口几何的唯一数据源在后端（`WINDOWS_INFO` 的默认尺寸 + `window_save.json`
//! 的历史几何），前端不再重复维护一份宽高表。

use serde::{Deserialize, Serialize};

/// 单个窗口的默认尺寸（kind 为前端窗口标识，如 "settings"）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowSizeDto {
    pub kind: String,
    pub width: f64,
    pub height: f64,
}
