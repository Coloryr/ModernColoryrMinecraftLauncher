//! 在线版本接口测试（依赖网络，默认忽略）。
//!
//! 运行方式：`cargo test --test online -- --ignored --nocapture`

use std::path::PathBuf;
use std::sync::OnceLock;

/// 临时运行目录（每次运行前先删除旧目录）
fn init_all() -> PathBuf {
    static INIT: OnceLock<PathBuf> = OnceLock::new();
    INIT.get_or_init(|| {
        let run_dir = std::env::temp_dir().join("mcml-game-online-test");
        let _ = std::fs::remove_dir_all(&run_dir);
        std::fs::create_dir_all(&run_dir).expect("创建测试运行目录失败");

        mcml_base::init(&run_dir);
        mcml_names::init(mcml_base::get_base_dir()).unwrap();
        mcml_log::start(mcml_base::get_base_dir()).unwrap();
        mcml_config::init(mcml_base::get_base_dir()).unwrap();
        mcml_game::init(mcml_base::get_base_dir()).unwrap();
        mcml_net::init();
        run_dir
    })
    .clone()
}

/// 在线检查版本更新：能拉到指定版本的启动参数 JSON 并落盘缓存。
#[tokio::test]
#[ignore = "依赖 Mojang 在线接口，仅在手动验证网络功能时运行"]
async fn check_update_online() {
    init_all();

    let obj = mcml_game::launcher_path::version_path::check_update("1.20.4")
        .await
        .expect("在线检查版本失败");
    assert_eq!(obj.id, "1.20.4");
    assert!(!obj.main_class.is_empty());

    // 再次调用应命中本地缓存
    let cached = mcml_game::launcher_path::version_path::get_version("1.20.4")
        .expect("本地缓存版本失败");
    assert_eq!(cached.id, "1.20.4");
}

/// 在线检查不存在的版本号：应返回版本未找到错误而不是 panic。
#[tokio::test]
#[ignore = "依赖 Mojang 在线接口，仅在手动验证网络功能时运行"]
async fn check_update_unknown_version() {
    init_all();

    let res = mcml_game::launcher_path::version_path::check_update("0.0.0-not-exist").await;
    assert!(res.is_err(), "不存在的版本号应返回错误");
}
