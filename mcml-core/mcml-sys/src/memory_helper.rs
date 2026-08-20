/// 获取内存大小
#[inline(always)]
pub fn get_memory_size() -> u64 {
    get_memory_size_inner()
}

/// 获取剩余内存大小
#[inline(always)]
pub fn get_memory_free() -> u64 {
    get_memory_free_inner()
}

#[cfg(target_os = "windows")]
fn get_memory_size_inner() -> u64 {
    use windows::Win32::System::SystemInformation::GlobalMemoryStatusEx;
    use windows::Win32::System::SystemInformation::MEMORYSTATUSEX;

    let mut ex = MEMORYSTATUSEX::default();
    if unsafe { GlobalMemoryStatusEx(&mut ex) }.is_ok() {
        ex.ullTotalPhys / 1024 / 1024
    } else {
        u64::MAX
    }
}

#[cfg(target_os = "windows")]
fn get_memory_free_inner() -> u64 {
    use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};

    let mut ex = MEMORYSTATUSEX::default();
    if unsafe { GlobalMemoryStatusEx(&mut ex) }.is_ok() {
        ex.ullAvailPhys / 1024 / 1024
    } else {
        u64::MAX
    }
}

#[cfg(target_os = "linux")]
fn get_memory_size_inner() -> u64 {
    use crate::path_helper;
    use std::path::Path;

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
                        return data / 1024;
                    }
                }
            }
        }

        u64::MAX
    } else {
        u64::MAX
    }
}

#[cfg(target_os = "linux")]
fn get_memory_free_inner() -> u64 {
    use crate::path_helper;
    use std::path::Path;

    let path = Path::new("/proc/meminfo");
    if path.exists() {
        if let Ok(data) = path_helper::read_text(path) {
            let datas = data.lines();
            for item in datas {
                if item.starts_with("MemFree:") {
                    let parts: Vec<&str> = item
                        .split_whitespace()
                        .filter(|item| !item.is_empty())
                        .collect();
                    if parts.len() >= 2
                        && let Ok(data) = parts[1].parse::<u64>()
                    {
                        return data / 1024;
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
    use crate::process_helper;

    let res = process_utils::run_command_arg("sysctl", &["hw.memsize"]);
    if res.is_err() {
        return u64::MAX;
    }

    let res = res.unwrap();
    for item in res {
        if item.starts_with("hw.memsize:") {
            let parts: Vec<&str> = item
                .split_whitespace()
                .filter(|item| !item.is_empty())
                .collect();
            if parts.len() >= 2
                && let Ok(data) = parts[1].parse::<u64>()
            {
                return data / 1024 / 1024;
            }
        }
    }

    return u64::MAX;
}

#[cfg(target_os = "macos")]
fn get_memory_free_inner() -> u64 {
    use crate::process_helper;

    let res = process_utils::run_command("vm_stat");
    if res.is_err() {
        return u64::MAX;
    }

    let res = res.unwrap();
    let mut free_pages = 0u64;
    let mut page_size = 4096u64;
    for item in res {
        if item.starts_with("Pages free:") {
            let parts: Vec<&str> = item
                .split_whitespace()
                .filter(|item| !item.is_empty())
                .collect();
            if parts.len() >= 3
                && let Ok(data) = parts[2].trim_end_matches('.').parse::<u64>()
            {
                free_pages = data;
            }
        } else if item.starts_with("page size of") {
            let parts: Vec<&str> = item
                .split_whitespace()
                .filter(|item| !item.is_empty())
                .collect();
            if parts.len() >= 4
                && let Ok(data) = parts[3].parse::<u64>()
            {
                page_size = data;
            }
        }
    }

    if free_pages > 0 {
        free_pages * page_size / 1024 / 1024
    } else {
        u64::MAX
    }
}
