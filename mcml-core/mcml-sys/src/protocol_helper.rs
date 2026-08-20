// 注册快捷启动
pub fn register_protocol_handler(over: bool) {
    if over {
        register_protocol_handler_inner("modrinth");
        register_protocol_handler_inner("colormc");
    }

    register_protocol_handler_inner("mcml");
}

// 取消快捷启动
pub fn delete_protocol_handler() {
    delete_protocol_handler_inner("mcml");
}

#[cfg(target_os = "windows")]
fn register_protocol_handler_inner(id: &str) {
    use winreg::{RegKey, enums::HKEY_CLASSES_ROOT};

    let hklm = RegKey::predef(HKEY_CLASSES_ROOT);
    hklm.open_subkey(id)
        .and_then(|data| {
            data.set_value("", &"URL:MCML Protocol").unwrap();
            data.set_value("URL Protocol", &"").unwrap();

            let file = std::env::current_exe().unwrap();

            let (key, _) = data.create_subkey("DefaultIcon").unwrap();
            key.set_value("", &format!("\"{}\",1", file.to_string_lossy()))
                .unwrap();

            let (key, _) = data.create_subkey("shell\\open\\command").unwrap();
            key.set_value("", &format!("\"{}\" \"%1\"", file.to_string_lossy()))
                .unwrap();

            Ok(())
        })
        .unwrap();
}

#[cfg(target_os = "windows")]
fn delete_protocol_handler_inner(id: &str) {
    use winreg::{RegKey, enums::HKEY_CLASSES_ROOT};

    let hklm = RegKey::predef(HKEY_CLASSES_ROOT);
    hklm.open_subkey(id)
        .and_then(|data| {
            data.set_value("", &"URL:MCML Protocol").unwrap();
            data.set_value("URL Protocol", &"").unwrap();

            let file = std::env::current_exe().unwrap();

            let (key, _) = data.create_subkey("DefaultIcon").unwrap();
            key.set_value("", &format!("\"{}\",1", file.to_string_lossy()))
                .unwrap();

            let (key, _) = data.create_subkey("shell\\open\\command").unwrap();
            key.set_value("", &format!("\"{}\" \"%1\"", file.to_string_lossy()))
                .unwrap();

            Ok(())
        })
        .unwrap();
}

#[cfg(not(target_os = "windows"))]
fn register_protocol_handler_inner(id: &str) {}

#[cfg(not(target_os = "windows"))]
fn delete_protocol_handler_inner(id: &str) {}
