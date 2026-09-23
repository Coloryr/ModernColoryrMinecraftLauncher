//! 资源管理窗口：实例的模组 / 材质包 / 存档 / 截图 / 服务器 / 光影包 / 结构 / 数据包列表与操作
//!
//! 列表复用 mml-game 的扫描（模组元数据、材质包 pack.mcmeta、存档 level.dat、
//! servers.dat、光影包语言文件、结构文件 NBT）；模组启用 / 禁用 / 删除按 uuid 定位后走
//! `ModObj` 方法；其余类型按「目录 + 纯文件名」直接操作（回收站）。图标在列表 DTO 里转
//! base64 data URL（条目少、体积小，不走图片协议）。

use std::path::{Path, PathBuf};

use mml_base::hash_helper;
use mml_game::game_mods::ModObj;
use mml_game::game_saves::SaveObj;
use mml_game::game_schematics::SchematicType;
use mml_game::launcher::instance_setting_obj::InstanceSettingObj;
use mml_game::GameInstance;
use mml_names::names;
use mml_sys::{open_helper, path_helper};
use uuid::Uuid;

use crate::dtos::{
    DataPackItemDto, ModItemDto, PackItemDto, SaveItemDto, ScreenshotItemDto, ServerItemDto,
    ShaderItemDto, SchematicItemDto,
};

/// 实例资源目录类别（open_folder 的 kind 入参）
const KIND_MODS: &str = "mods";
const KIND_RESOURCEPACKS: &str = "resourcepacks";
const KIND_SAVES: &str = "saves";
const KIND_SCREENSHOTS: &str = "screenshots";
const KIND_SHADERPACKS: &str = "shaderpacks";
const KIND_SCHEMATICS: &str = "schematics";
const KIND_SERVERS: &str = "servers";
const KIND_DATAPACKS: &str = "datapacks";

/// 解析实例 uuid
fn parse_instance(uuid: &str) -> Result<GameInstance, String> {
    let uuid = Uuid::parse_str(uuid).map_err(|_| "err.uuid".to_string())?;
    mml_game::get_instance(&uuid).ok_or_else(|| "err.gameNotFound".to_string())
}

/// 实例下的资源目录路径（锁内只取路径，立即释放）
fn instance_dir(instance: &GameInstance, kind: &str) -> Result<PathBuf, String> {
    let game = instance.read().unwrap();
    Ok(match kind {
        KIND_MODS => game.get_mods_path(),
        KIND_RESOURCEPACKS => game.get_resourcepacks_path(),
        KIND_SAVES => game.get_saves_path(),
        KIND_SCREENSHOTS => game.get_screenshots_path(),
        KIND_SHADERPACKS => game.get_shaderpacks_path(),
        KIND_SCHEMATICS => game.get_schematics_path(),
        KIND_SERVERS => game.get_game_path(),
        _ => return Err("err.fileTypeNotFound".to_string()),
    })
}

/// 目录 + 纯文件名定位一个资源文件（防路径穿越）
fn resource_file(instance: &GameInstance, kind: &str, name: &str) -> Result<PathBuf, String> {
    let dir = instance_dir(instance, kind)?;
    let name_path = Path::new(name);
    if name.is_empty() || name_path.file_name() != Some(name_path.as_os_str()) {
        return Err("err.fileName".to_string());
    }
    let file = dir.join(name_path);
    if !file.starts_with(&dir) {
        return Err("err.fileName".to_string());
    }
    Ok(file)
}

/// 图片字节转 data URL（无图返回空串）
fn data_url(bytes: Option<&Vec<u8>>) -> String {
    match bytes {
        Some(data) if !data.is_empty() => {
            format!("data:image/png;base64,{}", hash_helper::gen_base64_bytes(data))
        }
        _ => String::new(),
    }
}

/// 异步实例方法放阻塞线程执行的通用包装（锁卫不跨 await，future 保持 Send）
async fn block_on_instance<T: Send + 'static>(
    instance: GameInstance,
    f: impl FnOnce(&InstanceSettingObj) -> T + Send + 'static,
) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let game = instance.read().unwrap();
        f(&game)
    })
    .await
    .map_err(|err| err.to_string())
}

