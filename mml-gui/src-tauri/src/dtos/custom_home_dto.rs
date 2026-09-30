//! 自定义主页面 DTO（TS 命名，camelCase wire）
//!
//! 自定义主页面的本体是 `base_dir/custom_home/` 目录（服主导入的整页 HTML），
//! 磁盘上不存任何派生状态，这里的状态每次都由 `custom_home_status` 现算。
use serde::{Deserialize, Serialize};

/// 自定义主页面状态（`custom_home_status` / `custom_home_import` 返回值）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomHomeInfoDto {
    /// 是否已导入（目录存在且入口页存在）
    pub installed: bool,
    /// 配置文件里的启用开关（client.custom_home）
    pub enabled: bool,
    /// 入口页完整 URL（未导入时为空串），已带破缓存参数
    pub entry_url: String,
    /// 内容版本号（取 custom_home 目录 mtime），用于 iframe 破缓存
    pub version: String,
    /// 解包出的文件数
    pub file_count: u32,
}

/// 自定义主页面导入进度（`custom-home-progress` 事件）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomHomeProgressDto {
    /// 当前阶段 ID（extract 解包）
    pub state: String,
    /// 当前进度
    pub now: u32,
    /// 总量
    pub total: u32,
    /// 说明文本（当前文件名）
    pub sub_text: String,
}
