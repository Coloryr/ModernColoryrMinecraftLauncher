use std::{path::{Path, PathBuf}, sync::OnceLock};

use mcml_base::serialize_tools;
use mcml_names::names;
use mcml_sys::path_helper;

/// 窗口几何状态（windows.json）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct WindowState {
    pub uuid: String,
    /// 窗口标签（main / mcml-settings …）
    pub label: String,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

static FILE: OnceLock<PathBuf> = OnceLock::new();
// static FILE: OnceLock<PathBuf> = OnceLock::new();

/// 主窗口固定 UUID
pub const MAIN_WINDOW_UUID: &str = "8f6b1c2e-3d4a-4e5b-9c6d-7e8f9a0b1c2d";

pub fn init<P: AsRef<Path>>(path: P) {
    let file = FILE.get_or_init(|| path.as_ref().join(names::WINDOW_SAVE_FILE));

    if file.exists() && file.is_file() {
        let windows= serialize_tools::json_from_file::<>(file);
    }
}