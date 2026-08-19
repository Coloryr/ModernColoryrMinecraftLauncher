//! 整合包格式测试：下载真实的 mrpack 并验证字段与下载项构建。
//!
//! 样本不提交进 git，测试运行时通过 Modrinth API 解析并下载
//! Fabulously Optimized 14.0.0-beta.3（断言依赖该版本的具体字段）。
//! 网络不可用时跳过（打印提示，不 fail）。

mod common;

use std::io::Write;
use std::sync::{Arc, RwLock};

use mcml_base::archives::BaseArchive;
use mcml_game::{GameInstance, modrinth};
use mcml_game::launcher::instance_setting_obj::InstanceSettingObj;
use mcml_game::modpack::BaseModPackWorker;
use mcml_game::modrinth::pack_obj::{ModrinthPackFileObj, ModrinthPackObj};
use mcml_names::names;
use tokio_util::sync::CancellationToken;
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

/// mrpack 内的整合包清单文件名
const MODRINTH_INDEX: &str = "modrinth.index.json";

/// 打开测试整合包（mrpack 本质是 zip，但 `BaseArchive` 按后缀识别格式，
/// GUI 侧导入前会改名 .zip，测试时复制为唯一命名的 .zip 再打开）。
fn open_mrpack() -> Option<BaseArchive> {
    let mrpack = common::download_mrpack()?;
    let zip_path =
        std::env::temp_dir().join(format!("mcml-modpack-test-{}.zip", uuid::Uuid::new_v4()));
    std::fs::copy(mrpack, &zip_path).ok()?;
    BaseArchive::open(&zip_path).ok()
}

fn read_pack_obj() -> Option<ModrinthPackObj> {
    let archive = open_mrpack()?;
    let entry = archive
        .entries()
        .iter()
        .find(|item| item.name == MODRINTH_INDEX)?;
    let data = archive.read(&entry.name).ok()?;
    mcml_base::serialize_tools::json_from_bytes(&data).ok()
}

/// 解析真实 mrpack 的 `modrinth.index.json`。
#[test]
fn parse_real_mrpack_index() {
    let Some(obj) = read_pack_obj() else {
        eprintln!("跳过：无法下载测试整合包（需要网络）");
        return;
    };

    assert_eq!(obj.format_version, 1);
    assert_eq!(obj.version_id, "14.0.0-beta.3");
    assert_eq!(obj.name, "Fabulously Optimized");
    // 真实 mrpack 没有 summary 字段，缺失时使用默认空字符串
    assert!(obj.summary.is_empty());

    // 依赖：fabric-loader + minecraft
    assert_eq!(
        obj.dependencies.get("minecraft").map(String::as_str),
        Some("26.2")
    );
    assert_eq!(
        obj.dependencies.get("fabric-loader").map(String::as_str),
        Some("0.19.3")
    );

    // 48 个模组文件
    assert_eq!(obj.files.len(), 48);
}

/// 每个文件条目都应带 SHA1 校验和以及至少一个下载地址。
#[test]
fn mrpack_files_have_hash_and_download() {
    let Some(obj) = read_pack_obj() else {
        eprintln!("跳过：无法下载测试整合包（需要网络）");
        return;
    };

    for file in &obj.files {
        assert_eq!(file.hashes.sha1.len(), 40, "路径 {} 缺少 SHA1", file.path);
        assert_eq!(
            file.hashes.sha512.len(),
            128,
            "路径 {} 缺少 SHA512",
            file.path
        );
        assert!(
            !file.downloads.is_empty(),
            "路径 {} 缺少下载地址",
            file.path
        );
        assert!(file.file_size > 0, "路径 {} 的文件大小应为正数", file.path);
    }
}

/// 压缩包内条目：`modrinth.index.json` 应存在于根目录，其余为 `overrides/` 下的文件。
#[test]
fn mrpack_entries_layout() {
    let Some(archive) = open_mrpack() else {
        eprintln!("跳过：无法下载测试整合包（需要网络）");
        return;
    };
    let names: Vec<&str> = archive.entries().iter().map(|e| e.name.as_str()).collect();

    assert!(
        names.contains(&MODRINTH_INDEX),
        "应包含 modrinth.index.json"
    );
    // 除 index.json 外全部在 overrides/ 下
    let others: Vec<&&str> = names.iter().filter(|n| **n != MODRINTH_INDEX).collect();
    assert!(!others.is_empty());
    for name in others {
        assert!(
            name.starts_with("overrides/"),
            "非 index.json 条目 {} 应位于 overrides/ 下",
            name
        );
    }
}

