//! 资源包读取测试。
//!
//! 资源包样本不提交进 git：
//! - 用 `zip::ZipWriter` 程序化生成一个最小有效样本做精确断言（离线）；
//! - 通过 Modrinth API 解析多个常用资源包的最新版本并下载（网络不可用时跳过）。

mod common;

use std::io::Write;
use std::path::PathBuf;

use mcml_game::game_resourcepacks::{ResourcepackObj, process_resourcepack};
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

/// 1×1 透明 PNG（最小有效文件）。
const PACK_PNG: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4,
    0x89, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0x00, 0x01, 0x00, 0x00,
    0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE,
    0x42, 0x60, 0x82,
];

/// 生成一个最小资源包 zip：pack.mcmeta + pack.png。
fn make_mini_resourcepack() -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "mcml-resourcepack-mini-{}.zip",
        uuid::Uuid::new_v4()
    ));
    let file = std::fs::File::create(&path).unwrap();
    let mut writer = ZipWriter::new(file);
    let options = SimpleFileOptions::default();

    let meta = br#"{"pack":{"pack_format":15,"min_format":5,"max_format":30,"description":"mcml test pack"}}"#;
    writer.start_file("pack.mcmeta", options).unwrap();
    writer.write_all(meta).unwrap();
    writer.start_file("pack.png", options).unwrap();
    writer.write_all(PACK_PNG).unwrap();
    writer.finish().unwrap();

    path
}

/// 程序化生成的最小样本：精确断言描述、版本区间与图标。
#[test]
fn read_generated_resourcepack() {
    let path = make_mini_resourcepack();
    let obj: ResourcepackObj = process_resourcepack(&path).expect("应能解析生成的资源包");

    assert!(!obj.fail, "生成的样本应解析成功");
    assert_eq!(obj.description, "mcml test pack");
    assert_eq!(obj.pack_format, 15);
    assert_eq!(obj.min_format, 5);
    assert_eq!(obj.max_format, 30);
    assert_eq!(obj.icon.as_deref(), Some(PACK_PNG.as_ref()));
}

/// 从 Modrinth 下载多个常用资源包并解析。
/// 网络不可用时整体跳过；单个样本下载失败跳过，其余继续。
#[test]
fn read_real_resourcepacks() {
    let mut downloaded = 0;
    let mut parsed = 0;

    for slug in common::RESOURCEPACK_SLUGS {
        let Some(path) = common::download_latest(slug) else {
            eprintln!("[跳过] 资源包 {slug} 下载失败（网络不可用？）");
            continue;
        };
        downloaded += 1;

        match process_resourcepack(&path) {
            Ok(obj) => {
                // 下载成功但 pack.mcmeta 解析失败时 fail 置 true，属真实缺陷
                assert!(!obj.fail, "资源包 {slug} 的 pack.mcmeta 解析失败");
                // 新格式资源包（1.20.5+）不再提供 pack_format，只用 min/max_format
                assert!(
                    obj.pack_format > 0 || obj.min_format > 0 || obj.max_format > 0,
                    "资源包 {slug} 未提供任何格式版本字段"
                );
                parsed += 1;
                println!(
                    "{slug}: pack_format = {}, min_format = {}, max_format = {}, description = {}",
                    obj.pack_format, obj.min_format, obj.max_format, obj.description
                );
            }
            Err(err) => {
                eprintln!("[警告] 资源包 {slug} 读取失败：{err}");
            }
        }
    }

    if downloaded == 0 {
        eprintln!("全部资源包样本均无法下载（需要网络），测试跳过");
        return;
    }
    assert!(parsed > 0, "下载成功但未能解析任何资源包");
}
