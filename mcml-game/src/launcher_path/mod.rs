use std::{
    path::{Path, PathBuf},
    sync::LazyLock,
};

use mcml_base::path_helper;
use mcml_names::{i18_items::error_type::CoreResult, names};

pub mod assets_path;
pub mod instance_path;
pub mod libraries_path;
pub mod version_path;

const COLORASM_FILE: &[u8] = include_bytes!("../../assets/ColorASM-1.1-all.jar");

static COLORASM: LazyLock<PathBuf> = LazyLock::new(|| {
    let local = libraries_path::get_lib_dir()
        .join("com")
        .join("coloryr")
        .join("colorasm")
        .join("1.1")
        .join("ColorASM-1.1-all.jar");

    local
});

const WRAPPER_FILE: &[u8] = include_bytes!("../../assets/ForgeWrapper-prism-2025-12-07.jar");

static FORGE_WRAPPER: LazyLock<PathBuf> = LazyLock::new(|| {
    let local = libraries_path::get_lib_dir()
        .join("io")
        .join("github")
        .join("zekerzhayard")
        .join("prism-2025-12-07")
        .join("ForgeWrapper-prism-2025-12-07.jar");

    local
});

const OPTIFINE_FILE: &[u8] = include_bytes!("../../assets/OptifineWrapper-1.1.jar");

static OPTIFINE_WRAPPER: LazyLock<PathBuf> = LazyLock::new(|| {
    let local = libraries_path::get_lib_dir()
        .join("com")
        .join("coloryr")
        .join("optifinewrapper")
        .join("1.1")
        .join("optifinewrapper-1.1.jar");

    local
});

/// 初始化文件夹
/// - `dir`: 工作的目录
pub fn init<P: AsRef<Path>>(dir: P) -> CoreResult<()> {
    let dir = dir.as_ref().join(names::MINECRAFT);
    if !dir.exists() {
        path_helper::create_dir_all(&dir)?;
    }

    assets_path::init(&dir)?;
    version_path::init(&dir)?;
    instance_path::init(&dir)?;
    libraries_path::init(&dir)?;

    Ok(())
}

/// 准备ForgeWrapper jar
pub fn ready_forge_wrapper() -> CoreResult<PathBuf> {
    let local = FORGE_WRAPPER.clone();

    if !local.exists() {
        path_helper::write_bytes(&local, WRAPPER_FILE)?;
    }

    Ok(local)
}

pub fn ready_colorasm() -> CoreResult<PathBuf> {
    let local = COLORASM.clone();

    if !local.exists() {
        path_helper::write_bytes(&local, COLORASM_FILE)?;
    }

    Ok(local)
}

pub fn ready_optifine_wrapper() -> CoreResult<PathBuf> {
    let local = OPTIFINE_WRAPPER.clone();

    if !local.exists() {
        path_helper::write_bytes(&local, OPTIFINE_FILE)?;
    }

    Ok(local)
}
