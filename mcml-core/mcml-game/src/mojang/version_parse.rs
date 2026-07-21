/// Minecraft 版本号解析模块
/// 支持所有 Minecraft 版本格式：
/// - 旧格式正式版: 1.20.4, 1.7.10
/// - 旧格式快照: 24w13a, 25w08a
/// - 旧格式预发布: 1.20.4-pre1, 1.20.4-pre4
/// - 旧格式 RC: 1.20.4-rc1
/// - 新格式正式版: 26.1, 26.1.1 (2026年起)
/// - 新格式快照: 26.1-snapshot-1
/// - 新格式预发布: 26.1-pre-1
/// - 新格式 RC: 26.1-rc-2
/// - 远古版本: a1.0.16, b1.7.3, rd-160052

/// 将 Minecraft 版本号转换为可比较的元组
/// 返回 None 表示无法解析
pub fn parse_game_version(version: &str) -> Option<Vec<i32>> {
    let v = version.trim();

    // 1. 新格式快照: 26.1-snapshot-11
    if let Some(result) = parse_new_snapshot(v) {
        return Some(result);
    }

    // 2. 新格式预发布: 26.1-pre-1
    if let Some(result) = parse_new_pre_release(v) {
        return Some(result);
    }

    // 3. 新格式 RC: 26.1-rc-2
    if let Some(result) = parse_new_rc(v) {
        return Some(result);
    }

    // 4. 旧格式快照: 24w13a
    if let Some(result) = parse_old_snapshot(v) {
        return Some(result);
    }

    // 5. 旧格式预发布/RC: 1.20.4-pre1, 1.20.4-rc1
    if let Some(result) = parse_old_prerelease(v) {
        return Some(result);
    }

    // 6. 远古版本: a1.0.16, b1.7.3
    if let Some(result) = parse_ancient_version(v) {
        return Some(result);
    }

    // 7. 新格式正式版: 26.1, 26.1.1 (年份 >= 25)
    if let Some(result) = parse_new_release(v) {
        return Some(result);
    }

    // 8. 旧格式正式版: 1.20.4, 1.20, 1.20.0
    if let Some(result) = parse_old_release(v) {
        return Some(result);
    }

    None
}

/// 解析旧格式正式版（标准化为三段式）
/// "1.20"   -> [1, 20, 0]
/// "1.20.0" -> [1, 20, 0]
/// "1.7.10" -> [1, 7, 10]
/// "1.21"   -> [1, 21, 0]
fn parse_old_release(s: &str) -> Option<Vec<i32>> {
    let parts: Vec<i32> = s.split('.').filter_map(|p| p.parse().ok()).collect();

    if parts.is_empty() {
        return None;
    }

    // 确保是旧格式（以 1 开头）
    if parts[0] != 1 {
        return None;
    }

    // 标准化为三段式: [major, minor, patch]
    let major = parts[0];
    let minor = if parts.len() >= 2 { parts[1] } else { 0 };
    let patch = if parts.len() >= 3 { parts[2] } else { 0 };

    Some(vec![major, minor, patch])
}

/// 解析新格式正式版: 26.1, 26.1.1
/// 输出: [100 + year, release_num, hotfix_num]
fn parse_new_release(s: &str) -> Option<Vec<i32>> {
    let parts: Vec<&str> = s.split('.').collect();
    if parts.len() < 2 {
        return None;
    }

    let year = parts[0].parse::<i32>().ok()?;
    if year < 25 {
        return None;
    }

    let release_num = parts[1].parse::<i32>().ok()?;
    let hotfix = if parts.len() >= 3 {
        parts[2].parse::<i32>().ok()?
    } else {
        0
    };

    Some(vec![100 + year, release_num, hotfix])
}

/// 解析新格式快照: 26.1-snapshot-11
fn parse_new_snapshot(s: &str) -> Option<Vec<i32>> {
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 3 {
        return None;
    }
    if parts[1] != "snapshot" {
        return None;
    }

    let base_parts: Vec<&str> = parts[0].split('.').collect();
    if base_parts.len() < 2 {
        return None;
    }

    let year = base_parts[0].parse::<i32>().ok()?;
    if year < 25 {
        return None;
    }

    let release_num = base_parts[1].parse::<i32>().ok()?;
    let snapshot_num = parts[2].parse::<i32>().ok()?;

    // 快照类型 = 10，确保排在正式版(100+)和预发布(30)之前
    Some(vec![10, 100 + year, release_num, snapshot_num])
}

