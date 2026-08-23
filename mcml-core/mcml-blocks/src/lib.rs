use std::{
    path::{Path, PathBuf},
    sync::OnceLock,
};

use mcml_names::{i18_items::error_type::CoreResult, names};
use mcml_sys::path_helper;

pub mod block_database;

static BLOCK_FILE: OnceLock<PathBuf> = OnceLock::new();
static BLOCK_DIR: OnceLock<PathBuf> = OnceLock::new();

pub fn init<P: AsRef<Path>>(path: P) -> CoreResult<()> {
    BLOCK_FILE.get_or_init(|| path.as_ref().join(names::BLOCK_FILE));

    let dir = BLOCK_DIR.get_or_init(|| path.as_ref().join(names::BLOCK_DIR));
    if !dir.exists() {
        path_helper::create_dir_all(dir)?;
    }

    Ok(())
}
