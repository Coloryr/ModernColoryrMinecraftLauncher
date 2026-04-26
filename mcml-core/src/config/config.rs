pub mod config {
    use std::sync::OnceLock;

    use crate::config::config_obj::Config;

    static CONFIG: OnceLock<Config> = OnceLock::new();

    pub fn init(local: String) {
        
    }
}