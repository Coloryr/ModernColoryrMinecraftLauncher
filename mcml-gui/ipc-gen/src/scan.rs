//! 源码扫描与语法解析
//!
//! 全部是纯函数：输入源码文本，输出结构化数据，不碰文件系统与环境变量
//! （`walk_rs` 除外，它只负责遍历目录）。

use std::{
    fs,
    path::{Path, PathBuf},
};

/// 递归收集目录下的 .rs 文件
pub fn walk_rs(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk_rs(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// 文件路径 -> Rust 模块路径（src/windows/account.rs -> "windows::account"；
/// mod.rs / lib.rs / main.rs -> 所在目录即模块）
pub fn module_path(src: &Path, file: &Path) -> String {
    let rel = match file.strip_prefix(src) {
        Ok(rel) => rel,
        Err(_) => return String::new(),
    };
    let mut comps: Vec<String> = rel
        .components()
        .map(|c| c.as_os_str().to_string_lossy().to_string())
        .collect();
    comps.pop();
    let file_name = rel
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let stem = file_name.trim_end_matches(".rs");
    // mod.rs / src 根下的 lib.rs、main.rs 不追加自身名，其余文件名即模块名
    let is_special = stem == "mod" || ((stem == "lib" || stem == "main") && comps.is_empty());
    if !is_special {
        comps.push(stem.to_string());
    }
    comps.join("::")
}

/// 找 marker 之后的第一个函数名（跳过中间的文档 / 属性，标准格式下足够可靠）
pub fn fns_after(text: &str, marker: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut rest = text;
    while let Some(pos) = rest.find(marker) {
        let after = &rest[pos + marker.len()..];
        if let Some(f) = after.find("fn ") {
            let ident: String = after[f + 3..]
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect();
            if !ident.is_empty() {
                names.push(ident);
            }
        }
        rest = after;
    }
    names
}

/// 从 `from` 起，最早的 `pub struct` / `pub enum` 声明起始位置
fn next_decl(text: &str, from: usize) -> Option<usize> {
    let a = text[from..].find("\npub struct ").map(|p| p + from + 1);
    let b = text[from..].find("\npub enum ").map(|p| p + from + 1);
    match (a, b) {
        (Some(x), Some(y)) => Some(x.min(y)),
        (Some(x), None) => Some(x),
        (None, Some(y)) => Some(y),
        (None, None) => None,
    }
}

/// 扫描 `pub struct` / `pub enum` 声明
///
/// 返回 (类型名, 声明前的属性原文, 花括号内的正文, 是枚举)
pub fn scan_types(text: &str) -> Vec<(String, String, String, bool)> {
    let mut out = Vec::new();
    let mut search = 0usize;
    while let Some(pos) = next_decl(text, search) {
        let is_enum = text[pos..].starts_with("pub enum ");
        let head = if is_enum { "pub enum " } else { "pub struct " };
        let name_start = pos + head.len();
        let name: String = text[name_start..]
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
            .collect();
        if name.is_empty() {
            search = name_start;
            continue;
        }
        // 属性块：从声明往回逐行收集 `#[...]` / `///`，遇到其它内容（空行、`}`）即停
        let mut attr_lines: Vec<&str> = Vec::new();
        for line in text[..pos].lines().rev() {
            let t = line.trim();
            if t.starts_with("#[") || t.starts_with("///") {
                attr_lines.push(line);
            } else {
                break;
            }
        }
        attr_lines.reverse();
        let attrs = attr_lines.join("\n");

        // 正文：花括号配平
        let Some(open) = text[name_start..].find('{') else {
            search = name_start;
            continue;
        };
        let body_start = name_start + open + 1;
        let mut depth = 1usize;
        let mut i = body_start;
        let bytes = text.as_bytes();
        while i < bytes.len() && depth > 0 {
            match bytes[i] {
                b'{' => depth += 1,
                b'}' => depth -= 1,
                _ => {}
            }
            i += 1;
        }
        out.push((
            name,
            attrs,
            text[body_start..i.saturating_sub(1)].to_string(),
            is_enum,
        ));
        search = i;
    }
    out
}

/// 从属性块里取 `#[serde(rename_all = "camelCase")]` 的值
pub fn rename_all_of(attrs: &str) -> Option<String> {
    let p = attrs.find("rename_all")?;
    let rest = &attrs[p..];
    let q1 = rest.find('"')? + 1;
    let q2 = rest[q1..].find('"')? + q1;
    Some(rest[q1..q2].to_string())
}

/// 拆顶层逗号（不切泛型里的逗号）
pub fn split_top_level(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut cur = String::new();
    for c in s.chars() {
        match c {
            '<' | '(' | '[' => {
                depth += 1;
                cur.push(c);
            }
            '>' | ')' | ']' => {
                depth -= 1;
                cur.push(c);
            }
            ',' if depth == 0 => {
                out.push(cur.trim().to_string());
                cur.clear();
            }
            _ => cur.push(c),
        }
    }
    if !cur.trim().is_empty() {
        out.push(cur.trim().to_string());
    }
    out
}

/// 取 `#[tauri::command]` 后面的函数：返回 (函数名, 参数原文, 返回类型原文)
///
/// 参数/返回按括号与尖括号配平截取，能处理跨多行的签名。
pub fn commands_in(text: &str) -> Vec<(String, String, String)> {
    const MARK: &str = "#[tauri::command]";
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(pos) = rest.find(MARK) {
        let after = &rest[pos + MARK.len()..];
        let Some(fpos) = after.find("fn ") else {
            rest = after;
            continue;
        };
        let after_fn = &after[fpos + 3..];
        let name: String = after_fn
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
            .collect();
        let Some(lp) = after_fn.find('(') else {
            rest = after_fn;
            continue;
        };

        // 参数：括号配平
        let mut depth = 0i32;
        let mut end = None;
        for (i, c) in after_fn[lp..].char_indices() {
            match c {
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth == 0 {
                        end = Some(lp + i);
                        break;
                    }
                }
                _ => {}
            }
        }
        let Some(rp) = end else {
            rest = after_fn;
            continue;
        };
        let params = after_fn[lp + 1..rp].to_string();

        // 返回类型：若紧跟 `->`，取到函数体的 `{`（尖括号/圆括号配平）
        let tail = &after_fn[rp + 1..];
        let ret = match tail.find("->") {
            Some(a) if tail[..a].trim().is_empty() => {
                let ty_start = a + 2;
                let mut d = 0i32;
                let mut cut = tail.len();
                for (i, c) in tail[ty_start..].char_indices() {
                    match c {
                        '<' | '(' | '[' => d += 1,
                        '>' | ')' | ']' => d -= 1,
                        '{' if d == 0 => {
                            cut = ty_start + i;
                            break;
                        }
                        _ => {}
                    }
                }
                tail[ty_start..cut].trim().to_string()
            }
            _ => String::new(),
        };

        out.push((name, params, ret));
        rest = tail;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 命令签名跨多行也能解析，注入参数留在参数原文里（由 ts.rs 过滤）
    #[test]
    fn test_commands_in_multiline() {
        let src = r#"
/// 文档注释
#[tauri::command]
pub async fn add_modpack_search(
    source: String,
    query: Option<String>,
    page: u32,
) -> Result<ModpackSearchDto, String> {
    todo!()
}
"#;
        let cmds = commands_in(src);
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].0, "add_modpack_search");
        assert!(cmds[0].1.contains("source: String"));
        assert!(cmds[0].1.contains("page: u32"));
        assert_eq!(cmds[0].2, "Result<ModpackSearchDto, String>");
    }

    /// 没有返回类型的函数，返回类型原文是空串
    #[test]
    fn test_commands_in_without_return_type() {
        let src = "#[tauri::command]\npub fn add_set_close_guard(window: WebviewWindow, enabled: bool) {";
        let cmds = commands_in(src);
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].0, "add_set_close_guard");
        assert_eq!(cmds[0].2, "");
    }

    /// 多个命令都要收进来
    #[test]
    fn test_commands_in_multiple() {
        let src = "#[tauri::command]\npub fn a() -> bool { true }\n\n#[tauri::command]\npub fn b() -> i64 { 0 }";
        let names: Vec<String> = commands_in(src).into_iter().map(|c| c.0).collect();
        assert_eq!(names, vec!["a", "b"]);
    }

    /// 事件名：去 `emit_` 前缀、`_` 换 `-`
    #[test]
    fn test_fns_after_emit() {
        let src = "#[gui_macros::emit]\npub fn emit_add_pack_progress(window: &WebviewWindow) {}";
        assert_eq!(fns_after(src, "#[gui_macros::emit]"), vec!["emit_add_pack_progress"]);
        let event = "emit_add_pack_progress"
            .strip_prefix("emit_")
            .unwrap()
            .replace('_', "-");
        assert_eq!(event, "add-pack-progress");
    }

    /// 文件路径 -> 模块路径：mod.rs / 根 lib.rs / 根 main.rs 不追加自身名
    #[test]
    fn test_module_path() {
        let src = Path::new("/x/src");
        assert_eq!(module_path(src, Path::new("/x/src/windows/account.rs")), "windows::account");
        assert_eq!(module_path(src, Path::new("/x/src/windows/mod.rs")), "windows");
        assert_eq!(module_path(src, Path::new("/x/src/lib.rs")), "");
        assert_eq!(module_path(src, Path::new("/x/src/main.rs")), "");
    }

    /// 属性块回溯要能跨过文档注释，并取到 rename_all
    #[test]
    fn test_scan_types_attrs_and_rename() {
        let src = r#"
/// 文档注释
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Foo {
    pub bar_baz: String,
}
"#;
        let types = scan_types(src);
        assert_eq!(types.len(), 1);
        assert_eq!(types[0].0, "Foo");
        assert!(!types[0].3, "不是枚举");
        assert_eq!(rename_all_of(&types[0].1).as_deref(), Some("camelCase"));
        assert!(types[0].2.contains("bar_baz"));
    }

    /// 枚举变体前的文档注释与 `#[default]` 不能干扰变体名提取
    #[test]
    fn test_scan_types_enum_body() {
        let src = r#"
#[derive(Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ViewMode {
    /// 列表（默认）
    #[default]
    List,
    Group,
    Grid,
}
"#;
        let types = scan_types(src);
        assert!(types[0].3, "是枚举");
        assert_eq!(types[0].2.trim().starts_with("/// 列表（默认）"), true);
    }

    /// 顶层逗号拆分不切泛型内部的逗号
    #[test]
    fn test_split_top_level() {
        assert_eq!(split_top_level("a: String, b: u32"), vec!["a: String", "b: u32"]);
        assert_eq!(
            split_top_level("HashMap<String, Vec<String>>, u32"),
            vec!["HashMap<String, Vec<String>>", "u32"]
        );
        assert!(split_top_level("").is_empty());
    }
}
