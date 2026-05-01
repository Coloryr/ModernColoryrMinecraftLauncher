use mcml_core::config::config_obj::{
    ConfigObj, DnsObj, GCType, HttpObj, JvmConfigObj, RunArgObj, SourceLocal, WindowSettingObj,
};
use mcml_core::core::CoreInitObj;

#[test]
fn test_core_init_obj_new() {
    let obj = CoreInitObj::new(
        "/path/to/local".to_string(),
        "oauth_key_123".to_string(),
        "curseforge_key_456".to_string(),
    );
    assert_eq!(obj.local, "/path/to/local");
    assert_eq!(obj.oauth_key, "oauth_key_123");
    assert_eq!(obj.curseforge_key, "curseforge_key_456");
}

#[test]
fn test_jvm_config_default() {
    let config = JvmConfigObj::default();
    assert_eq!(config.name, "");
    assert_eq!(config.local, "");
}

#[test]
fn test_jvm_config_custom() {
    let config = JvmConfigObj {
        name: "Java 17".to_string(),
        local: "C:/Java/jdk-17".to_string(),
    };
    assert_eq!(config.name, "Java 17");
    assert_eq!(config.local, "C:/Java/jdk-17");
}

#[test]
fn test_source_local_default() {
    let source = SourceLocal::default();
    assert_eq!(source, SourceLocal::Offical);
}

#[test]
fn test_source_local_variants() {
    assert_eq!(SourceLocal::Offical as u8, 0);
    assert_eq!(SourceLocal::Bmclapi as u8, 1);
}

#[test]
fn test_http_obj_default() {
    let http = HttpObj::default();
    assert_eq!(http.source, SourceLocal::Offical);
    assert_eq!(http.download_thread, 5);
    assert_eq!(http.proxy_ip, "127.0.0.1");
    assert_eq!(http.proxy_port, 7890);
    assert!(http.check_file);
    assert!(http.auto_download);
    assert!(!http.login_proxy);
    assert!(!http.download_proxy);
    assert!(!http.game_proxy);
}

#[test]
fn test_http_obj_custom() {
    let http = HttpObj {
        source: SourceLocal::Bmclapi,
        download_thread: 10,
        proxy_ip: "192.168.1.1".to_string(),
        proxy_port: 8080,
        ..Default::default()
    };
    assert_eq!(http.source, SourceLocal::Bmclapi);
    assert_eq!(http.download_thread, 10);
    assert_eq!(http.proxy_ip, "192.168.1.1");
    assert_eq!(http.proxy_port, 8080);
}

#[test]
fn test_dns_obj_default() {
    let dns = DnsObj::default();
    assert!(!dns.enable);
    assert!(dns.https.is_empty());
    assert!(!dns.http_proxy);
}

#[test]
fn test_dns_obj_custom() {
    let dns = DnsObj {
        enable: true,
        https: vec!["https://dns.example.com/dns-query".to_string()],
        http_proxy: true,
    };
    assert!(dns.enable);
    assert_eq!(dns.https.len(), 1);
    assert!(dns.http_proxy);
}

#[test]
fn test_gc_type_default() {
    let gc = GCType::default();
    assert_eq!(gc, GCType::Auto);
}

#[test]
fn test_gc_type_variants() {
    assert_eq!(GCType::Auto as u8, 0);
    assert_eq!(GCType::G1GC as u8, 1);
    assert_eq!(GCType::ZGC as u8, 2);
    assert_eq!(GCType::None as u8, 3);
}

#[test]
fn test_run_arg_obj_default() {
    let arg = RunArgObj::default();
    assert!(arg.remove_jvm_arg.is_none());
    assert!(arg.remove_game_arg.is_none());
    assert!(arg.jvm_args.is_none());
    assert!(arg.max_memory.is_none());
}

#[test]
fn test_run_arg_obj_new() {
    let arg = RunArgObj::new();
    assert_eq!(arg.remove_jvm_arg, Some(false));
    assert_eq!(arg.remove_game_arg, Some(false));
    assert_eq!(arg.jvm_args, Some(String::new()));
    assert_eq!(arg.gc_mode, Some(GCType::Auto));
    assert_eq!(arg.max_memory, Some(512));
    assert_eq!(arg.min_memory, Some(4096));
    assert_eq!(arg.color_asm, Some(false));
}

#[test]
fn test_run_arg_obj_custom() {
    let arg = RunArgObj {
        remove_jvm_arg: Some(true),
        jvm_args: Some("-Xmx2G".to_string()),
        max_memory: Some(2048),
        min_memory: Some(1024),
        gc_mode: Some(GCType::G1GC),
        ..Default::default()
    };
    assert_eq!(arg.remove_jvm_arg, Some(true));
    assert_eq!(arg.jvm_args, Some("-Xmx2G".to_string()));
    assert_eq!(arg.max_memory, Some(2048));
    assert_eq!(arg.min_memory, Some(1024));
    assert_eq!(arg.gc_mode, Some(GCType::G1GC));
}

#[test]
fn test_window_setting_obj_default() {
    let window = WindowSettingObj::default();
    assert!(window.full_screen.is_none());
    assert!(window.width.is_none());
    assert!(window.height.is_none());
    assert!(window.game_title.is_none());
}

#[test]
fn test_window_setting_obj_custom() {
    let window = WindowSettingObj {
        full_screen: Some(true),
        width: Some(1920),
        height: Some(1080),
        game_title: Some("Minecraft".to_string()),
        ..Default::default()
    };
    assert_eq!(window.full_screen, Some(true));
    assert_eq!(window.width, Some(1920));
    assert_eq!(window.height, Some(1080));
    assert_eq!(window.game_title, Some("Minecraft".to_string()));
}

#[test]
fn test_config_obj_default() {
    let config = ConfigObj::default();
    assert!(!config.version.is_empty());
    assert!(config.java_list.is_empty());
    assert_eq!(config.http.source, SourceLocal::Offical);
    assert!(!config.dns.enable);
}

#[test]
fn test_config_obj_custom() {
    let config = ConfigObj {
        version: "1.0.0".to_string(),
        java_list: vec![JvmConfigObj {
            name: "Java 21".to_string(),
            local: "/usr/lib/jvm/java-21".to_string(),
        }],
        http: HttpObj {
            source: SourceLocal::Bmclapi,
            ..Default::default()
        },
        ..Default::default()
    };
    assert_eq!(config.version, "1.0.0");
    assert_eq!(config.java_list.len(), 1);
    assert_eq!(config.java_list[0].name, "Java 21");
    assert_eq!(config.http.source, SourceLocal::Bmclapi);
}
