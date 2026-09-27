use std::env;

use mml_net::openj9_api;

fn init() {
    let exe_path = env::current_exe().expect("Failed to get exe path");
    let exe_dir = exe_path.parent().expect("Failed to get exe directory");
    let run_dir = exe_dir.parent().unwrap().to_path_buf();

    mml_base::init(&run_dir);
    mml_log::start(&run_dir).unwrap();
    mml_config::init(&run_dir).unwrap();
    mml_net::init();
}

#[tokio::test]
async fn test() {
    init();

    let data = openj9_api::get_java_list().await.unwrap();
}
