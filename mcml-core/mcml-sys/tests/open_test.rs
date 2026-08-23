use std::env;

use mcml_sys::open_helper;

#[ignore = "reason"]
#[test]
pub fn open_file() {
    let exe_path = env::current_exe().expect("Failed to get exe path");
    let exe_dir = exe_path.parent().expect("Failed to get exe directory");
    let dir = exe_dir.parent().unwrap().parent().unwrap().parent().unwrap();

    println!("{}", dir.to_string_lossy());

    open_helper::open_file(dir.join(".gitignore"));
    // open_helper::open_file_with_explorer(dir.join("Cargo.toml"));
    // open_helper::open_file_with_explorer(dir);
}

#[ignore = "reason"]
#[test]
pub fn open_url() {
    open_helper::open_url("https://www.baidu.com");
}