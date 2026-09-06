//! 实例生命周期集成测试（离线）：创建、重名自动改名、改名、分组、删除。
//!
//! 不依赖网络与版本下载（版本号只是字符串标记，不真正启动游戏）。
//! 全局初始化每进程一次，互斥锁串行各用例。

use std::path::PathBuf;
use std::sync::{Mutex, Once};

use mcml_game::launcher::instance_setting_obj::InstanceSettingObj;
use uuid::Uuid;

/// 测试运行目录（系统临时目录 + 进程号，避免多进程冲突）
fn run_dir() -> PathBuf {
    std::env::temp_dir().join(format!("mcml-instance-lifecycle-{}", std::process::id()))
}

/// 初始化链（与 mcml_core::init 相同），每进程一次
fn ensure_init() {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        let dir = run_dir();
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        mcml_base::init(&dir);
        mcml_names::init(mcml_base::get_base_dir()).unwrap();
        mcml_log::start(mcml_base::get_base_dir()).unwrap();
        mcml_config::init(mcml_base::get_base_dir()).unwrap();
        mcml_config::config_save::start();
        mcml_game::init(mcml_base::get_base_dir()).unwrap();
    });
}

/// 全局实例表共享同一进程，串行执行各用例
static TEST_LOCK: Mutex<()> = Mutex::new(());

/// 用唯一前缀构造实例设置
fn setting(name: &str, group: Option<&str>) -> InstanceSettingObj {
    InstanceSettingObj {
        name: name.to_string(),
        group: group.map(String::from),
        version: "1.20.1".to_string(),
        ..Default::default()
    }
}

/// 创建：目录结构与全局表登记
#[tokio::test]
async fn create_instance_makes_dirs() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    ensure_init();

    let suffix = Uuid::new_v4().simple().to_string();
    let name = format!("生命周期-{suffix}");
    let game = setting(&name, Some("测试组"))
        .create_instance(None)
        .await
        .expect("创建实例失败");
    let uuid = game.read().unwrap().uuid;

    // 全局表登记
    assert!(mcml_game::have_instance_name(&name));
    assert!(mcml_game::get_instance_by_name(&name).is_some());
    assert!(mcml_game::get_group_keys().iter().any(|g| g == "测试组"));

    // 目录结构齐全
    let game2 = mcml_game::get_instance(&uuid).unwrap();
    let read = game2.read().unwrap();
    assert!(read.get_base_path().exists());
    assert!(read.get_game_path().exists());
    assert!(read.get_mods_path().exists());
    assert!(read.get_config_path().exists());
    assert!(read.get_logs_path().exists());
    assert!(read.get_saves_path().exists());
    assert!(read.get_resourcepacks_path().exists());
    assert_eq!(read.version, "1.20.1");
    drop(read);

    // 清理
    mcml_game::delete_instance(&uuid).unwrap();
    assert!(mcml_game::get_instance(&uuid).is_none());
    assert!(!mcml_game::have_instance_name(&name));
}

/// 重名创建（无界面）：自动改名 `{name}1`，两个实例共存
#[tokio::test]
async fn create_duplicate_name_auto_renames() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    ensure_init();

    let suffix = Uuid::new_v4().simple().to_string();
    let name = format!("重名-{suffix}");

    let game1 = setting(&name, None).create_instance(None).await.unwrap();
    let uuid1 = game1.read().unwrap().uuid;

    let game2 = setting(&name, None).create_instance(None).await.unwrap();
    let uuid2 = game2.read().unwrap().uuid;
    let name2 = game2.read().unwrap().name.clone();

    assert_ne!(uuid1, uuid2);
    assert_eq!(name2, format!("{name}1"), "重名实例应自动追加序号");
    assert!(mcml_game::have_instance_name(&name2));

    mcml_game::delete_instance(&uuid1).unwrap();
    mcml_game::delete_instance(&uuid2).unwrap();
}

/// 改名：目录跟随改名；新名重复被拒绝；空名被拒绝；改回原名允许
#[tokio::test]
async fn rename_instance_rules() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    ensure_init();

    let suffix = Uuid::new_v4().simple().to_string();
    let name = format!("改名-{suffix}");

    let game = setting(&name, None).create_instance(None).await.unwrap();
    let uuid = game.read().unwrap().uuid;

    // 改名成功，目录跟随
    let new_name = format!("改名后-{suffix}");
    mcml_game::rename_instance(&uuid, &new_name).expect("改名失败");
    let read = mcml_game::get_instance(&uuid).unwrap();
    let read = read.read().unwrap();
    assert_eq!(read.name, new_name);
    assert!(read.get_base_path().exists());
    assert!(read.get_base_path().file_name().unwrap().to_string_lossy() != name);
    drop(read);
    assert!(!mcml_game::have_instance_name(&name));
    assert!(mcml_game::have_instance_name(&new_name));

    // 改成别人占用的名字 → 拒绝
    let other = format!("改名占位-{suffix}");
    let game2 = setting(&other, None).create_instance(None).await.unwrap();
    let uuid2 = game2.read().unwrap().uuid;
    assert!(mcml_game::rename_instance(&uuid, &other).is_err());

    // 空名 → 拒绝
    assert!(mcml_game::rename_instance(&uuid, "   ").is_err());

    // 保持原名 → 允许（自己不算重名）
    mcml_game::rename_instance(&uuid, &new_name).expect("保持原名不应报错");

    mcml_game::delete_instance(&uuid).unwrap();
    mcml_game::delete_instance(&uuid2).unwrap();
}

/// 删除：从全局表与分组中移除，目录清理
#[tokio::test]
async fn delete_instance_removes_everywhere() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    ensure_init();

    let suffix = Uuid::new_v4().simple().to_string();
    let group = format!("删除组-{suffix}");
    let game = setting(&format!("待删-{suffix}"), Some(&group))
        .create_instance(None)
        .await
        .unwrap();
    let uuid = game.read().unwrap().uuid;
    let base = game.read().unwrap().get_base_path();

    assert!(mcml_game::get_group_keys().iter().any(|g| *g == group));

    mcml_game::delete_instance(&uuid).unwrap();

    assert!(mcml_game::get_instance(&uuid).is_none());
    assert!(
        !mcml_game::get_group_keys().iter().any(|g| *g == group),
        "删除唯一成员后空分组应被回收"
    );
    assert!(!base.exists(), "实例目录应被清理（回收站）");

    // 重复删除 → 报错（uuid 已不存在）
    assert!(mcml_game::delete_instance(&uuid).is_err());
}
