//! 真实整合包下载安装测试：
//! 1. Modrinth 真包（Fabulously Optimized 14.0.0-beta.3）：下载 .mrpack →
//!    完整安装链（含按包内清单从 Modrinth API 解析并下载全部 48 个 mod 文件）。
//! 2. CurseForge 真包（`#[ignore]` 手动测试）：CurseForge API 必须有 key（测试内
//!    不硬编码密钥，由环境变量 `MCML_CF_API_KEY` 注入），有 key 时搜索整合包 →
//!    下载 → 完整安装。
//!
//! 全局初始化每进程一次，互斥锁串行各用例。

use std::path::PathBuf;
use std::sync::{Arc, Mutex, Once};

use mcml_downloader::download_item::DownloadItem;
use mcml_game::add_game::{self, PackType};
use mcml_game::gui_hook::{AddModPackState, IAddModPackGui};
use mcml_game::loader::LoaderType;

mod common;

/// 测试运行目录（系统临时目录 + 进程号，避免多进程冲突）
fn run_dir() -> PathBuf {
    std::env::temp_dir().join(format!("mcml-real-pack-{}", std::process::id()))
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
        mcml_net::init();

        mcml_downloader::init(&dir).unwrap();
        mcml_downloader::set_gui_handel(Box::new(TestDownloader));
        mcml_downloader::start();
    });
}

/// 全局状态共享同一进程，串行执行各用例
static TEST_LOCK: Mutex<()> = Mutex::new(());

/// 下载真实 Modrinth 整合包（Fabulously Optimized 14.0.0-beta.3）到临时缓存。
///
/// 版本号硬编码：断言依赖该版本的具体字段（26.2 / fabric-loader 0.19.3），
/// 跟随最新版会随上游变动而失效。下载失败（网络不可用）返回 `None`。
async fn download_mrpack() -> Option<PathBuf> {
    use mcml_base::serialize_tools::json_from_bytes;
    use mcml_net::modrinth_api::version_obj::ModrinthVersionObj;

    let cache = std::env::temp_dir().join("mcml-real-pack-fo-14.0.0-beta.3.mrpack");
    if cache.exists() {
        return Some(cache);
    }

    let data = mcml_net::get_work_client()
        .get_bytes("https://api.modrinth.com/v2/project/fabulously-optimized/version")
        .await
        .ok()?;
    let versions: Vec<ModrinthVersionObj> = json_from_bytes(&data).ok()?;
    let target = versions
        .into_iter()
        .find(|v| v.version_number == "14.0.0-beta.3")?;
    let file = target
        .files
        .into_iter()
        .find(|f| f.primary || !f.url.is_empty())?;

    let data = mcml_net::get_work_client().get_bytes(&file.url).await.ok()?;
    if data.is_empty() {
        return None;
    }
    std::fs::write(&cache, &data).ok()?;
    Some(cache)
}

/// 安装阶段记录器（供断言走完所有阶段）
#[derive(Default, Clone)]
struct StateRecorder {
    states: Arc<Mutex<Vec<&'static str>>>,
}

impl StateRecorder {
    fn reached(&self, state: &str) -> bool {
        self.states.lock().unwrap().contains(&state)
    }
}

fn state_id(state: &AddModPackState) -> &'static str {
    match state {
        AddModPackState::DownloadPack => "downloadPack",
        AddModPackState::ReadInfo => "readInfo",
        AddModPackState::GetInfo => "getInfo",
        AddModPackState::DownloadFile => "downloadFile",
        AddModPackState::Extract => "extract",
        AddModPackState::Done => "done",
    }
}

impl IAddModPackGui for StateRecorder {
    fn set_state(&self, state: AddModPackState) {
        self.states.lock().unwrap().push(state_id(&state));
    }

    fn set_now(&self, _value: usize, _all: Option<usize>) {}

    fn set_sub_text(&self, _text: Option<String>) {}

    fn set_sub_now(&self, _value: usize, _all: Option<usize>) {}
}

struct TestDownloader;

impl mcml_downloader::IDownloadGui for TestDownloader {
    fn update(&self, _thread: u32, _file: &Arc<DownloadItem>) {}

    fn update_task(&self, _state: mcml_downloader::DownloadTaskState) {}
}

