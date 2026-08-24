use mcml_names::{i18, i18_items::error_type::ErrorType};

/// 致命错误：弹系统错误框并退出程序（核心初始化失败等）
pub fn fatal_error(e: ErrorType) -> ! {
    // names 提供的 i18n 方法：ErrorType -> 本地化字符串
    let msg = i18::get_error(e);
    eprintln!("{msg}");

    rfd::MessageDialog::new()
        .set_level(rfd::MessageLevel::Error)
        .set_title("MCML 启动器 - 初始化失败")
        .set_description(msg.as_str())
        .show();

    std::process::exit(1)
}

/// 致命错误：弹系统错误框并退出程序
pub fn fatal_error_text(text: &str) -> ! {
    eprintln!("{text}");

    rfd::MessageDialog::new()
        .set_level(rfd::MessageLevel::Error)
        .set_title("MCML 启动器 - 运行错误")
        .set_description(text)
        .show();

    std::process::exit(1)
}