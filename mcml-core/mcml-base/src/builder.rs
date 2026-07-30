//! 字符串构建工具
//!
//! 提供常用字符串拼接操作的辅助函数。

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
