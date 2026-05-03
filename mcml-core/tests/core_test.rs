use std::{env};

use mcml_core::{self, CoreInitObj};

#[test]
fn core_test() {
    let exe_path = env::current_exe().expect("Failed to get exe path");
    let exe_dir = exe_path.parent().expect("Failed to get exe directory");
    let run_dir = exe_dir.parent().unwrap();

    println!("{}", run_dir.display());

    let config = CoreInitObj {
        local: run_dir.join("mcml"),
        oauth_key: String::new(),
        curseforge_key: String::new(),
    };
    mcml_core::init(config);
    assert!(mcml_core::get_state());
    mcml_core::stop();
}
