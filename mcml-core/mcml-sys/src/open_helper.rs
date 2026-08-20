use std::path::Path;

/// 在浏览器打开网址
pub fn open_url(url: &str) {
    webbrowser::open(url).unwrap();
}

/// 在资源管理器打开文件
pub fn open_file_with_explorer<P: AsRef<Path>>(path: P) {
    opener::reveal(path.as_ref()).unwrap();
}

/// 以系统默认方式打开文件
pub fn open_file<P: AsRef<Path>>(path: P) {
    opener::open(path.as_ref()).unwrap();
}