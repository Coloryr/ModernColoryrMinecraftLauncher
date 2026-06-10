use std::{env, fs, path::PathBuf, sync::LazyLock};

use crate::{Os, get_system_info};

static INNER: LazyLock<PathBuf> = LazyLock::new(|| get_inner_path());

pub fn inner() -> PathBuf {
    INNER.clone()
}

/// 初始化内部路径
fn get_inner_path() -> PathBuf {
    let inner_path = if get_system_info().os == Os::MacOS {
        let home = env::var("HOME").expect("");
        PathBuf::from(home).join(".mcml")
    } else {
        let local_app_data = env::var("LOCALAPPDATA")
            .or_else(|_| env::var("HOME").map(|h| format!("{}/.local/share", h)))
            .expect("");
        PathBuf::from(local_app_data).join("mcml")
    };

    if !inner_path.exists() {
        fs::create_dir_all(&inner_path).expect("");
    }

    inner_path
}
