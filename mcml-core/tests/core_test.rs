use mcml_core::core::{self, CoreInitObj};

#[test]
fn core_test() {
    let config = CoreInitObj {
        local: String::from("./test_run"),
        oauth_key: String::new(),
        curseforge_key: String::new(),
    };
    core::init(config);
    assert!(core::get_state());
    core::stop();
}
