use std::path::PathBuf;

use mcml_base::path_helper;
use mcml_names::{i18_items::error_type::CoreResult, names};

pub mod assets_path;
pub mod instance_path;
pub mod libraies_path;
pub mod version_path;

/// 初始化文件夹
/// - `dir`: 工作的目录
pub fn init_minecraft_path(dir: &PathBuf) -> CoreResult<()> {
    let dir = &dir.join(names::MINECRAFT);
    if !dir.exists() {
        path_helper::create_dir_all(dir)?;
    }

    assets_path::init(dir)?;
    version_path::init(dir)?;
    instance_path::init(dir)?;
    libraies_path::init(dir)?;

    Ok(())
}