/// 解析新格式预发布: 26.1-pre-1
fn parse_new_pre_release(s: &str) -> Option<Vec<i32>> {
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 3 {
        return None;
    }
    if parts[1] != "pre" {
        return None;
    }

    let base_parts: Vec<&str> = parts[0].split('.').collect();
    if base_parts.len() < 2 {
        return None;
    }

    let year = base_parts[0].parse::<i32>().ok()?;
    if year < 25 {
        return None;
    }

    let release_num = base_parts[1].parse::<i32>().ok()?;
    let pre_num = parts[2].parse::<i32>().ok()?;

    // 预发布类型 = 30，快照(10) < 预发布(30) < 正式版(100+)
    Some(vec![30, 100 + year, release_num, pre_num])
}

/// 解析新格式 RC: 26.1-rc-2
fn parse_new_rc(s: &str) -> Option<Vec<i32>> {
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 3 {
        return None;
    }
    if parts[1] != "rc" {
        return None;
    }

    let base_parts: Vec<&str> = parts[0].split('.').collect();
    if base_parts.len() < 2 {
        return None;
    }

    let year = base_parts[0].parse::<i32>().ok()?;
    if year < 25 {
        return None;
    }

    let release_num = base_parts[1].parse::<i32>().ok()?;
    let rc_num = parts[2].parse::<i32>().ok()?;

    // RC 类型 = 20，快照(10) < RC(20) < 预发布(30) < 正式版(100+)
    Some(vec![20, 100 + year, release_num, rc_num])
}

/// 解析旧格式快照: 24w13a
fn parse_old_snapshot(s: &str) -> Option<Vec<i32>> {
    // 格式: YYwWWx
    let chars: Vec<char> = s.chars().collect();
    if chars.len() < 5 {
        return None;
    }
    if chars[2] != 'w' {
        return None;
    }

    let year_str: String = chars[0..2].iter().collect();
    let week_str: String = chars[3..5].iter().collect();

    let year: i32 = year_str.parse().ok()?;
    let week: i32 = week_str.parse().ok()?;
    let sub = if chars.len() > 5 {
        (chars[5] as i32) - ('a' as i32) + 1
    } else {
        0
    };

    // 旧快照类型 = -10，确保排在正式版(1.x)之前
    Some(vec![-10, 2000 + year, week, sub])
}

/// 解析旧格式预发布/RC: 1.20.4-pre1, 1.20.4-rc1
fn parse_old_prerelease(s: &str) -> Option<Vec<i32>> {
    let (base_part, tag, num) = if let Some(idx) = s.find("-pre") {
        let base = &s[..idx];
        let num_str = &s[idx + 4..];
        let num = num_str.parse::<i32>().ok()?;
        (base, -5, num)
    } else if let Some(idx) = s.find("-rc") {
        let base = &s[..idx];
        let num_str = &s[idx + 3..];
        let num = num_str.parse::<i32>().ok()?;
        (base, -3, num)
    } else {
        return None;
    };

    // 解析基础版本并标准化为三段式
    let base_parts: Vec<i32> = base_part
        .split('.')
        .filter_map(|p| p.parse().ok())
        .collect();
    if base_parts.is_empty() {
        return None;
    }

    let major = base_parts[0];
    let minor = if base_parts.len() >= 2 {
        base_parts[1]
    } else {
        0
    };
    let patch = if base_parts.len() >= 3 {
        base_parts[2]
    } else {
        0
    };

    Some(vec![tag, major, minor, patch, num])
}

/// 解析远古版本: a1.0.16, b1.7.3, rd-160052
fn parse_ancient_version(s: &str) -> Option<Vec<i32>> {
    // Alpha: a1.0.16
    if s.starts_with('a') {
        let without_prefix = &s[1..];
        let parts: Vec<i32> = without_prefix
            .split('.')
            .filter_map(|p| p.parse().ok())
            .collect();
        if !parts.is_empty() {
            let mut result = vec![-30]; // Alpha 标记
            result.extend(parts);
            return Some(result);
        }
    }

    // Beta: b1.7.3
    if s.starts_with('b') {
        let without_prefix = &s[1..];
        let parts: Vec<i32> = without_prefix
            .split('.')
            .filter_map(|p| p.parse().ok())
            .collect();
        if !parts.is_empty() {
            let mut result = vec![-20]; // Beta 标记
            result.extend(parts);
            return Some(result);
        }
    }

    // rd-160052 等远古版本
    if s.starts_with("rd-") || s.starts_with("inf-") {
        let parts: Vec<i32> = s.split('-').filter_map(|p| p.parse().ok()).collect();
        if !parts.is_empty() {
            return Some(vec![-40, parts[0]]);
        }
    }

    None
}