/// 获取实例的存档列表（异步），阻塞线程内执行
async fn load_saves(instance: GameInstance) -> Result<Vec<mml_game::game_saves::SaveObj>, String> {
    block_on_instance(instance, |game| {
        tokio::runtime::Handle::current()
            .block_on(async { game.get_saves().await })
    })
    .await
}

/// 按目录名（saves 下的目录）找存档
async fn find_save(instance: GameInstance, dir: &str) -> Result<SaveObj, String> {
    let dir = dir.to_string();
    let saves = load_saves(instance).await?;
    saves
        .into_iter()
        .find(|item| {
            item.path
                .file_name()
                .map(|n| n.to_string_lossy().to_string() == dir)
                .unwrap_or(false)
        })
        .ok_or_else(|| "err.saveNotFound".to_string())
}

// ==================== 模组 ====================

/// 模组列表（解析 jar 元数据，条目多时耗时数秒）
#[tauri::command]
pub async fn resource_list_mods(uuid: String) -> Result<Vec<ModItemDto>, String> {
    let instance = parse_instance(&uuid)?;

    let list = block_on_instance(instance, |game| {
        tokio::runtime::Handle::current()
            .block_on(async { game.read_mod(false).await })
    })
    .await?;

    Ok(list
        .iter()
        .filter_map(|item| {
            let file = item.file.file_name()?.to_string_lossy().to_string();
            if file.is_empty() {
                return None;
            }
            // 显示信息取第一个解析出的元数据条目；图标取任一非空的
            let info = item.info.first();
            let icon = item.info.iter().find_map(|i| i.icon.as_ref());
            Some(ModItemDto {
                uuid: item.uuid.to_string(),
                file,
                disable: item.disable,
                fail: item.fail,
                core: item.core,
                mod_id: info.map(|i| i.mod_id.clone()).unwrap_or_default(),
                name: info.map(|i| i.name.clone()).unwrap_or_default(),
                version: info.and_then(|i| i.version.clone()).unwrap_or_default(),
                author: info.map(|i| i.author.join(", ")).unwrap_or_default(),
                description: info.and_then(|i| i.description.clone()).unwrap_or_default(),
                icon: data_url(icon),
            })
        })
        .collect())
}

/// 在 mods 目录扫描结果里按 uuid 找模组（启用 / 禁用 / 删除需要带状态的 ModObj）
async fn find_mod(instance: GameInstance, mod_uuid: &str) -> Result<ModObj, String> {
    let list = block_on_instance(instance, |game| {
        tokio::runtime::Handle::current()
            .block_on(async { game.read_mod_fast().await })
    })
    .await?;

    list.into_iter()
        .find(|item| item.uuid.to_string() == mod_uuid)
        .ok_or_else(|| "err.fileNotFound".to_string())
}

/// 启用模组（去掉 .disable / .disabled 后缀）
#[tauri::command]
pub async fn resource_mod_enable(uuid: String, mod_uuid: String) -> Result<(), String> {
    let instance = parse_instance(&uuid)?;
    let obj = find_mod(instance, &mod_uuid).await?;
    obj.enable().map_err(|err| err.to_string())
}

/// 禁用模组（追加 .disable 后缀，已禁用或文件不存在时报错）
#[tauri::command]
pub async fn resource_mod_disable(uuid: String, mod_uuid: String) -> Result<(), String> {
    let instance = parse_instance(&uuid)?;
    let obj = find_mod(instance, &mod_uuid).await?;
    obj.disable().map_err(|err| err.to_string())
}

/// 删除模组（进回收站）
#[tauri::command]
pub async fn resource_delete_mod(uuid: String, mod_uuid: String) -> Result<(), String> {
    let instance = parse_instance(&uuid)?;
    let obj = find_mod(instance, &mod_uuid).await?;
    obj.delete().map_err(|err| err.to_string())
}

// ==================== 材质包 ====================

/// 材质包列表（解析 pack.mcmeta + 图标）
#[tauri::command]
pub async fn resource_list_resourcepacks(uuid: String) -> Result<Vec<PackItemDto>, String> {
    let instance = parse_instance(&uuid)?;

    let list = block_on_instance(instance, |game| {
        tokio::runtime::Handle::current()
            .block_on(async { game.get_resourcepacks().await })
    })
    .await?;

    Ok(list
        .iter()
        .filter_map(|item| {
            let file = item.path.file_name()?.to_string_lossy().to_string();
            if file.is_empty() {
                return None;
            }
            Some(PackItemDto {
                file,
                description: item.description.clone(),
                pack_format: item.pack_format,
                min_format: item.min_format,
                max_format: item.max_format,
                fail: item.fail,
                icon: data_url(item.icon.as_ref()),
            })
        })
        .collect())
}

