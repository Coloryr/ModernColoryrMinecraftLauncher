use std::{
    sync::atomic::{AtomicBool, Ordering},
    thread,
    time::Duration,
};

use mcml_game::game_lan::{GameLan, GameMotd};

static HAVE: AtomicBool = AtomicBool::new(false);

fn read_motd(motd: &GameMotd) {
    println!(
        "motd:{} port:{} addr:{}",
        motd.motd,
        motd.port,
        motd.addr.unwrap().to_string()
    );

    HAVE.store(true, Ordering::Release);
}

#[test]
fn test_lan() {
    let client = GameLan::new_client().unwrap();
    let server = GameLan::new_server().unwrap();

    client.add_event_handler(read_motd);
    client.start_read().unwrap();

    server
        .start_send(GameMotd {
            motd: "测试".to_string(),
            port: "456".to_string(),
            addr: None,
        })
        .unwrap();

    thread::sleep(Duration::from_secs(6));

    client.stop();
    server.stop();

    assert!(HAVE.load(Ordering::Acquire));
}
