use std::path::Path;

use crate::path_helper;

/// 获取内存大小
pub fn get_memory_size() -> u64 {
    get_memory_size_inner()
}

#[cfg(target_os = "windows")]
fn get_memory_size_inner() -> u64 {
    use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};

    let mut ex = MEMORYSTATUSEX::default();
    if unsafe { GlobalMemoryStatusEx(&mut ex) } != 0 {
        ex.ullTotalPhys
    } else {
        u64::MAX
    }
}

#[cfg(target_os = "linux")]
fn get_memory_size_inner() -> u64 {
    let path = Path::new("/proc/meminfo");
    if path.exists() {
        if let Ok(data) = path_helper::read_text(path) {
            let datas = data.lines();
            for item in datas {
                if item.starts_with("MemTotal:") {
                    let parts: Vec<&str> = item
                        .split_whitespace()
                        .filter(|item| !item.is_empty())
                        .collect();
                    if parts.len() >= 2
                        && let Ok(data) = parts[1].parse::<u64>()
                    {
                        return data;
                    }
                }
            }
        }

        u64::MAX
    } else {
        u64::MAX
    }
}

#[cfg(target_os = "macos")]
fn get_memory_size_inner() -> u64 {
    let path = Path::new("/proc/meminfo");
    if path.exists() {
        if let Ok(data) = path_helper::read_text(path) {
            let datas = data.lines();
            for item in datas {
                if item.starts_with("MemTotal:") {
                    let parts: Vec<&str> = item
                        .split_whitespace()
                        .filter(|item| !item.is_empty())
                        .collect();
                    if parts.len() >= 2
                        && let Ok(data) = parts[1].parse::<u64>()
                    {
                        return data;
                    }
                }
            }
        }

        u64::MAX
    } else {
        u64::MAX
    }
}