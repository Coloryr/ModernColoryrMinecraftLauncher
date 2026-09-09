//! 手动对比测试：走真实下载链渲染方块动画贴图
//!
//! 1. 从版本清单取最新正式版，下载客户端 jar 并解包提取方块贴图（`load_blocks`）
//! 2. 用其中的动画贴图（magma.png，岩浆块）逐帧渲染立方体，合成 APNG 输出
//!
//! 运行（需外网，首次会下载约 25MB 的客户端 jar）：
//! `cargo test -p mcml-tex-draw --test render_compare -- --ignored --nocapture`

use std::{path::PathBuf, sync::Arc, sync::Once};

use mcml_downloader::download_item::DownloadItem;

/// 串行锁：多个测试共用同一运行目录，避免并发下载/渲染冲突
static CHAIN_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// 测试运行目录（系统临时目录下固定目录，跨进程复用已下载的jar）
fn run_dir() -> PathBuf {
    std::env::temp_dir().join("mcml-block-render-test")
}

/// 对比文件输出目录（tests 文件夹下的 out 子目录）
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

        // 可选测试代理：设置 MCML_TEST_PROXY=ip:port 时走显式代理
        if let Ok(proxy) = std::env::var("MCML_TEST_PROXY") {
            let (ip, port) = proxy
                .split_once(':')
                .expect("MCML_TEST_PROXY 格式应为 ip:port");
            use mcml_config::config_obj::ProxyState;
            {
                let mut config = mcml_config::write_config();
                config.http.work_proxy = ProxyState::User;
                config.http.login_proxy = ProxyState::User;
                config.http.proxy_ip = ip.to_string();
                config.http.proxy_port = port.parse().expect("MCML_TEST_PROXY 端口不合法");
            }
            println!("[代理] 走显式代理 {proxy}");
        }

        mcml_downloader::init(&dir).unwrap();
        mcml_downloader::set_gui_handel(Box::new(TestDownloader));
        mcml_downloader::start();

        // 方块数据放到tests/out（block/与langs/），方便直接查看生成的图片
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

/// 手动测试：渲染楼梯（走全量渲染后从输出取，对比wiki图标）
#[tokio::test]
async fn render_stairs_sample() {
    let _lock = CHAIN_LOCK.lock().await;
    ensure_init();

    if !network_available().await {
        println!("无外网，跳过");
        return;
    }

    let start = std::time::Instant::now();
    mcml_tex_draw::load_blocks(Some(std::sync::Arc::new(TimedProgress {
        start,
        last: std::sync::Mutex::new(start),
    })))
    .await
    .expect("load_blocks 应成功");
    println!(
        "[渲染] load_blocks 总耗时 {:.1}s",
        start.elapsed().as_secs_f64()
    );

    // 从渲染输出里复制楼梯
    let src = out_dir().join("block").join("minecraft_oak_stairs.png");
    assert!(
        src.exists(),
        "应已渲染 minecraft_oak_stairs.png：{}",
        src.display()
    );
    let dst = out_dir().join("oak_stairs.png");
    std::fs::copy(&src, &dst).unwrap();
    println!("楼梯：{}", dst.display());
}

/// 手动测试：验证创造栏图标表渲染（精灵/组合模型/分类写入）
#[tokio::test]
async fn render_merged_samples() {
    let _lock = CHAIN_LOCK.lock().await;
    ensure_init();

    if !network_available().await {
        println!("无外网，跳过");
        return;
    }

    mcml_tex_draw::load_blocks(None)
        .await
        .expect("load_blocks 应成功");

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
        println!("图标：{}", path.display());
    }

    // 注册表：数量与固定表非Skip行数一致，分类全部写入
    let ids = mcml_tex_draw::blocks();
    let expected = mcml_tex_draw::block::icons::BLOCK_ICONS
        .iter()
        .filter(|(_, _, spec)| !matches!(spec, mcml_tex_draw::block::icons::IconSpec::Skip))
        .count();
    assert_eq!(ids.len(), expected, "注册数应等于表中非Skip条目数");
    println!("注册：{} 个方块图标", ids.len());
}

