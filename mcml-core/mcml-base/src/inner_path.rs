use std::{env, fs, path::PathBuf, sync::LazyLock};

use crate::Os;

static INNER: LazyLock<PathBuf> = LazyLock::new(|| {
    let inner_path = if crate::get_system_info().os == Os::MacOS {
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
});

pub fn get_inner_path() -> PathBuf {
    INNER.clone()
}
