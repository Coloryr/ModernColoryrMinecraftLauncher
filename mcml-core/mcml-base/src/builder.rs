/// 将字符串列表换成字符串，一个数据一个换行
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
