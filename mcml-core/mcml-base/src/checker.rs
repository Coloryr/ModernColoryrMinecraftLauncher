//! 字符串格式校验模块
//!
//! 提供基于正则表达式的输入校验功能，使用 `LazyLock` 惰性编译正则。

use regex::Regex;
use std::sync::LazyLock;

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
