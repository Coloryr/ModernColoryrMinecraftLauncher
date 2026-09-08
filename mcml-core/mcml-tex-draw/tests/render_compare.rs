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
        // 读回上次的结果，load_blocks 才能命中版本短路，跳过重复下载/渲染
        let _ = mcml_tex_draw::load();
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

/// 获取客户端jar路径（load_blocks经libraries_path下载，与游戏库共用）
async fn get_client_jar() -> Option<PathBuf> {
    let data = mcml_net::get_work_client()
        .get_bytes("https://piston-meta.mojang.com/mc/game/version_manifest_v2.json")
        .await
        .ok()?;
    let versions: mcml_game::mojang::version_obj::VersionObj =
        mcml_base::serialize_tools::json_from_bytes(&data).ok()?;
    let last = versions.latest.release;

    let jar = mcml_game::launcher_path::libraries_path::get_game_file(&last);
    if jar.exists() { Some(jar) } else { None }
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "手动测试：下载最新客户端jar并生成APNG对比图"]
async fn render_compare() {
    let _lock = CHAIN_LOCK.lock().await;
    ensure_init();

    if !network_available().await {
        println!("无外网，跳过");
        return;
    }

    // 走完整链：下载jar → 提取贴图 → 渲染保存
    mcml_tex_draw::load_blocks()
        .await
        .expect("load_blocks 应成功");

    // 打开jar，取岩浆块的动画贴图做对比
    let jar = get_client_jar().await.expect("客户端jar应已下载");
    let archive = mcml_base::archives::BaseArchive::open(&jar).expect("jar应能打开");
    let tex_path = "assets/minecraft/textures/block/magma.png";
    let bytes = archive.read(tex_path).expect("jar内应有magma.png");

    let tex = mcml_tex_draw::block_render::decode_png(&bytes).expect("贴图应能解码");
    let frames = tex.height() / tex.width();
    println!("magma.png {}x{}，共 {frames} 帧", tex.width(), tex.height());

    // 动画配置按游戏内的mcmeta来（帧间隔 + 插值）
    let meta = mcml_tex_draw::block_render::read_anim_meta(&archive, tex_path);
    println!(
        "mcmeta frametime = {} 刻（{}ms/帧），interpolate = {}",
        meta.frametime,
        meta.frametime * 50,
        meta.interpolate
    );

    let dir = out_dir();
    std::fs::create_dir_all(&dir).unwrap();

    // APNG
    let apng = mcml_tex_draw::block_render::make_block_apng(&tex, meta).expect("APNG应能编码");
    let apng_file = dir.join("magma_apng.png");
    std::fs::write(&apng_file, &apng).unwrap();
    println!("APNG：{}（{} KB）", apng_file.display(), apng.len() / 1024);

    println!("用浏览器分别打开两个文件对比画质");
}

/// 手动测试：渲染一个静态方块图片（石头）
#[tokio::test(flavor = "multi_thread")]
#[ignore = "手动测试：下载最新客户端jar并生成静态方块图"]
async fn render_static_block() {
    let _lock = CHAIN_LOCK.lock().await;
    ensure_init();

    if !network_available().await {
        println!("无外网，跳过");
        return;
    }

    // 走完整链：下载jar → 提取贴图 → 渲染保存
    mcml_tex_draw::load_blocks()
        .await
        .expect("load_blocks 应成功");

    // 打开jar，取石头的静态贴图
    let jar = get_client_jar().await.expect("客户端jar应已下载");
    let archive = mcml_base::archives::BaseArchive::open(&jar).expect("jar应能打开");
    let bytes = archive
        .read("assets/minecraft/textures/block/stone.png")
        .expect("jar内应有stone.png");

    let tex = mcml_tex_draw::block_render::decode_png(&bytes).expect("贴图应能解码");

    // 渲染静态方块并保存
    let img = mcml_tex_draw::block_render::make_block_png(&tex).expect("静态方块应能渲染");
    #[allow(deprecated)]
    let data = img
        .encode_to_data(skia_safe::EncodedImageFormat::PNG)
        .expect("PNG应能编码");
    let dir = out_dir();
    std::fs::create_dir_all(&dir).unwrap();
    let out_file = dir.join("stone.png");
    std::fs::write(&out_file, data.as_bytes()).unwrap();
    println!("静态方块：{}（{} KB）", out_file.display(), data.len() / 1024);
}

/// 手动测试：渲染楼梯（走全量渲染后从输出取，对比wiki图标）
#[tokio::test(flavor = "multi_thread")]
#[ignore = "手动测试：渲染楼梯并复制到tests/out"]
async fn render_stairs_sample() {
    let _lock = CHAIN_LOCK.lock().await;
    ensure_init();

    if !network_available().await {
        println!("无外网，跳过");
        return;
    }

    mcml_tex_draw::load_blocks()
        .await
        .expect("load_blocks 应成功");

    // 从渲染输出里复制楼梯
    let src = out_dir().join("block").join("minecraft_oak_stairs.png");
    assert!(src.exists(), "应已渲染 minecraft_oak_stairs.png：{}", src.display());
    let dst = out_dir().join("oak_stairs.png");
    std::fs::copy(&src, &dst).unwrap();
    println!("楼梯：{}", dst.display());
}
