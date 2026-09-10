//! 全量渲染测试：走真实下载链（版本清单 → 客户端jar → 解包）分别渲染全部方块 / 全部物品
//!
//! 运行（需外网，首次会下载约 25MB 的客户端jar）：
//! `cargo test -p mcml-tex-draw --test render_all -- --nocapture`
//! （无外网时自动跳过；渲染输出在 tests/out/block 与 tests/out/items）

use std::{path::PathBuf, sync::Arc, sync::Once};

use mcml_base::{
    archives::BaseArchive,
    file_item::{FileHash, FileItemObj, LaterRun},
};
use mcml_downloader::download_item::DownloadItem;
use mcml_game::launcher_path::{libraries_path, version_path};

/// 串行锁：多个测试共用同一运行目录，避免并发下载/渲染冲突
static CHAIN_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// 测试运行目录（系统临时目录下固定目录，跨进程复用已下载的jar）
fn run_dir() -> PathBuf {
    std::env::temp_dir().join("mcml-block-render-test")
}

/// 渲染输出目录（tests 文件夹下的 out 子目录）
fn out_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests").join("out")
}

/// 初始化链（与 mcml_core::init 相同），每进程一次
fn ensure_init() {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        let dir = run_dir();
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

        // 方块/物品数据放到tests/out（block/ items/ 与langs/），方便直接查看生成的图片
        mcml_tex_draw::init(out_dir()).unwrap();
    });
}

/// 探测网络是否可用（无法访问外网时跳过用例）
async fn network_available() -> bool {
    mcml_net::get_work_client()
        .get_bytes("https://piston-meta.mojang.com/mc/game/version_manifest_v2.json")
        .await
        .is_ok()
}

/// 带计时的进度回调：每次上报打印百分比、累计耗时、距上次上报的间隔
/// （间隔突增处即变慢的位置）
struct TimedProgress {
    start: std::time::Instant,
    last: std::sync::Mutex<std::time::Instant>,
}

impl mcml_game::gui_hook::IProgressGui for TimedProgress {
    fn set_progress_text(&self, _text: Option<String>) {}

    fn set_progress_now(&self, value: usize, all: Option<usize>) {
        let now = std::time::Instant::now();
        let total = all.unwrap_or(100).max(1);
        let mut last = self.last.lock().unwrap();
        println!(
            "[渲染] {value}/{total}（{:.1}%） 累计{:.1}s 本段{:.2}s",
            value as f64 / total as f64 * 100.0,
            (now - self.start).as_secs_f64(),
            (now - *last).as_secs_f64(),
        );
        *last = now;
    }
}

struct TestDownloader;

impl mcml_downloader::IDownloadGui for TestDownloader {
    fn update(&self, thread: u32, file: &Arc<DownloadItem>) {
        let pro = file.progress() as u64;
        if pro > 0 && pro % 25 == 0 {
            println!("[下载] 线程 {thread} {} {}%", file.base.name, pro);
        }
    }

    fn update_task(&self, _state: mcml_downloader::DownloadTaskState) {}
}

/// 走下载链取得最新正式版客户端jar（本地已存在且sha1匹配时跳过下载）
async fn ensure_jar() -> BaseArchive {
    ensure_init();
    let versions = version_path::get_version_obj_online()
        .await
        .expect("版本清单拉取应成功");
    let last = versions.latest.release.clone();
    let ver = versions
        .versions
        .iter()
        .find(|v| v.id == last)
        .expect("版本清单应包含最新正式版");
    let obj = version_path::add_game(ver)
        .await
        .expect("版本JSON拉取应成功");

    let item = FileItemObj {
        name: format!("{last}.jar"),
        file: libraries_path::get_game_file(&last),
        url: mcml_net::url_helper::get_minecraft_client(&obj.downloads.client.url, &last),
        hash: FileHash::Sha1(obj.downloads.client.sha1.clone()),
        later: LaterRun::None,
    };
    if !item.check_hash() && !mcml_downloader::start_download_task(vec![item.clone()]).await {
        panic!("客户端jar下载失败");
    }
    BaseArchive::open(&item.file).expect("客户端jar应能打开")
}

/// 手动测试：渲染全部方块图标（走真实下载链）
#[tokio::test]
async fn render_all_blocks() {
    let _lock = CHAIN_LOCK.lock().await;
    ensure_init();
    if !network_available().await {
        println!("无外网，跳过");
        return;
    }

    let archive = ensure_jar().await;
    let start = std::time::Instant::now();
    mcml_tex_draw::block::render_blocks(
        &archive,
        Some(Arc::new(TimedProgress {
            start,
            last: std::sync::Mutex::new(start),
        })),
    )
    .expect("render_blocks 应成功");
    println!(
        "[渲染] 全部方块总耗时 {:.1}s",
        start.elapsed().as_secs_f64()
    );

    // 注册表：数量与固定表非Skip行数一致，分类全部写入
    let ids = mcml_tex_draw::blocks();
    let expected = mcml_tex_draw::block::icons::BLOCK_ICONS
        .iter()
        .filter(|(_, _, spec)| !matches!(spec, mcml_tex_draw::block::icons::IconSpec::Skip))
        .count();
    assert_eq!(ids.len(), expected, "注册数应等于表中非Skip条目数");

    // 平面精灵（门/木牌/花草）按新表设计不进表、留给item渲染；
    // 床是composite拼合3D模型，fence用inventory外观
    let dir = out_dir().join("block");
    for name in [
        "minecraft_white_bed.png",
        "minecraft_oak_fence.png",
        "minecraft_oak_stairs.png",
        "minecraft_furnace.png",
    ] {
        let path = dir.join(name);
        assert!(path.exists(), "应已渲染图标：{}", path.display());
    }
    println!("注册：{} 个方块图标", ids.len());
}

/// 手动测试：渲染全部物品图标（走真实下载链）
#[tokio::test]
async fn render_all_items() {
    let _lock = CHAIN_LOCK.lock().await;
    ensure_init();
    if !network_available().await {
        println!("无外网，跳过");
        return;
    }

    let archive = ensure_jar().await;
    let start = std::time::Instant::now();
    mcml_tex_draw::item::render_items(
        &archive,
        Some(Arc::new(TimedProgress {
            start,
            last: std::sync::Mutex::new(start),
        })),
    )
    .expect("render_items 应成功");
    println!(
        "[渲染] 全部物品总耗时 {:.1}s",
        start.elapsed().as_secs_f64()
    );

    // extrude平面物品（apple）、分派模型（compass）、special跳过后仍应有足量图标
    let items = mcml_tex_draw::items();
    assert!(
        items.contains(&"minecraft:apple".to_string()),
        "物品表应包含apple"
    );
    assert!(
        items.contains(&"minecraft:compass".to_string()),
        "物品表应包含compass"
    );
    assert!(items.len() > 1000, "物品数量异常：{}", items.len());
    let apple = mcml_tex_draw::get_item_path("minecraft:apple").expect("apple应已注册");
    assert!(apple.exists(), "apple图标应已写盘：{}", apple.display());
    println!("注册：{} 个物品图标", items.len());
}