/// 删除材质包（进回收站）
#[tauri::command]
pub async fn resource_delete_resourcepack(uuid: String, file: String) -> Result<(), String> {
    let instance = parse_instance(&uuid)?;
    let file = resource_file(&instance, KIND_RESOURCEPACKS, &file)?;
    path_helper::move_to_trash(&file).map_err(|err| err.to_string())
}

// ==================== 存档 ====================

/// 存档列表（解析 level.dat）
#[tauri::command]
pub async fn resource_list_saves(uuid: String) -> Result<Vec<SaveItemDto>, String> {
    let instance = parse_instance(&uuid)?;
    let list = load_saves(instance).await?;

    Ok(list
        .iter()
        .filter_map(|item| {
            let dir = item.path.file_name()?.to_string_lossy().to_string();
            if dir.is_empty() {
                return None;
            }
            // 图标是存档目录里的 png（SaveObj.icon 给了路径），小文件直接读
            let icon = item.icon.as_ref().and_then(|p| path_helper::read_byte(p).ok());
            Some(SaveItemDto {
                dir,
                level_name: item.level_name.clone(),
                last_played: item.last_played,
                game_type: item.game_type,
                hard_core: item.hard_core != 0,
                difficulty: item.difficulty,
                broken: item.broken,
                icon: data_url(icon.as_ref()),
            })
        })
        .collect())
}

/// 删除存档（进回收站）
#[tauri::command]
pub async fn resource_delete_save(uuid: String, dir: String) -> Result<(), String> {
    let instance = parse_instance(&uuid)?;
    let file = resource_file(&instance, KIND_SAVES, &dir)?;
    if !file.is_dir() {
        return Err("err.saveNotFound".to_string());
    }
    path_helper::move_to_trash(&file).map_err(|err| err.to_string())
}

/// 备份存档（zip 到实例备份目录并登记），返回备份文件名
#[tauri::command]
pub async fn resource_backup_save(uuid: String, dir: String) -> Result<String, String> {
    let instance = parse_instance(&uuid)?;

    // 备份期间压缩耗时较长：get_saves 拿到列表后锁只保护设置读写，
    // 压缩阶段持读锁（读读并发，只挡住同时改实例设置这种罕见操作）
    let name = tauri::async_runtime::spawn_blocking(move || -> Result<String, String> {
        let saves = tokio::runtime::Handle::current()
            .block_on(async { instance.read().unwrap().get_saves().await });
        let save = saves
            .iter()
            .find(|item| {
                item.path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string() == dir)
                    .unwrap_or(false)
            })
            .ok_or_else(|| "err.saveNotFound".to_string())?;

        let game = instance.read().unwrap();
        save.backup(&game, None).map_err(|err| err.to_string())?;
        Ok(save.level_name.clone())
    })
    .await
    .map_err(|err| err.to_string())??;

    Ok(name)
}

// ==================== 截图 ====================

/// 截图列表
#[tauri::command]
pub fn resource_list_screenshots(uuid: String) -> Result<Vec<ScreenshotItemDto>, String> {
    let instance = parse_instance(&uuid)?;
    let list = instance.read().unwrap().get_screenshots();

    Ok(list
        .iter()
        .map(|item| ScreenshotItemDto {
            name: item.name.clone(),
        })
        .collect())
}

/// 删除截图（进回收站）
#[tauri::command]
pub async fn resource_delete_screenshot(uuid: String, name: String) -> Result<(), String> {
    let instance = parse_instance(&uuid)?;
    let file = resource_file(&instance, KIND_SCREENSHOTS, &name)?;
    path_helper::move_to_trash(&file).map_err(|err| err.to_string())
}

/// 清空全部截图（进回收站，文件多时耗时）
#[tauri::command]
pub async fn resource_clear_screenshots(uuid: String) -> Result<(), String> {
    let instance = parse_instance(&uuid)?;
    tauri::async_runtime::spawn_blocking(move || {
        instance
            .read()
            .unwrap()
            .clear_screenshots()
            .map_err(|err| err.to_string())
    })
    .await
    .map_err(|err| err.to_string())?
}

