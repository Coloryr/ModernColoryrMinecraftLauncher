//! LauncherPack（导入其他启动器压缩包）的检测逻辑测试。
//!
//! 覆盖 `.minecraft` 定位、版本隔离判定、版本扫描与主版本选择。
//! 这些测试不依赖全局状态（实例列表、路径缓存等），可以独立运行。
//! 压缩包为程序化生成（二进制样本不提交进 git）。

use std::io::Write;
use std::path::PathBuf;

use mcml_base::archives::BaseArchive;
use mcml_game::add_game::{
    find_minecraft_prefix, pick_primary, scan_versions, version_folder_has_data,
};
use uuid::Uuid;
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

/// 程序化生成一个 zip 压缩包
fn make_zip(files: &[(&str, &[u8])]) -> PathBuf {
    let zip_path = std::env::temp_dir().join(format!("mcml-launcherpack-{}.zip", Uuid::new_v4()));
    let file = std::fs::File::create(&zip_path).expect("创建测试压缩包失败");
    let mut writer = ZipWriter::new(file);
    let options = SimpleFileOptions::default();
    for (name, content) in files {
        writer.start_file(*name, options).unwrap();
        writer.write_all(content).unwrap();
    }
    writer.finish().unwrap();
    zip_path
}

/// 生成一个最小可解析的官方版本 json
fn version_json(id: &str) -> Vec<u8> {
    format!(r#"{{"id":"{id}"}}"#).into_bytes()
}

#[test]
fn find_minecraft_prefix_finds_root() {
    let zip = make_zip(&[
        (".minecraft/mods/a.jar", b"a"),
        (".minecraft/options.txt", b"x"),
    ]);
    let archive = BaseArchive::open(&zip).unwrap();
    assert_eq!(
        find_minecraft_prefix(archive.entries()).as_deref(),
        Some(".minecraft/")
    );
}

#[test]
fn find_minecraft_prefix_finds_wrapped() {
    let zip = make_zip(&[("mygame/.minecraft/mods/a.jar", b"a")]);
    let archive = BaseArchive::open(&zip).unwrap();
    assert_eq!(
        find_minecraft_prefix(archive.entries()).as_deref(),
        Some("mygame/.minecraft/")
    );
}

#[test]
fn find_minecraft_prefix_none() {
    let zip = make_zip(&[("some/other/file.txt", b"a")]);
    let archive = BaseArchive::open(&zip).unwrap();
    assert!(find_minecraft_prefix(archive.entries()).is_none());
}

#[test]
fn version_folder_has_data_detects_isolation() {
    // 版本文件夹内除 json/jar 外还有游戏资源 → 视为隔离
    let json = version_json("1.21.1");
    let zip = make_zip(&[
        (".minecraft/versions/1.21.1/1.21.1.json", json.as_slice()),
        (".minecraft/versions/1.21.1/1.21.1.jar", b"jar"),
        (".minecraft/versions/1.21.1/config/example.toml", b"c"),
    ]);
    let archive = BaseArchive::open(&zip).unwrap();
    assert!(version_folder_has_data(&archive, ".minecraft/", "1.21.1"));

    // 只有 json/jar → 常规布局，未隔离
    let json = version_json("1.21.1");
    let zip = make_zip(&[
        (".minecraft/versions/1.21.1/1.21.1.json", json.as_slice()),
        (".minecraft/versions/1.21.1/1.21.1.jar", b"jar"),
    ]);
    let archive = BaseArchive::open(&zip).unwrap();
    assert!(!version_folder_has_data(&archive, ".minecraft/", "1.21.1"));
}

#[test]
fn scan_versions_detects_isolation_and_name() {
    let root = version_json("1.20.1");
    let isolated = version_json("1.21.1");
    let zip = make_zip(&[
        (".minecraft/1.20.1.json", root.as_slice()),
        (
            ".minecraft/versions/1.21.1/1.21.1.json",
            isolated.as_slice(),
        ),
    ]);
    let archive = BaseArchive::open(&zip).unwrap();
    let versions = scan_versions(&archive, ".minecraft/");
    assert_eq!(versions.len(), 2);

    let non_isolated = versions
        .iter()
        .find(|version| version.version_name == "1.20.1")
        .unwrap();
    assert!(!non_isolated.isolated);
    assert_eq!(non_isolated.obj.id, "1.20.1");

    let isolated = versions
        .iter()
        .find(|version| version.version_name == "1.21.1")
        .unwrap();
    assert!(isolated.isolated);
    assert_eq!(isolated.obj.id, "1.21.1");
}

#[test]
fn pick_primary_prefers_launcher_profiles() {
    let v1 = version_json("1.20.1");
    let v2 = version_json("1.21.1");
    let zip = make_zip(&[
        (".minecraft/1.20.1.json", v1.as_slice()),
        (".minecraft/1.21.1.json", v2.as_slice()),
        (
            ".minecraft/launcher_profiles.json",
            br#"{"profiles":{"a":{"lastVersionId":"1.21.1"}}}"#,
        ),
    ]);
    let archive = BaseArchive::open(&zip).unwrap();
    let versions = scan_versions(&archive, ".minecraft/");
    let index = pick_primary(&archive, ".minecraft/", &versions).unwrap();
    assert_eq!(versions[index].obj.id, "1.21.1");
}

#[test]
fn pick_primary_prefers_isolated_with_data() {
    let v1 = version_json("1.20.1");
    let v2 = version_json("1.21.1");
    let zip = make_zip(&[
        (".minecraft/1.20.1.json", v1.as_slice()),
        (".minecraft/versions/1.21.1/1.21.1.json", v2.as_slice()),
        (".minecraft/versions/1.21.1/config/example.toml", b"c"),
    ]);
    let archive = BaseArchive::open(&zip).unwrap();
    let versions = scan_versions(&archive, ".minecraft/");
    let index = pick_primary(&archive, ".minecraft/", &versions).unwrap();
    assert_eq!(versions[index].obj.id, "1.21.1");
}

#[test]
fn pick_primary_single_version() {
    let v1 = version_json("1.20.1");
    let zip = make_zip(&[(".minecraft/1.20.1.json", v1.as_slice())]);
    let archive = BaseArchive::open(&zip).unwrap();
    let versions = scan_versions(&archive, ".minecraft/");
    assert_eq!(pick_primary(&archive, ".minecraft/", &versions), Some(0));
}