/// 真实 Modrinth 整合包完整安装（含全部 mod 文件下载，约 40+ 文件）。
#[tokio::test]
async fn install_real_modrinth_pack() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    ensure_init();

    // 真包从 Modrinth 下载并缓存（无网跳过）
    let Some(pack) = download_mrpack().await else {
        eprintln!("跳过: 网络不可用（无法下载 Modrinth 整合包）");
        return;
    };

    let recorder = StateRecorder::default();
    let uuid = add_game::install_archive_from_file(
        &pack,
        None,
        None,
        None,
        None,
        Some(Arc::new(recorder.clone())),
        None,
        PackType::Modrinth,
        tokio_util::sync::CancellationToken::new(),
    )
    .await
    .expect("真实 Modrinth 整合包安装失败");

    // 包元数据：minecraft 26.2 + fabric-loader 0.19.3
    let game = mcml_game::get_instance(&uuid).expect("安装后应能取到实例");
    let read = game.read().unwrap();
    assert!(!read.name.is_empty());
    assert_eq!(read.version, "26.2");
    assert!(matches!(read.loader, LoaderType::Fabric));
    assert_eq!(read.loader_version.as_deref(), Some("0.19.3"));
    assert!(read.is_modpack);
    drop(read);

    // 包内清单的全部 mod 都下载到了 mods 目录（约 48 个文件）
    let game = mcml_game::get_instance(&uuid).unwrap();
    let mods = game.read().unwrap().get_mods_path();
    let count = std::fs::read_dir(&mods)
        .expect("mods 目录应存在")
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().ends_with(".jar"))
        .count();
    assert!(count >= 40, "mods 目录应下载 40+ 个 jar，实际 {count}");

    for state in ["readInfo", "extract", "getInfo", "downloadFile", "done"] {
        assert!(recorder.reached(state), "缺少安装阶段: {state}");
    }

    mcml_game::delete_instance(&uuid).unwrap();
}

/// 真实 CurseForge 整合包完整安装（手动测试）。
///
/// CurseForge API 需要 key（GUI 运行时注入，测试内不硬编码），
/// 先设置环境变量再手动运行：
///
/// ```bash
/// MCML_CF_API_KEY='你的key' cargo test -p mcml-game --test real_pack_download \
///     install_real_curseforge_pack -- --ignored --nocapture
/// ```
#[tokio::test]
#[ignore]
async fn install_real_curseforge_pack() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    ensure_init();

    let Ok(key) = std::env::var("MCML_CF_API_KEY") else {
        eprintln!("跳过: 未设置 MCML_CF_API_KEY 环境变量（CurseForge API 需要 key）");
        return;
    };
    mcml_net::curseforge_api::set_key(&key);

    // 搜索整合包并取其最新文件（体积较小的 Fabulously Optimized）
    let list = mcml_net::curseforge_api::get_modpack_list(mcml_net::curseforge_api::CurseFogreArg {
        filter: Some("Fabulously Optimized".to_string()),
        page_size: Some(5),
        ..Default::default()
    })
    .await
    .expect("搜索 CurseForge 整合包失败");
    let Some(item) = list.data.first() else {
        eprintln!("跳过: 搜索结果为空");
        return;
    };
    println!("选中整合包: {} (id={})", item.name, item.id);

    let mut files = mcml_net::curseforge_api::get_files_page(mcml_net::curseforge_api::CurseFogreArg {
        id: Some(item.id.to_string()),
        page_size: Some(1),
        ..Default::default()
    })
    .await
    .expect("获取 CurseForge 文件列表失败");
    let Some(mut file) = files.data.first_mut() else {
        eprintln!("跳过: 整合包没有文件");
        return;
    };
    println!("选中文件: {} (id={})", file.display_name, file.id);

    let recorder = StateRecorder::default();
    let uuid = add_game::install_curseforge(
        &mut file,
        None,
        None,
        None,
        Some(Arc::new(recorder.clone())),
        None,
        tokio_util::sync::CancellationToken::new(),
    )
    .await
    .expect("真实 CurseForge 整合包安装失败");

    let game = mcml_game::get_instance(&uuid).expect("安装后应能取到实例");
    let read = game.read().unwrap();
    assert!(!read.name.is_empty());
    assert!(read.is_modpack);
    drop(read);

    for state in ["readInfo", "extract", "getInfo", "downloadFile", "done"] {
        assert!(recorder.reached(state), "缺少安装阶段: {state}");
    }

    mcml_game::delete_instance(&uuid).unwrap();
}