// ==================== 服务器 ====================

/// 服务器列表（servers.dat，同步纯读）
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
#[tauri::command]
pub fn resource_server_delete(uuid: String, name: String, ip: String) -> Result<(), String> {
    let instance = parse_instance(&uuid)?;
    instance
        .read()
        .unwrap()
        .remove_server(&name, &ip)
        .map_err(|err| err.to_string())
}

// ==================== 光影包 ====================

/// 光影包列表（解析 zip 内语言文件），并按 options.txt 的 shaderPack 标出启用中的包
#[tauri::command]
pub async fn resource_list_shaderpacks(uuid: String) -> Result<Vec<ShaderItemDto>, String> {
    let instance = parse_instance(&uuid)?;

    block_on_instance(instance, |game| {
        let list = tokio::runtime::Handle::current()
            .block_on(async { game.get_shaderpacks().await });
        let selected = game
            .get_minecraft_options()
            .ok()
            .and_then(|opts| opts.get("shaderPack").cloned())
            .unwrap_or_default();
        (list, selected)
    })
    .await
    .map(|(list, selected)| {
        list.iter()
            .filter_map(|item| {
                let file = item.file.file_name()?.to_string_lossy().to_string();
                if file.is_empty() {
                    return None;
                }
                Some(ShaderItemDto {
                    selected: selected == file,
                    name: if item.name.is_empty() {
                        file.clone()
                    } else {
                        item.name.clone()
                    },
                    file,
                    comment: item.comment.clone(),
                })
            })
            .collect()
    })
}

/// 启用 / 停用光影包（写 options.txt 的 shaderPack 键；None = OFF 停用）
#[tauri::command]
pub async fn resource_shader_set(uuid: String, file: Option<String>) -> Result<(), String> {
    if let Some(file) = file.as_ref() {
        let path = Path::new(file);
        if file.is_empty() || path.file_name() != Some(path.as_os_str()) {
            return Err("err.fileName".to_string());
        }
    }
    let instance = parse_instance(&uuid)?;

    block_on_instance(instance, move |game| {
        let mut opts = game.get_minecraft_options().map_err(|err| err.to_string())?;
        opts.insert(
            "shaderPack".to_string(),
            file.unwrap_or_else(|| "OFF".to_string()),
        );
        game.save_minecraft_options(&opts)
            .map_err(|err| err.to_string())
    })
    .await?
}

/// 删除光影包（进回收站）
#[tauri::command]
pub async fn resource_delete_shaderpack(uuid: String, file: String) -> Result<(), String> {
    let instance = parse_instance(&uuid)?;
    let file = resource_file(&instance, KIND_SHADERPACKS, &file)?;
    path_helper::move_to_trash(&file).map_err(|err| err.to_string())
}

// ==================== 结构文件 ====================

/// 结构类型标签
fn schematic_type_name(schematic_type: &SchematicType) -> &'static str {
    match schematic_type {
        SchematicType::Minecraft => "Minecraft",
        SchematicType::Litematic => "Litematic",
        SchematicType::WorldEdit => "WorldEdit",
        SchematicType::Create => "Create",
    }
}

/// 结构文件列表（按扩展名解析 NBT）
#[tauri::command]
pub async fn resource_list_schematics(uuid: String) -> Result<Vec<SchematicItemDto>, String> {
    let instance = parse_instance(&uuid)?;

    let list = block_on_instance(instance, |game| {
        tokio::runtime::Handle::current()
            .block_on(async { game.get_schematics().await })
    })
    .await?;

    Ok(list
        .iter()
        .filter_map(|item| {
            let file = item.path.file_name()?.to_string_lossy().to_string();
            if file.is_empty() {
                return None;
            }
            Some(SchematicItemDto {
                type_name: schematic_type_name(&item.schematic_type).to_string(),
                file,
                name: item.name.clone(),
                author: item.author.clone(),
                description: item.description.clone(),
                width: item.width,
                height: item.height,
                length: item.length,
                block_count: item.block_count,
                block_types: item.block_types,
                fail: item.fail,
            })
        })
        .collect())
}

