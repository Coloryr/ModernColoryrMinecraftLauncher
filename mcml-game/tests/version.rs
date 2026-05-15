use mcml_game::mojang::version_checker::{is_game_version_117, is_game_version_120, is_game_version_1202, is_game_version_greater};


#[test]
fn test_is_game_version_1202() {
    // 低于 1.20.2
    assert_eq!(is_game_version_1202("1.20.1".to_string()), false);
    assert_eq!(is_game_version_1202("1.20.0".to_string()), false);
    assert_eq!(is_game_version_1202("1.19.4".to_string()), false);
    assert_eq!(is_game_version_1202("1.7.10".to_string()), false);
    assert_eq!(is_game_version_1202("a1.0.16".to_string()), false);
    assert_eq!(is_game_version_1202("b1.7.3".to_string()), false);

    // 等于 1.20.2
    assert_eq!(is_game_version_1202("1.20.2".to_string()), true);

    // 高于 1.20.2
    assert_eq!(is_game_version_1202("1.20.3".to_string()), true);
    assert_eq!(is_game_version_1202("1.20.4".to_string()), true);
    assert_eq!(is_game_version_1202("1.21".to_string()), true);
    assert_eq!(is_game_version_1202("1.21.1".to_string()), true);

    // 新格式
    assert_eq!(is_game_version_1202("26.1".to_string()), true);
    assert_eq!(is_game_version_1202("26.1.1".to_string()), true);

    // 快照和预发布
    assert_eq!(is_game_version_1202("24w13a".to_string()), false); // 1.20.4 的快照，但时间线上早于 1.20.2
    assert_eq!(is_game_version_1202("1.20.2-pre1".to_string()), false); // 预发布 < 正式版
    assert_eq!(is_game_version_1202("1.20.2-rc1".to_string()), false); // RC < 正式版
}

#[test]
fn test_is_game_version_120() {
    // 低于 1.20
    assert_eq!(is_game_version_120("1.20.0".to_string()), false);
    assert_eq!(is_game_version_120("1.19.4".to_string()), false);
    assert_eq!(is_game_version_120("1.7.10".to_string()), false);
    assert_eq!(is_game_version_120("a1.0.16".to_string()), false);
    assert_eq!(is_game_version_120("b1.7.3".to_string()), false);

    // 等于 1.20
    assert_eq!(is_game_version_120("1.20.0".to_string()), true);

    // 高于 1.20
    assert_eq!(is_game_version_120("1.20.3".to_string()), true);
    assert_eq!(is_game_version_120("1.20.4".to_string()), true);
    assert_eq!(is_game_version_120("1.21".to_string()), true);
    assert_eq!(is_game_version_120("1.21.1".to_string()), true);

    // 新格式
    assert_eq!(is_game_version_120("26.1".to_string()), true);
    assert_eq!(is_game_version_120("26.1.1".to_string()), true);

    // 快照和预发布
    assert_eq!(is_game_version_120("24w13a".to_string()), false); // 1.20.4 的快照，但时间线上早于 1.20.2
    assert_eq!(is_game_version_120("1.20-pre1".to_string()), false); // 预发布 < 正式版
    assert_eq!(is_game_version_120("1.20-rc1".to_string()), false); // RC < 正式版
}

#[test]
fn test_is_game_version_117() {
    // 低于 1.17
    assert_eq!(is_game_version_117("1.16.5".to_string()), false);
    assert_eq!(is_game_version_117("1.7.10".to_string()), false);
    assert_eq!(is_game_version_117("a1.0.16".to_string()), false);
    assert_eq!(is_game_version_117("b1.7.3".to_string()), false);

    // 等于 1.17
    assert_eq!(is_game_version_117("1.17.0".to_string()), true);

    // 高于 1.17
    assert_eq!(is_game_version_117("1.20.3".to_string()), true);
    assert_eq!(is_game_version_117("1.20.4".to_string()), true);
    assert_eq!(is_game_version_117("1.21".to_string()), true);
    assert_eq!(is_game_version_117("1.21.1".to_string()), true);

    // 新格式
    assert_eq!(is_game_version_117("26.1".to_string()), true);
    assert_eq!(is_game_version_117("26.1.1".to_string()), true);

    // 快照和预发布
    assert_eq!(is_game_version_117("24w13a".to_string()), false); // 1.20.4 的快照，但时间线上早于 1.20.2
    assert_eq!(is_game_version_117("1.20-pre1".to_string()), false); // 预发布 < 正式版
    assert_eq!(is_game_version_117("1.20-rc1".to_string()), false); // RC < 正式版
}

#[test]
fn test_version_comparison() {
    // 正式版比较
    assert!(is_game_version_greater("1.20.4", "1.20.2"));
    assert!(is_game_version_greater("1.21", "1.20.4"));
    assert!(!is_game_version_greater("1.20.1", "1.20.2"));

    // 新格式比较
    assert!(is_game_version_greater("26.1", "1.21.1"));
    assert!(is_game_version_greater("26.1.1", "26.1"));

    // 快照与正式版
    assert!(!is_game_version_greater("24w13a", "1.20.2"));

    // 预发布与正式版
    assert!(!is_game_version_greater("1.20.2-pre1", "1.20.2"));

    // 新格式各类型
    assert!(is_game_version_greater("26.1", "26.1-snapshot-1"));
    assert!(is_game_version_greater("26.1-pre-1", "26.1-snapshot-1"));
    assert!(is_game_version_greater("26.1-rc-1", "26.1-snapshot-1"));
    assert!(is_game_version_greater("26.1", "26.1-rc-1"));
}
