use std::path::PathBuf;

pub mod assets_path;
pub mod version_path;
pub mod instance_path;
pub mod libraies_path;

pub fn init_minecraft_path(dir: &PathBuf) {
    assets_path::init(dir);
    version_path::init(dir);
    instance_path::init(dir);
    libraies_path::init(dir);
}