/// 删除结构文件（进回收站）
#[tauri::command]
pub async fn resource_delete_schematic(uuid: String, file: String) -> Result<(), String> {
    let instance = parse_instance(&uuid)?;
    let file = resource_file(&instance, KIND_SCHEMATICS, &file)?;
    path_helper::move_to_trash(&file).map_err(|err| err.to_string())
}

// ==================== 数据包（存档子页） ====================

/// 存档的数据包列表（get_datapacks 同步 + rayon 解 zip，放阻塞线程）
#[tauri::command]
pub async fn resource_list_datapacks(
    uuid: String,
    dir: String,
) -> Result<Vec<DataPackItemDto>, String> {
    let instance = parse_instance(&uuid)?;
    let save = find_save(instance, &dir).await?;

    let packs = tauri::async_runtime::spawn_blocking(move || {
        save.get_datapacks().map_err(|err| err.to_string())
    })
    .await
    .map_err(|err| err.to_string())??;

    Ok(packs
        .iter()
        .map(|item| DataPackItemDto {
            file: item
                .path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default(),
            name: item.name.clone(),
            description: item.description.clone(),
            pack_format: item.pack_format,
            enable: item.enable,
        })
        .collect())
}

/// 切换数据包启用状态（change_data_pack 对传入的包做状态翻转，传单个即 toggle）
#[tauri::command]
pub async fn resource_datapack_toggle(
    uuid: String,
    dir: String,
    name: String,
) -> Result<(), String> {
    let instance = parse_instance(&uuid)?;
    let save = find_save(instance, &dir).await?;

    tauri::async_runtime::spawn_blocking(move || -> Result<(), String> {
        let mut save = save;
        let packs = save.get_datapacks().map_err(|err| err.to_string())?;
        let pack = packs
            .into_iter()
            .find(|item| item.name.eq_ignore_ascii_case(&name))
            .ok_or_else(|| "err.fileNotFound".to_string())?;
        save.change_data_pack(&vec![pack])
            .map_err(|err| err.to_string())
    })
    .await
    .map_err(|err| err.to_string())?
}

/// 删除数据包（清 level.dat 引用后把文件 / 目录一并进回收站）
#[tauri::command]
pub async fn resource_datapack_delete(
    uuid: String,
    dir: String,
    name: String,
) -> Result<(), String> {
    let instance = parse_instance(&uuid)?;
    let save = find_save(instance, &dir).await?;

    tauri::async_runtime::spawn_blocking(move || -> Result<(), String> {
        let mut save = save;
        let packs = save.get_datapacks().map_err(|err| err.to_string())?;
        let pack = packs
            .into_iter()
            .find(|item| item.name.eq_ignore_ascii_case(&name))
            .ok_or_else(|| "err.fileNotFound".to_string())?;
        let path = pack.path.clone();
        save.delete_datapack(&vec![pack])
            .map_err(|err| err.to_string())?;
        path_helper::move_to_trash(&path).map_err(|err| err.to_string())
    })
    .await
    .map_err(|err| err.to_string())?
}

// ==================== 打开文件夹 ====================

/// 打开资源目录（name 为空打开目录本身，目录不存在则先创建；
/// 带 name 时资源管理器定位到该文件。datapacks 类别需要 parent = 存档目录名）
#[tauri::command]
pub fn resource_open_folder(
    uuid: String,
    kind: String,
    name: Option<String>,
    parent: Option<String>,
) -> Result<(), String> {
    let instance = parse_instance(&uuid)?;
    let dir = if kind == KIND_DATAPACKS {
        let parent = parent
            .as_deref()
            .filter(|p| !p.trim().is_empty())
            .ok_or_else(|| "err.fileName".to_string())?;
        let parent_path = Path::new(parent);
        if parent_path.file_name() != Some(parent_path.as_os_str()) {
            return Err("err.fileName".to_string());
        }
        let saves = instance.read().unwrap().get_saves_path();
        saves.join(parent_path).join(names::GAME_DATAPACK_DIR)
    } else {
        instance_dir(&instance, &kind)?
    };

    let target = match name.as_deref() {
        Some(name) if !name.trim().is_empty() => resource_file(&instance, &kind, name)?,
        _ => {
            if !dir.exists() {
                path_helper::create_dir_all(&dir).map_err(|err| err.to_string())?;
            }
            dir
        }
    };
    if !target.exists() {
        return Err("err.fileNotFound".to_string());
    }

    open_helper::open_file_with_explorer(&target);
    Ok(())
}
