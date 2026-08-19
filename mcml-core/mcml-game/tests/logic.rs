//! 纯逻辑单元测试：版本解析、版本比较、加载器、运行库版本对比、规则判断。
//!
//! 这些测试不依赖全局状态（实例列表、路径缓存等），可以独立运行。

use mcml_game::launcher_path::libraries_path::LibVersionObj;
use mcml_game::launcher_path::version_path::get_forge_json_name;
use mcml_game::loader::{LoaderKey, LoaderType};
use mcml_game::mojang::check_allow;
use mcml_game::mojang::game_arg_obj::{GameOsObj, GameRulesObj};
use mcml_game::mojang::version_parse::parse_game_version;
use mcml_sys::Os;

/// 版本号解析：覆盖正式版、快照、预发布、RC、新格式、远古版本。
#[test]
fn parse_game_version_formats() {
    // 旧格式正式版
    assert_eq!(parse_game_version("1.20.4"), Some(vec![1, 20, 4]));
    assert_eq!(parse_game_version("1.20"), Some(vec![1, 20, 0]));
    assert_eq!(parse_game_version("1.7.10"), Some(vec![1, 7, 10]));

    // 新格式正式版（100 + 年份）
    assert_eq!(parse_game_version("26.1"), Some(vec![126, 1, 0]));
    assert_eq!(parse_game_version("26.1.1"), Some(vec![126, 1, 1]));

    // 新格式快照 / 预发布 / RC
    assert_eq!(parse_game_version("26.1-snapshot-11"), Some(vec![10, 126, 1, 11]));
    assert_eq!(parse_game_version("26.1-pre-1"), Some(vec![30, 126, 1, 1]));
    assert_eq!(parse_game_version("26.1-rc-2"), Some(vec![20, 126, 1, 2]));

    // 旧格式快照
    assert_eq!(parse_game_version("24w13a"), Some(vec![-10, 2024, 13, 1]));

    // 旧格式预发布 / RC
    assert_eq!(parse_game_version("1.20.4-pre1"), Some(vec![-5, 1, 20, 4, 1]));
    assert_eq!(parse_game_version("1.20.4-rc1"), Some(vec![-3, 1, 20, 4, 1]));

    // 远古版本
    assert_eq!(parse_game_version("a1.0.16"), Some(vec![-30, 1, 0, 16]));
    assert_eq!(parse_game_version("b1.7.3"), Some(vec![-20, 1, 7, 3]));
    assert_eq!(parse_game_version("rd-160052"), Some(vec![-40, 160052]));

    // 无法解析
    assert_eq!(parse_game_version(""), None);
    assert_eq!(parse_game_version("latest"), None);
    assert_eq!(parse_game_version("snapshot"), None);
    // 非数字段被过滤：首位数字成为主版本（历史行为）
    assert_eq!(parse_game_version("1.x"), Some(vec![1, 0, 0]));
    assert_eq!(parse_game_version("x.1.2"), Some(vec![1, 2, 0]));
    // 主版本既非 1（旧格式）也非 >=25（新格式）→ 无法解析
    assert_eq!(parse_game_version("2.0"), None);
    assert_eq!(parse_game_version("20.1"), None);
}

/// 版本排序关系：正式版 > RC > 预发布 > 快照，旧格式早于新格式。
#[test]
fn parse_game_version_ordering() {
    // 旧格式：补丁号决定顺序
    assert!(parse_game_version("1.20.4") > parse_game_version("1.20.2"));
    assert!(parse_game_version("1.21") > parse_game_version("1.20.4"));
    assert!(parse_game_version("1.20.1") < parse_game_version("1.20.2"));

    // 新格式整体晚于旧格式（年份 >= 25 → 100 + 年份）
    assert!(parse_game_version("26.1") > parse_game_version("1.21.1"));

    // 新格式内部：正式版 > 预发布 > RC > 快照
    let release = parse_game_version("26.1").unwrap();
    let pre = parse_game_version("26.1-pre-1").unwrap();
    let rc = parse_game_version("26.1-rc-2").unwrap();
    let snap = parse_game_version("26.1-snapshot-11").unwrap();
    assert!(pre > rc, "预发布应晚于 RC");
    assert!(rc > snap, "RC 应晚于快照");
    assert!(release > pre, "正式版应晚于预发布");

    // 旧格式快照早于旧格式正式版
    assert!(parse_game_version("24w13a") < parse_game_version("1.20.2"));
}

/// 生成 Forge / NeoForge 的 JSON 文件名。
#[test]
fn forge_json_name() {
    // 普通 Forge：install 与非 install
    assert_eq!(
        get_forge_json_name("1.20.4", "1.20.4-49.0.0", false, true),
        "forge-1.20.4-49.0.0-install.json"
    );
    assert_eq!(
        get_forge_json_name("1.20.4", "1.20.4-49.0.0", false, false),
        "forge-1.20.4-49.0.0.json"
    );

    // NeoForge：mc >= 1.20.2 时用 neoforge-{version} 前缀
    assert_eq!(
        get_forge_json_name("1.20.4", "20.4.80", true, true),
        "neoforge-20.4.80-install.json"
    );
    assert_eq!(
        get_forge_json_name("1.20.4", "20.4.80", true, false),
        "neoforge-20.4.80.json"
    );

    // NeoForge：mc < 1.20.2 时回退为 forge-{mc}-{version} 前缀
    assert_eq!(
        get_forge_json_name("1.12.2", "14.23.5.2860", true, true),
        "forge-1.12.2-14.23.5.2860-install.json"
    );
    assert_eq!(
        get_forge_json_name("1.12.2", "14.23.5.2860", true, false),
        "forge-1.12.2-14.23.5.2860.json"
    );
}

