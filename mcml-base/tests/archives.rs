use std::{collections::HashMap, fs, path::{Path, PathBuf}};

use mcml_base::archives::{IArchive, r7z_runner::R7zProcess, zip_runner::ZipProcess};

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
    let file = Path::new("tests");

    let runner = ZipProcess::new(None);

    let zip = file.join("test.zip");
    let dir = file.join("test_compress/");
    let filter = Some(vec![
        String::from("skip.text"),
        String::from("dir1/skip.text"),
    ]);
    let res = runner.compress(&zip, &dir, Some(&dir), &filter);
    assert!(res.is_ok());
}

#[test]
#[ignore]
fn unzip() {
    let file = Path::new("tests");

    let runner = ZipProcess::new(None);

    let zip = file.join("test.zip");
    let dir = file.join("test_unzip/");
    let res = runner.decompress(&zip, &dir);
    assert!(res.is_ok());
}

#[test]
#[ignore]
fn r7z() {
    let file = Path::new("tests");

    let runner = R7zProcess::new(None);

    let zip = file.join("test.7z");
    let dir = file.join("test_compress/");
    let filter = Some(vec![
        String::from("skip.text"),
        String::from("dir1/skip.text"),
    ]);
    let res = runner.compress(&zip, &dir, Some(&dir), &filter);
    assert!(res.is_ok());
}

#[test]
#[ignore]
fn unr7z() {
    let file = Path::new("tests");

    let runner = R7zProcess::new(None);

    let zip = file.join("test.7z");
    let dir = file.join("test_un7z/");
    let res = runner.decompress(&zip, &dir);
    assert!(res.is_ok());
}

#[test]
#[ignore]
fn zip_equal() {
    let file = Path::new("tests");
    let dir = file.join("test_compress/");
    let dir1 = file.join("test_unzip/");

    compare_folders(&dir, &dir1, &[
        String::from("skip.text"),
        String::from("dir1/skip.text"),
    ]);
}

#[test]
#[ignore]
fn r7z_equal() {
    let file = Path::new("tests");
    let dir = file.join("test_compress/");
    let dir1 = file.join("test_un7z/");

    compare_folders(&dir, &dir1, &[
        String::from("skip.text"),
        String::from("dir1/skip.text"),
    ]);
}

#[test]
#[ignore]
fn remove_file() {
    let file = Path::new("tests");
    let zip = file.join("test.zip");
    fs::remove_file(zip).ok();
    let zip = file.join("test.7z");
    fs::remove_file(zip).ok();

    let dir = file.join("test_unzip/");
    fs::remove_dir_all(dir).ok();
    let dir = file.join("test_un7z/");
    fs::remove_dir_all(dir).ok();
}

#[test]
fn archive_test() {
    zip();
    unzip();
    r7z();
    unr7z();
    zip_equal();
    r7z_equal();
    remove_file();
}