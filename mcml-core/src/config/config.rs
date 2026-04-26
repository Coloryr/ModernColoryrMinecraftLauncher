pub mod config {
    use std::sync::OnceLock;

    use crate::config::config_obj::ConfigObj;

    static CONFIG: OnceLock<ConfigObj> = OnceLock::new();

    pub fn init(local: String) {
        
    }
}