//! GUI 状态（gui_config.json）
//!
//! 与前端 `mcml-vue/src/lib/guiConfig.ts` 对应：
//! theme / locale / windowMode / sidebarSide / sidebarCollapsed。
//! 读写统一走 [`init`] 初始化后的内存缓存，保存经 `config_save` 异步落盘。

use std::{
    path::{Path, PathBuf},
    sync::{Arc, OnceLock, RwLock},
};

use mcml_base::serialize_tools;
use mcml_config::config_save;
use mcml_names::{names, uuids};
use serde::{Deserialize, Serialize};

/// GUI 状态（gui_config.json）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct GuiConfig {
    /// 主题：dark / light
    pub theme: String,
    /// 语言：zh-CN / en-US
    pub locale: String,
    /// 窗口模式：multi / single
    pub window_mode: String,
    /// 侧栏位置：left / right
    pub sidebar_side: String,
    /// 侧栏是否收起
    pub sidebar_collapsed: bool,
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
