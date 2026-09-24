//! 致命错误弹窗

use mml_names::{i18, i18_items::error_type::ErrorType};

/// 弹系统错误框并退出程序（核心初始化失败等）
///
/// # 参数
///
/// - `e`: 错误信息（经 i18n 转为本地化文本显示）
pub fn fatal_error(e: ErrorType) -> ! {
    let msg = i18::get_error(e);
    eprintln!("{msg}");

    rfd::MessageDialog::new()
        .set_level(rfd::MessageLevel::Error)
        .set_title("M²L 启动器 - 初始化失败")
        .set_description(msg.as_str())
        .show();

    std::process::exit(1)
}

/// 弹系统错误框并退出程序
///
/// # 参数
///
/// - `text`: 直接显示的错误文本（不经 i18n）
pub fn fatal_error_text(text: &str) -> ! {
    eprintln!("{text}");

    rfd::MessageDialog::new()
        .set_level(rfd::MessageLevel::Error)
        .set_title("M²L 启动器 - 运行错误")
        .set_description(text)
        .show();

    std::process::exit(1)
}
