use std::env;

use mcml_net::net::adoptium_api;

fn init() {
    let exe_path = env::current_exe().expect("Failed to get exe path");
    let exe_dir = exe_path.parent().expect("Failed to get exe directory");
    let run_dir = exe_dir.parent().unwrap().to_path_buf();

    mcml_log::start(&run_dir);
    mcml_config::init(&run_dir);
    mcml_http::init();
}

fn stop() {
    mcml_log::stop();
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
