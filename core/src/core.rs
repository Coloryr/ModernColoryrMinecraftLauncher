pub mod core {
    use std::sync::OnceLock;

    use const_format::formatcp;

    use crate::objs::core_init_obj::CoreInitObj;

    pub const VERSION_NUM: i32 = 40;
    pub const DATE: &str = "20260412";
    pub const VERSION: &str = formatcp!("{}-{DATE}", VERSION_NUM);

    pub static BASE_DIR: OnceLock<String> = OnceLock::new();
    pub static CORE_ARG: OnceLock<CoreInitObj> = OnceLock::new();

    pub fn init(arg: CoreInitObj) {
        if arg.local.is_empty() {
            panic!("Run local is empty");
        }
    }
}
