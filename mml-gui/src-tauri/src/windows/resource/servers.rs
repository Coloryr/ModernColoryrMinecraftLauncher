//! 资源管理窗口的服务器分类：列表 / 添加 / 修改 / 删除
//!
//! 读写 `servers.dat`（NBT），由内核 `InstanceSettingObj` 的 server 方法承担。

use crate::dtos::ServerItemDto;

use super::parse_instance;

// ==================== 服务器 ====================

/// 服务器列表（servers.dat，同步纯读）
#[gui_macros::ipc_group("resource")]
#[tauri::command]
pub fn resource_list_servers(uuid: String) -> Result<Vec<ServerItemDto>, String> {
    let instance = parse_instance(&uuid)?;
    let list = instance
        .read()
        .unwrap()
        .get_server_infos()
        .map_err(|err| err.to_string())?;

    Ok(list
        .iter()
        .map(|item| ServerItemDto {
            name: item.name.clone(),
            ip: item.ip.clone(),
            accept_textures: item.accept_textures,
            // servers.dat 里存的就是 base64 字符串，直接拼 data URL
            icon: match item.icon.as_deref() {
                Some(icon) if !icon.is_empty() => format!("data:image/png;base64,{icon}"),
                _ => String::new(),
            },
        })
        .collect())
}

/// 添加服务器
#[gui_macros::ipc_group("resource")]
#[tauri::command]
pub fn resource_server_add(uuid: String, name: String, ip: String) -> Result<(), String> {
    if name.trim().is_empty() || ip.trim().is_empty() {
        return Err("err.nameIp".to_string());
    }
    let instance = parse_instance(&uuid)?;
    instance
        .read()
        .unwrap()
        .add_server(&name, &ip)
        .map_err(|err| err.to_string())
}

/// 编辑服务器（按原 name + ip 定位，替换名字 / 地址 / 资源包接受开关）
#[gui_macros::ipc_group("resource")]
#[tauri::command]
pub fn resource_server_update(
    uuid: String,
    name: String,
    ip: String,
    new_name: String,
    new_ip: String,
    accept_textures: bool,
) -> Result<(), String> {
    if new_name.trim().is_empty() || new_ip.trim().is_empty() {
        return Err("err.nameIp".to_string());
    }
    let instance = parse_instance(&uuid)?;
    let game = instance.read().unwrap();
    let mut list = game.get_server_infos().map_err(|err| err.to_string())?;

    let mut found = false;
    for item in list.iter_mut() {
        if item.name == name && item.ip == ip {
            item.name = new_name;
            item.ip = new_ip;
            item.accept_textures = accept_textures;
            found = true;
            break;
        }
    }
    if !found {
        return Err("err.fileNotFound".to_string());
    }

    game.save_servers(&list).map_err(|err| err.to_string())
}

/// 删除服务器
#[gui_macros::ipc_group("resource")]
#[tauri::command]
pub fn resource_server_delete(uuid: String, name: String, ip: String) -> Result<(), String> {
    let instance = parse_instance(&uuid)?;
    instance
        .read()
        .unwrap()
        .remove_server(&name, &ip)
        .map_err(|err| err.to_string())
}
