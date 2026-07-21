use std::{collections::HashMap, fs, path::{Path, PathBuf}};

use mcml_base::archives::{ArchiveType, TarMode, compress, decompress};

/// 对比两个文件夹是否一致
pub fn compare_folders(
    dir1: &PathBuf,
    dir2: &PathBuf,
    skip_patterns: &[String],
) -> bool {
    let mut differences = Vec::new();
    
    // 收集第一个文件夹的所有文件
    let files1 = collect_files_with_filter(dir1, dir1, skip_patterns);
    let files2 = collect_files_with_filter(dir2, dir2, skip_patterns);
    
    // 检查文件数量
    if files1.len() != files2.len() {
        differences.push(format!(
            "文件数量不一致: {} vs {}",
            files1.len(),
            files2.len()
        ));
    }
    
    // 检查缺失的文件
    for (rel_path, _) in &files1 {
        if !files2.contains_key(rel_path) {
            differences.push(format!("文件缺失: {} 在第二个文件夹中不存在", rel_path));
        }
    }
    
    for (rel_path, _) in &files2 {
        if !files1.contains_key(rel_path) {
            differences.push(format!("多余文件: {} 在第一个文件夹中不存在", rel_path));
        }
    }
    
    // 对比共同文件的内容
    for (rel_path, content1) in &files1 {
        if let Some(content2) = files2.get(rel_path) {
            if content1 != content2 {
                differences.push(format!("文件内容不一致: {}", rel_path));
            }
        }
    }
    
    differences.is_empty()
}

/// 收集文件夹中所有文件的内容（跳过符合模式的文件）
fn collect_files_with_filter(
    current_dir: &Path,
    root_dir: &Path,
    skip_patterns: &[String],
) -> HashMap<String, Vec<u8>> {
    let mut files = HashMap::new();
    collect_files_recursive(current_dir, root_dir, skip_patterns, &mut files);
    files
}

/// 递归收集文件
fn collect_files_recursive(
    current_dir: &Path,
    root_dir: &Path,
    skip_patterns: &[String],
    files: &mut HashMap<String, Vec<u8>>,
) {
    if let Ok(entries) = fs::read_dir(current_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            
            if path.is_file() {
                // 计算相对路径
                let rel_path = path.strip_prefix(root_dir).unwrap_or(&path);
                let rel_path_str = normalize_path(rel_path);
                
                // 检查是否需要跳过
                if should_skip(&rel_path_str, skip_patterns) {
                    println!("  跳过对比: {}", rel_path_str);
                    continue;
                }
                
                // 读取文件内容
                if let Ok(content) = fs::read(&path) {
                    files.insert(rel_path_str, content);
                } else {
                    eprintln!("无法读取文件: {}", path.display());
                }
            } else if path.is_dir() {
                // 递归处理子目录
                collect_files_recursive(&path, root_dir, skip_patterns, files);
            }
        }
    }
}

/// 统一路径分隔符
fn normalize_path(path: &Path) -> String {
    path.to_string_lossy()
        .to_string()
        .replace('\\', "/")
}

/// 检查是否应该跳过该文件
fn should_skip(file_path: &str, patterns: &[String]) -> bool {
    patterns.iter().any(|pattern| {
        let pattern_normalized = pattern.replace('\\', "/");
        
        // 支持简单的通配符
        if pattern_normalized.contains('*') {
            matches_wildcard(file_path, &pattern_normalized)
        } else {
            // 精确匹配或包含匹配
            file_path == pattern_normalized || 
            file_path.contains(&pattern_normalized) ||
            // 检查文件名是否匹配
            Path::new(file_path).file_name()
                .map(|name| name.to_string_lossy().to_string() == pattern_normalized)
                .unwrap_or(false)
        }
    })
}

/// 简单的通配符匹配
fn matches_wildcard(text: &str, pattern: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    
    let text_parts: Vec<&str> = text.split('/').collect();
    let pattern_parts: Vec<&str> = pattern.split('/').collect();
    
    // 简单的模式匹配
    if pattern_parts.len() != text_parts.len() && !pattern.contains('*') {
        return false;
    }
    
    pattern_parts.iter().enumerate().all(|(i, p)| {
        if i >= text_parts.len() {
            return false;
        }
        if *p == "*" {
            true
        } else if p.contains('*') {
            let prefix = p.trim_end_matches('*');
            text_parts[i].starts_with(prefix)
        } else {
            text_parts[i] == *p
        }
    })
}

#[test]
fn zip() {
    let file = Path::new("test_run");
    let source = Path::new("tests").join("test_compress/");
    let zip = file.join("test.zip");
    let filter = Some(vec![
        String::from("skip.text"),
        String::from("dir1/skip.text"),
    ]);
    let res = compress(ArchiveType::Zip, &zip, &source, Some(&source), &filter, None);
    assert!(res.is_ok());
}

#[test]
#[ignore]
fn unzip() {
    let file = Path::new("test_run");
    let zip = file.join("test.zip");
    let dir = file.join("test_zip/");
    let res = decompress(ArchiveType::Zip, &zip, &dir, None);
    assert!(res.is_ok());
}

