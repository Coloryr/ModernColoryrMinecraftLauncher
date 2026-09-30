//! 复现生产环境的真实形态：四个请求同时卡在"读 body"，中断从**同步线程**发出
//!
//! 与之前几个测试的关键差别（也是之前测试没能复现问题的原因）：
//! - 之前：请求与 abort 都在**同一个线程里的同一个 runtime**，且都是"卡在头部请求"；
//! - 真实：请求跑在 Tauri runtime（多线程），abort 由**同步命令**在另一个线程调用，
//!   四个请求**同时**处于 `resp.body()` 读取阶段。
//!
//! 本测试用 4 个独立线程 + 4 个独立 current_thread runtime 模拟这种分布，
//! 让 4 个请求同时卡住，然后从一个**没有 runtime 上下文的线程**调用 `abort_all()`。

use std::io::Read;
use std::net::TcpListener;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use mml_config::config_obj::ProxyState;

/// "只收不回"的服务：响应头都不回，制造卡死
fn silent_server() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").expect("绑定失败");
    let port = listener.local_addr().unwrap().port();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            std::thread::spawn(move || {
                let mut buf = [0u8; 4096];
                let _ = stream.read(&mut buf);
                std::thread::sleep(Duration::from_secs(60));
            });
        }
    });
    port
}

fn init_once() {
    use std::sync::Once;
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        let exe = std::env::current_exe().unwrap();
        let run_dir = exe.parent().unwrap().parent().unwrap().to_path_buf();
        let _ = mml_log::start(&run_dir);
        let _ = mml_config::init(&run_dir);
        mml_net::init();
    });
}

#[test]
fn four_concurrent_stuck_requests_all_abort() {
    init_once();
    let port = silent_server();

    // 四个请求：各自独立线程 + 独立 current_thread runtime（模拟真实分布）
    let mut receivers = Vec::new();
    for i in 0..4 {
        let (tx, rx) = mpsc::channel();
        receivers.push(rx);
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(async move {
                let started = Instant::now();
                let client = mml_net::Client::new(ProxyState::Auto);
                let url = format!("http://127.0.0.1:{port}/stuck{i}");
                let res = client.get_text(&url).await;
                let aborted = match &res {
                    Ok(_) => false,
                    Err(e) => mml_net::is_aborted(e),
                };
                let _ = tx.send((started.elapsed(), aborted));
            });
        });
    }

    // 让四个请求都进入卡住状态
    std::thread::sleep(Duration::from_millis(600));

    // 从中断发起方：一个**没有 runtime 上下文**的普通线程（对应 Tauri 同步命令）
    let abort_at = Instant::now();
    std::thread::spawn(move || {
        mml_net::abort_all();
    })
    .join()
    .unwrap();

    let mut all_ok = true;
    for (i, rx) in receivers.iter().enumerate() {
        match rx.recv_timeout(Duration::from_secs(5)) {
            Ok((elapsed, aborted)) => {
                println!("请求 {i}：总耗时 {elapsed:?}，aborted={aborted}");
                if !aborted {
                    all_ok = false;
                }
            }
            Err(_) => {
                println!("请求 {i}：中断后仍未返回 ✗");
                all_ok = false;
            }
        }
    }
    println!("中断后统一耗时：{:?}", abort_at.elapsed());
    assert!(all_ok, "四个卡住的请求应全部被中断返回");
}
