use std::env;

use mml_net::adoptium_api;

fn init() {
    let exe_path = env::current_exe().expect("Failed to get exe path");
    let exe_dir = exe_path.parent().expect("Failed to get exe directory");
    let run_dir = exe_dir.parent().unwrap().to_path_buf();

    mml_log::start(&run_dir);
    mml_config::init(&run_dir);
    mml_net::init();
}

fn stop() {
    mml_log::stop();
}

async fn get_adoptium() {
    let list = adoptium_api::get_java_version().await;
    assert!(list.is_ok());

    let list = list.unwrap();
    assert!(list.contains(&String::from("8")));
}

#[tokio::test]
async fn java_test() {
    init();

    get_adoptium().await;

    stop();
}
