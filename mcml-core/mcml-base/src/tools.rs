use regex::Regex;
use std::{path::Path, sync::LazyLock};

/// 匹配含非数字字符的正则
static REGEX_NUMBER: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[^0-9]+").unwrap());
/// 匹配纯英文数字的正则
static REGEX_WORD: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[a-zA-Z0-9]+$").unwrap());

/// 检查输入是否包含非数字字符
///
/// - `input`: 需要检查的内容
pub fn check_is_not_number(input: &str) -> bool {
    if input.trim().is_empty() {
        return true;
    }
    REGEX_NUMBER.is_match(input)
}

/// 检查是否为英文数字
///
/// - `input`: 需要检查的内容
pub fn check_is_word(input: &str) -> bool {
    REGEX_WORD.is_match(input)
}

/// 截取字符串
pub fn get_string(input: &str, start: &str, end: &str) -> String {
    if let Some(start_byte) = input.find(start) {
        let start_end_byte = start_byte + start.len();

        let tail = &input[start_end_byte..];
        let skip_bytes = match tail.chars().next() {
            Some(ch) => ch.len_utf8(),
            None => return input.to_string(),
        };
        let search_start_byte = start_end_byte + skip_bytes;

        if let Some(rel_end_byte) = input[search_start_byte..].find(end) {
            let end_byte = search_start_byte + rel_end_byte;
            return input[start_end_byte..end_byte].to_string();
        }
    }
    input.to_string()
}

/// 将字符串列表转换为以换行符分隔的单个字符串
///
/// 每个原始字符串后附加平台对应的换行符（`\r\n` 或 `\n`）。
///
/// - `vec`: 输入数据
pub fn build_vec_string(vec: &Vec<String>) -> String {
    let mut str = String::new();

    for item in vec.iter() {
        str.push_str(item);
        str.push_str(&mcml_names::get_line_ending());
    }

    str
}

pub struct PathPart {
    pub parent: String,
    pub file: String,
}

/// 将输入路径拆分
///
/// 例如输入home/user/text.txt，则输出home/user和text.txt
pub fn get_path_part(input: &str) -> PathPart {
    let path = Path::new(input);

    let parent = path
        .parent()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|| "".to_string());

    let file = path
        .file_name()
        .map(|f| f.to_string_lossy().into_owned())
        .unwrap_or_else(|| "".to_string());

    PathPart { parent, file }
}

/// 命令行参数解析
///
/// # 参数
/// * `input` - 命令行字符串
///
/// # 返回值
/// 参数字符串向量
pub fn arg_parse(input: &str) -> Vec<String> {
    let quote_char = '"';
    let escape_char = '\\';
    let mut inside_quote = false;
    let mut inside_escape = false;

    let mut current_arg = String::new();
    let mut current_arg_char_count = 0;
    let mut result = Vec::new();

    for c in input.chars() {
        if c == quote_char {
            current_arg_char_count += 1;

            if inside_escape {
                current_arg.push(c);
                inside_escape = false;
            } else if inside_quote {
                inside_quote = false;
            } else {
                inside_quote = true;
            }
        } else if c == escape_char {
            current_arg_char_count += 1;

            if inside_escape {
                current_arg.push_str(&format!("{}{}", escape_char, escape_char));
            }

            inside_escape = !inside_escape;
        } else if c.is_whitespace() {
            if inside_quote {
                current_arg_char_count += 1;
                current_arg.push(c);
            } else {
                if current_arg_char_count > 0 {
                    result.push(current_arg.clone());
                }
                current_arg_char_count = 0;
                current_arg.clear();
            }
        } else {
            current_arg_char_count += 1;
            if inside_escape {
                current_arg.push(escape_char);
                current_arg_char_count = 0;
                inside_escape = false;
            }
            current_arg.push(c);
        }
    }

    if current_arg_char_count > 0 {
        result.push(current_arg);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 检查 check_is_not_number 的各种输入
    #[test]
    fn test_check_is_not_number() {
        // 空串 / 空白串视为含非数字
        assert!(check_is_not_number(""));
        assert!(check_is_not_number("   "));
        // 纯数字（无任何空白）
        assert!(!check_is_not_number("123"));
        // 注意现状：trim 仅用于空串判断，首尾空白本身按非数字字符处理
        assert!(check_is_not_number(" 123 "));
        // 含非数字字符
        assert!(check_is_not_number("12a3"));
        assert!(check_is_not_number("1.5"));
        assert!(check_is_not_number("-1"));
    }

    /// 检查 check_is_word 的各种输入
    #[test]
    fn test_check_is_word() {
        assert!(check_is_word("abc123"));
        assert!(check_is_word("ABC"));
        assert!(check_is_word("123"));
        assert!(!check_is_word(""));
        assert!(!check_is_word("a-b"));
        assert!(!check_is_word("中文"));
        assert!(!check_is_word("a b"));
    }

    /// get_string 截取 start 与 end 之间的内容
    #[test]
    fn test_get_string() {
        assert_eq!(get_string("hello world", "l", "o"), "l");
        assert_eq!(get_string("a=1;b=2;", "=", ";"), "1");
        // start 不存在时返回原串
        assert_eq!(get_string("abc", "x", "y"), "abc");
        // end 不存在时返回原串
        assert_eq!(get_string("abc", "a", "z"), "abc");
    }

    /// build_vec_string 用平台换行符拼接
    #[test]
    fn test_build_vec_string() {
        let ending = mcml_names::get_line_ending();
        let vec = vec!["a".to_string(), "b".to_string()];
        assert_eq!(build_vec_string(&vec), format!("a{ending}b{ending}"));
        // 空列表
        assert_eq!(build_vec_string(&Vec::new()), "");
    }

    /// get_path_part 拆分目录与文件名
    #[test]
    fn test_get_path_part() {
        let part = get_path_part("home/user/text.txt");
        assert_eq!(part.parent.replace('\\', "/"), "home/user");
        assert_eq!(part.file, "text.txt");

        // 只有文件名时父目录为空
        let part = get_path_part("text.txt");
        assert_eq!(part.parent, "");
        assert_eq!(part.file, "text.txt");
    }

    /// arg_parse 基本分词
    #[test]
    fn test_arg_parse_simple() {
        assert_eq!(arg_parse("a b c"), vec!["a", "b", "c"]);
        // 多余空白
        assert_eq!(arg_parse("  a   b  "), vec!["a", "b"]);
        // 空串
        assert!(arg_parse("").is_empty());
        assert!(arg_parse("   ").is_empty());
    }

    /// arg_parse 引号内空白保留
    #[test]
    fn test_arg_parse_quote() {
        assert_eq!(arg_parse("a \"b c\" d"), vec!["a", "b c", "d"]);
        // 引号在参数中间
        assert_eq!(arg_parse("pre\"x y\"post"), vec!["prex ypost"]);
    }

    /// arg_parse 转义
    #[test]
    fn test_arg_parse_escape() {
        // 现状记录（疑似 bug）：转义字符后的字符虽被追加，但计数被清零，
        // 若转义后紧跟参数结尾，整个参数会被丢弃
        assert_eq!(arg_parse(r"a\b"), Vec::<String>::new());
        // 转义后还有后续字符时参数保留（内容为 a + 反斜杠 + bc）
        assert_eq!(arg_parse(r"a\bc"), vec![r"a\bc"]);
        // 引号前的反斜杠转义引号
        assert_eq!(arg_parse("a\\\"b"), vec!["a\"b"]);
    }
}
