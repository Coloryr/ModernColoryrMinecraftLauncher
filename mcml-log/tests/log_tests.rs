use mcml_log::log_item::LogItem;
use mcml_log::log_level::LogLevel;

#[test]
fn test_log_item_new() {
    let item = LogItem::new("test message".to_string(), LogLevel::Info);
    assert_eq!(item.log, "test message");
}

#[test]
fn test_log_item_get_level() {
    let info = LogItem::new("info".to_string(), LogLevel::Info);
    let warn = LogItem::new("warn".to_string(), LogLevel::Warn);
    let error = LogItem::new("error".to_string(), LogLevel::Error);
    let fault = LogItem::new("fault".to_string(), LogLevel::Fault);

    assert_eq!(info.get_level(), "Info");
    assert_eq!(warn.get_level(), "Warn");
    assert_eq!(error.get_level(), "Error");
    assert_eq!(fault.get_level(), "Fault");
}

#[test]
fn test_log_item_get_time_format() {
    let item = LogItem::new("time test".to_string(), LogLevel::Info);
    let time_str = item.get_time();
    // 格式应为 "YYYY-M-D H:M:S"
    assert!(time_str.contains('-'), "时间格式应包含 '-'");
    assert!(time_str.contains(':'), "时间格式应包含 ':'");
}

#[test]
fn test_log_level_variants() {
    // 验证 LogLevel 可以匹配所有变体
    let levels = vec![LogLevel::Info, LogLevel::Warn, LogLevel::Error, LogLevel::Fault];
    assert_eq!(levels.len(), 4);
}

#[test]
fn test_log_item_different_messages() {
    let empty = LogItem::new(String::new(), LogLevel::Info);
    let long = LogItem::new("a".repeat(1000), LogLevel::Warn);
    let special = LogItem::new("Hello\nWorld\t!".to_string(), LogLevel::Error);

    assert_eq!(empty.log, "");
    assert_eq!(long.log.len(), 1000);
    assert_eq!(special.log, "Hello\nWorld\t!");
}
