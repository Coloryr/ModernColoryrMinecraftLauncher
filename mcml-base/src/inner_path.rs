use std::{env, fs, path::PathBuf};

use crate::{Os, get_system_info};

lazy_static::lazy_static! {
    static ref INNER: PathBuf = {
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
    };
}

pub fn inner() -> &'static PathBuf {
    &INNER
}