/// 运行库版本对象：解析、版本无关比较。
#[test]
fn lib_version_obj() {
    // 标准 maven 坐标 group:artifact:version
    let obj = LibVersionObj::new("net.minecraftforge:forge:1.20.4");
    assert_eq!(obj.pack, "net.minecraftforge");
    assert_eq!(obj.name, "forge");
    assert_eq!(obj.version, "1.20.4");
    assert_eq!(obj.extr, "");

    // 带 classifier（第 4 段）
    let obj = LibVersionObj::new("org.lwjgl:lwjgl:3.3.1:windows");
    assert_eq!(obj.pack, "org.lwjgl");
    assert_eq!(obj.name, "lwjgl");
    assert_eq!(obj.version, "3.3.1");
    assert_eq!(obj.extr, "windows");

    // 不足 3 段时整体作为名字
    let obj = LibVersionObj::new("plain-name");
    assert_eq!(obj.pack, "");
    assert_eq!(obj.name, "plain-name");
    assert_eq!(obj.version, "");
    assert_eq!(obj.extr, "");

    // 版本无关比较：仅版本不同 → 相等
    let v1 = LibVersionObj::new("net.minecraftforge:forge:1.20.4");
    let v2 = LibVersionObj::new("net.minecraftforge:forge:1.20.1");
    assert!(v1.eq_without_version(&v2));
    // PartialEq 基于 eq_without_version
    assert!(v1 == v2);
    // 名字不同 → 不等
    assert!(v1 != LibVersionObj::new("net.minecraftforge:neoforge:1.20.4"));
    // classifier 不同 → 不等
    assert!(LibVersionObj::new("a:b:1") != LibVersionObj::new("a:b:1:win"));
}

/// 加载器类型前缀与键。
#[test]
fn loader_type_prefix() {
    assert_eq!(LoaderType::Normal.prefix(), "normal");
    assert_eq!(LoaderType::Forge.prefix(), "forge");
    assert_eq!(LoaderType::Fabric.prefix(), "fabric");
    assert_eq!(LoaderType::Quilt.prefix(), "quilt");
    assert_eq!(LoaderType::NeoForge.prefix(), "neoforge");
    assert_eq!(LoaderType::OptiFine.prefix(), "optifine");
    assert_eq!(LoaderType::LiteLoader.prefix(), "liteloader");
    assert_eq!(LoaderType::Custom.prefix(), "custom");

    let key = LoaderKey::new("1.20.4", "49.0.0");
    assert_eq!(key.mc, "1.20.4");
    assert_eq!(key.version, "49.0.0");
    assert_eq!(key, LoaderKey::new("1.20.4", "49.0.0"));
}

/// 规则判断：allow / disallow 及平台匹配。
#[test]
fn check_allow_rules() {
    let sys = mcml_sys::get_system_info();

    // 空规则 → 允许
    assert!(check_allow(&Vec::new()));

    // 无 os 限制的 allow → 允许；无 os 限制的 disallow → 拒绝
    assert!(check_allow(&vec![GameRulesObj {
        action: "allow".to_string(),
        os: None,
    }]));
    assert!(!check_allow(&vec![GameRulesObj {
        action: "disallow".to_string(),
        os: None,
    }]));

    // 平台匹配：allow {os} 只有当 os 与当前系统一致才允许
    let allow_win = GameRulesObj {
        action: "allow".to_string(),
        os: Some(GameOsObj {
            name: "windows".to_string(),
            arch: String::new(),
        }),
    };
    let expect_win = matches!(sys.os, Os::Windows);
    assert_eq!(check_allow(&vec![allow_win]), expect_win);

    // disallow {os}：只有当前系统匹配时才拒绝
    let disallow_linux = GameRulesObj {
        action: "disallow".to_string(),
        os: Some(GameOsObj {
            name: "linux".to_string(),
            arch: String::new(),
        }),
    };
    let expect_linux_block = matches!(sys.os, Os::Linux);
    assert_eq!(check_allow(&vec![disallow_linux]), !expect_linux_block);

    // 架构匹配：allow x86 在非 ARM 平台上放行
    let allow_x86 = GameRulesObj {
        action: "allow".to_string(),
        os: Some(GameOsObj {
            name: String::new(),
            arch: "x86".to_string(),
        }),
    };
    assert_eq!(check_allow(&vec![allow_x86]), !sys.is_arm);

    // disallow x86 在非 ARM 平台上拒绝
    let disallow_x86 = GameRulesObj {
        action: "disallow".to_string(),
        os: Some(GameOsObj {
            name: String::new(),
            arch: "x86".to_string(),
        }),
    };
    assert_eq!(check_allow(&vec![disallow_x86]), sys.is_arm);
}