/// 用真实文件构建下载项：目标路径为游戏目录 + 包内路径，哈希为 SHA1+SHA512。
#[test]
fn build_download_from_real_file() {
    let Some(obj) = read_pack_obj() else {
        eprintln!("跳过：无法下载测试整合包（需要网络）");
        return;
    };
    let file: &ModrinthPackFileObj = &obj.files[0];

    let item = modrinth::make_pack_download_obj(file, "game");
    assert_eq!(item.url, file.downloads[0]);
    assert_eq!(item.name, file.path);
    assert_eq!(item.file, std::path::PathBuf::from("game").join(&file.path));
}

/// 生成一个最小的 mrpack（zip）：根目录的 `modrinth.index.json` +
/// `overrides/` 下的覆盖文件。
///
/// 二进制样本不提交进 git；这里直接程序化生成，无需网络。
fn make_mini_mrpack() -> std::path::PathBuf {
    let zip_path =
        std::env::temp_dir().join(format!("mcml-modpack-mini-{}.zip", uuid::Uuid::new_v4()));
    let file = std::fs::File::create(&zip_path).expect("创建测试 mrpack 失败");
    let mut writer = ZipWriter::new(file);
    let options = SimpleFileOptions::default();

    let index =
        br#"{"formatVersion":1,"game":"minecraft","versionId":"1","name":"mini","files":[]}"#;
    writer.start_file(names::MODRINTH_FILE, options).unwrap();
    writer.write_all(index).unwrap();
    writer
        .start_file(
            &format!("{}/config/example.txt", names::OVERRIDE_DIR),
            options,
        )
        .unwrap();
    writer.write_all(b"hello").unwrap();
    writer
        .start_file(
            &format!(
                "{}/config/modpack_defaults/config/bettergrass.json",
                names::OVERRIDE_DIR
            ),
            options,
        )
        .unwrap();
    writer.write_all(b"{}").unwrap();
    writer.finish().unwrap();

    zip_path
}

/// 验证 `extract_pack_files` 的路径路由：
/// - `overrides/` 前缀去除后写入游戏根目录（.minecraft）
/// - 其余文件（如 `modrinth.index.json`）直接写入游戏基础目录
/// - `unselect` 指定的条目被跳过
#[test]
fn extract_pack_files_routing() {
    let temp = std::env::temp_dir().join(format!("mcml-modpack-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp);
    mcml_game::init(&temp).expect("初始化运行路径失败");

    let instance = InstanceSettingObj {
        name: "routing-test".to_string(),
        dir: "routing-test".to_string(),
        version: "26.2".to_string(),
        ..Default::default()
    };
    let base_path = instance.get_base_path();
    let game_path = instance.get_game_path();
    let game: GameInstance = Arc::new(RwLock::new(instance));

    let archive = BaseArchive::open(&make_mini_mrpack()).expect("打开测试 mrpack 失败");
    let mut worker = BaseModPackWorker::new(archive, None, None, None, CancellationToken::new());
    worker.game = Some(game.clone());

    // 第一次解压：跳过 modrinth.index.json
    worker
        .extract_pack_files(
            names::OVERRIDE_DIR,
            Some(vec![names::MODRINTH_FILE.to_string()]),
        )
        .expect("解压失败");

    // 被跳过的根文件不写入基础目录
    assert!(
        !base_path.join(names::MODRINTH_FILE).exists(),
        "unselect 的 modrinth.index.json 不应被解压"
    );
    // overrides/ 文件去掉前缀写入游戏目录
    let override_file = game_path.join("config/modpack_defaults/config/bettergrass.json");
    assert!(override_file.exists(), "overrides 文件应解压到游戏目录");

    // 第二次全量解压：根文件写入基础目录
    worker
        .extract_pack_files(names::OVERRIDE_DIR, None)
        .expect("全量解压失败");
    assert!(
        base_path.join(names::MODRINTH_FILE).exists(),
        "modrinth.index.json 应写入基础目录"
    );

    let _ = std::fs::remove_dir_all(&temp);
}