#[test]
#[ignore]
fn zip_equal() {
    let file = Path::new("test_run");
    let source = Path::new("tests").join("test_compress/");
    let dir1 = file.join("test_zip/");
    assert!(compare_folders(&source, &dir1, &[
        String::from("skip.text"),
        String::from("dir1/skip.text"),
    ]));
}

#[test]
#[ignore]
fn r7z() {
    let file = Path::new("test_run");
    let source = Path::new("tests").join("test_compress/");
    let zip = file.join("test.7z");
    let filter = Some(vec![
        String::from("skip.text"),
        String::from("dir1/skip.text"),
    ]);
    let res = compress(ArchiveType::R7Z, &zip, &source, Some(&source), &filter, None);
    assert!(res.is_ok());
}

#[test]
#[ignore]
fn unr7z() {
    let file = Path::new("test_run");
    let zip = file.join("test.7z");
    let dir = file.join("test_7z/");
    let res = decompress(ArchiveType::R7Z, &zip, &dir, None);
    assert!(res.is_ok());
}

#[test]
#[ignore]
fn r7z_equal() {
    let file = Path::new("test_run");
    let source = Path::new("tests").join("test_compress/");
    let dir1 = file.join("test_7z/");
    assert!(compare_folders(&source, &dir1, &[
        String::from("skip.text"),
        String::from("dir1/skip.text"),
    ]));
}

#[test]
#[ignore]
fn targz() {
    let file = Path::new("test_run");
    let source = Path::new("tests").join("test_compress/");
    let zip = file.join("test.tar.gz");
    let filter = Some(vec![
        String::from("skip.text"),
        String::from("dir1/skip.text"),
    ]);
    let res = compress(ArchiveType::TarGz, &zip, &source, Some(&source), &filter, None);
    assert!(res.is_ok());
}

#[test]
#[ignore]
fn untargz() {
    let file = Path::new("test_run");
    let zip = file.join("test.tar.gz");
    let dir = file.join("test_targz/");
    let res = decompress(ArchiveType::TarGz, &zip, &dir, None);
    assert!(res.is_ok());
}

#[test]
#[ignore]
fn targz_equal() {
    let file = Path::new("test_run");
    let source = Path::new("tests").join("test_compress/");
    let dir1 = file.join("test_targz/");
    assert!(compare_folders(&source, &dir1, &[
        String::from("skip.text"),
        String::from("dir1/skip.text"),
    ]));
}

#[test]
#[ignore]
fn tarxz() {
   let file = Path::new("test_run");
    let source = Path::new("tests").join("test_compress/");
    let zip = file.join("test.tar.xz");
    let filter = Some(vec![
        String::from("skip.text"),
        String::from("dir1/skip.text"),
    ]);
    let res = compress(ArchiveType::TarXz, &zip, &source, Some(&source), &filter, None);
    assert!(res.is_ok());
}

#[test]
#[ignore]
fn untarxz() {
    let file = Path::new("test_run");
    let zip = file.join("test.tar.xz");
    let dir = file.join("test_tarxz/");
    let res = decompress(ArchiveType::TarXz, &zip, &dir, None);
    assert!(res.is_ok());
}

#[test]
#[ignore]
fn tarxz_equal() {
    let file = Path::new("test_run");
    let source = Path::new("tests").join("test_compress/");
    let dir1 = file.join("test_tarxz/");
    assert!(compare_folders(&source, &dir1, &[
        String::from("skip.text"),
        String::from("dir1/skip.text"),
    ]));
}

#[test]
fn test_tar_mode_from_path() {
    // 测试 .tar.gz
    let path_gz = &Path::new("archive.tar.gz").to_path_buf();
    assert_eq!(TarMode::try_from_path(path_gz), Some(TarMode::Gz));
    
    // 测试 .tgz
    let path_tgz = &Path::new("archive.tgz").to_path_buf();
    assert_eq!(TarMode::try_from_path(path_tgz), Some(TarMode::Gz));
    
    // 测试 .tar.xz
    let path_xz = &Path::new("archive.tar.xz").to_path_buf();
    assert_eq!(TarMode::try_from_path(path_xz), Some(TarMode::Xz));
    
    // 测试 .txz
    let path_txz = &Path::new("archive.txz").to_path_buf();
    assert_eq!(TarMode::try_from_path(path_txz), Some(TarMode::Xz));
    
    // 测试不支持的类型
    let path_zip = &Path::new("archive.zip").to_path_buf();
    assert!(TarMode::try_from_path(path_zip).is_none());
}

#[test]
#[ignore]
fn remove_file() {
    let file = Path::new("test_run");
    fs::remove_dir_all(file).ok();
}

#[test]
fn archive_test() {
    zip();
    r7z();
    targz();
    tarxz();

    unzip();
    unr7z();
    untargz();
    untarxz();

    zip_equal();
    r7z_equal();
    targz_equal();
    tarxz_equal();

    remove_file();
}