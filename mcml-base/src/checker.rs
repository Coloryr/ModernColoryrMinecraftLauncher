use regex::Regex;
use std::sync::LazyLock;

static REGEX_NUMBER: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[^0-9]+").unwrap());
static REGEX_WORD: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[a-zA-Z0-9]+$").unwrap());

/// 检查是否为数字
/// - `input`: 需要检查的内容
pub fn check_is_not_number(input: &String) -> bool {
    if input.trim().is_empty() {
        return true;
    }
    REGEX_NUMBER.is_match(input)
}

/// 检查是否为英文数字
/// - `input`: 需要检查的内容
pub fn check_is_word(input: &str) -> bool {
    REGEX_WORD.is_match(input)
}
