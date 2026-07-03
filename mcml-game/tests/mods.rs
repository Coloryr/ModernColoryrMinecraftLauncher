use mcml_base::path_helper;
use mcml_game::class_scan;
use toml::Table;

#[tokio::test]
async fn get_mod_info() {}

#[test]
fn test_scan_file() {
    let result = class_scan::scan_jar("./tests/mods/[forge-1.12.2]AllMusic_Client-3.7.4.jar");
    if result.is_err() {
        let err = result.as_ref().err().unwrap();
        println!("{}", err);
    }
    assert!(!result.is_err());

    let res = result.unwrap();
    println!("找到 {} 个 Mod:", res.mods.len());
    for m in &res.mods {
        println!("  modid: {}, side: {:?}", m.modid, m.side);
    }
    assert!(!res.mods.is_empty(), "应该至少找到一个 Mod");

    let result = class_scan::scan_jar("./tests/mods/[forge-1.12.2]AllMusic_Server-4.0.3.jar");
    if result.is_err() {
        let err = result.as_ref().err().unwrap();
        println!("{}", err);
    }
    assert!(!result.is_err());

    let res = result.unwrap();
    println!("找到 {} 个 Mod:", res.mods.len());
    for m in &res.mods {
        println!("  modid: {}, side: {:?}", m.modid, m.side);
    }
    assert!(!res.mods.is_empty(), "应该至少找到一个 Mod");
}

#[test]
fn toml() {
    let file = path_helper::read_text("./tests/tomls/test.toml").unwrap();

    let datas = file.parse::<Table>().unwrap();
    println!("{}", datas);
}   