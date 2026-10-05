//! 资源管理窗口：实例的模组 / 材质包 / 存档 / 截图 / 服务器 / 光影包 / 结构 / 数据包列表与操作
//!
//! 列表复用 mml-game 的扫描（模组元数据、材质包 pack.mcmeta、存档 level.dat、
//! servers.dat、光影包语言文件、结构文件 NBT）；模组启用 / 禁用 / 删除按 uuid 定位后走
//! `ModObj` 方法；其余类型按「目录 + 纯文件名」直接操作（回收站）。图标在列表 DTO 里转
//! base64 data URL（条目少、体积小，不走图片协议）。

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use mml_base::file_item::FileHash;
use mml_base::hash_helper;
use mml_game::GameInstance;
use mml_game::game_mods::{LoadSideType, ModObj};
use mml_game::game_saves::SaveObj;
use mml_game::game_schematics::SchematicType;
use mml_game::gui_hook::{IProgressGui, ProgressGui};
use mml_game::launcher::instance_setting_obj::InstanceSettingObj;
use mml_names::names;
use mml_sys::{open_helper, path_helper};
use tauri::{Emitter, WebviewWindow};
use uuid::Uuid;

use crate::dtos::{
    DataPackItemDto, ModGroupDto, ModItemDto, ModScanProgressDto, PackItemDto, ResourceViewDto,
    SaveItemDto, SchematicItemDto, ScreenshotItemDto, ServerItemDto, ShaderItemDto,
};
use crate::listens;

/// 实例资源目录类别（open_folder 的 kind 入参）
const KIND_MODS: &str = "mods";
/// 材质包目录
const KIND_RESOURCEPACKS: &str = "resourcepacks";
/// 存档目录
const KIND_SAVES: &str = "saves";
/// 截图目录
const KIND_SCREENSHOTS: &str = "screenshots";
/// 光影包目录
const KIND_SHADERPACKS: &str = "shaderpacks";
/// 结构文件目录
const KIND_SCHEMATICS: &str = "schematics";
/// 服务器（打开游戏根目录）
const KIND_SERVERS: &str = "servers";
/// 数据包（存档子页，需配合 parent = 存档目录名）
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
        Some(data) if !data.is_empty() => format!(
            "data:{};base64,{}",
            image_mime(data),
            hash_helper::gen_base64_bytes(data)
        ),
        _ => String::new(),
    }
}

/// 按魔术字节认图片 MIME（认不出的按 `image/png`）
///
/// 以前一律写死 `image/png`，但模组 logo 里 JPG / GIF 很常见：声明错了
/// 只能靠浏览器嗅探兜底（WebView2 会兜，但不该指望）。
fn image_mime(data: &[u8]) -> &'static str {
    if data.starts_with(&[0xFF, 0xD8, 0xFF]) {
        "image/jpeg"
    } else if data.starts_with(b"GIF87a") || data.starts_with(b"GIF89a") {
        "image/gif"
    } else if data.starts_with(b"BM") {
        "image/bmp"
    } else if data.len() >= 12 && data.starts_with(b"RIFF") && &data[8..12] == b"WEBP" {
        "image/webp"
    } else {
        "image/png"
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
        tokio::runtime::Handle::current().block_on(async { game.get_saves().await })
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

/// 模组扫描进度回调：把 `x/x` 直接 emit 给发起扫描的那个窗口
///
/// 与方块渲染的 `BlockRenderGui`（windows/block.rs）同一套做法：实现
/// [`IProgressGui`]，内核在扫描过程中回调，桌面壳负责转成前端事件。
struct ModScanGui {
    window: WebviewWindow,
}

impl IProgressGui for ModScanGui {
    /// 扫描用不到进度文字，忽略（trait 要求实现）
    fn set_progress_text(&self, _text: Option<String>) {}

    /// 每解析完一个 jar 报一次 `(已完成, 总数)` → 转发为事件
    fn set_progress_now(&self, value: usize, all: Option<usize>) {
        emit_resource_list_mods_progress(
            &self.window,
            ModScanProgressDto {
                done: value,
                total: all.unwrap_or(0),
            },
        );
    }
}

/// 模组扫描进度事件（`resource-list-mods-progress`）
///
/// 解析一个 jar 要读元数据 / 图标 / 扫 class，几百个包要好几秒 ——
/// 界面靠它显示 `x/x`，否则用户面对空列表不知道是在跑还是卡住了。
#[gui_macros::emit]
pub fn emit_resource_list_mods_progress(window: &WebviewWindow, dto: ModScanProgressDto) {
    let _ = window.emit(listens::RESOURCE_LIST_MODS_PROGRESS, dto);
}

/// 模组列表（解析 jar 元数据，条目多时耗时数秒）
///
/// `window`：扫描进度（`x/x`）发到**发起这次扫描的那个窗口**（资源窗口）。
/// 不用 `app.emit` 全局广播：主窗口等也会收到，白跑一遍状态更新。
#[tauri::command]
pub async fn resource_list_mods(
    window: WebviewWindow,
    uuid: String,
) -> Result<Vec<ModItemDto>, String> {
    let instance = parse_instance(&uuid)?;

    // 进度回调交给内核（与方块渲染同一套 gui_hook 接口）；
    // 回调是同步的，直接 emit，不需要额外缓冲
    let gui: ProgressGui = Some(Arc::new(ModScanGui {
        window: window.clone(),
    }) as Arc<dyn IProgressGui>);

    // 四份附带数据都在这一步一起读，省掉前端额外的往返与按文件名 / SHA1 的合并：
    // - 备注：`guisetting.json` 的 `Mod.ModName`
    // - 在线信息：实例的 `online_info.json`（下载源 / 项目编号 / 文件编号）
    // - 游戏根目录：把绝对路径裁成 `.minecraft\mods\xxx.jar` 这种相对路径
    let (list, notes, online, game_path) = block_on_instance(instance, move |game| {
        let notes = crate::gui_setting::load(game).mods.mod_name;
        let online = game.read_online_info();
        let game_path = game.get_game_path();
        let list = tokio::runtime::Handle::current()
            .block_on(async { game.read_mod(false, gui).await });
        (list, notes, online, game_path)
    })
    .await?;

    // 按 SHA1 索引：在线信息表的主键是 SHA1（见 OnlineInfoObj），模组的 hash 正好是它
    let online_of: HashMap<String, mml_game::launcher::file_online_info_obj::OnlineInfoObj> = online
        .into_iter()
        .filter(|(_, info)| !info.sha1.is_empty())
        .map(|(_, info)| (info.sha1.clone(), info))
        .collect();

    let ctx = ModCtx {
        notes: &notes,
        online: &online_of,
        game_path: &game_path,
    };

    Ok(list.iter().filter_map(|item| mod_item(item, &ctx)).collect())
}

/// `mod_item` 需要的几份外部数据（打包传，省得一路加参数）
struct ModCtx<'a> {
    /// `guisetting.json` 的 `Mod.ModName`（文件名 → 说明）
    notes: &'a HashMap<String, Option<String>>,
    /// 实例的在线信息表，按 **SHA1** 索引
    online: &'a HashMap<String, mml_game::launcher::file_online_info_obj::OnlineInfoObj>,
    /// 实例的游戏根目录（`.minecraft`），用来算相对路径
    game_path: &'a Path,
}

/// 一个（可能带内置模组的）模组条目 → DTO
///
/// 顶层条目取文件名；内置模组（`jar_in_jar`）没有独立文件，用显示名兜底 ——
/// 它们装在父 jar 里，只用于展示（列表里缩进一层），不提供启用 / 删除。
fn mod_item(item: &ModObj, ctx: &ModCtx) -> Option<ModItemDto> {
    let file = item
        .file
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_default();
    // 顶层条目必须有文件名（拿不到就说明不是 mods 目录下的文件，跳过）；
    // 内置模组 file 为空，靠 modid / 名字展示
    let info = item.info.first();
    let mod_id = info.map(|i| i.mod_id.clone()).unwrap_or_default();
    let name = info.map(|i| i.name.clone()).unwrap_or_default();
    if file.is_empty() && mod_id.is_empty() && name.is_empty() {
        return None;
    }
    let icon = item.info.iter().find_map(|i| i.icon.as_ref());

    // 在线信息按 SHA1 关联：手动放进 mods 的模组不在表里，那三列就空着
    let sha1 = sha1_of(&item.hash);
    let online = ctx.online.get(&sha1);
    let project_id = online.map(|i| i.modid.clone()).unwrap_or_default();
    let file_id = online.map(|i| i.fileid.clone()).unwrap_or_default();
    // 下载源由项目号 + 文件号的形态判定（见 mml_game::launcher::get_source_type）：
    // 两个都没有就是"不是从平台下的"，显示空串而不是硬判成 CurseForge
    let source = if project_id.is_empty() && file_id.is_empty() {
        String::new()
    } else {
        mml_game::launcher::get_source_type(&project_id, &file_id).to_string()
    };

    Some(ModItemDto {
        uuid: item.uuid.to_string(),
        sha1,
        path: mod_rel_path(&item.file, ctx.game_path),
        disable: item.disable,
        fail: item.fail,
        core: item.core,
        mod_id,
        name,
        version: info.and_then(|i| i.version.clone()).unwrap_or_default(),
        author: info.map(|i| i.author.join(", ")).unwrap_or_default(),
        description: info.and_then(|i| i.description.clone()).unwrap_or_default(),
        loader: info.map(|i| i.loaders.to_string()).unwrap_or_default().to_string(),
        side: mod_side_name(info.map(|i| i.side)),
        url: info.and_then(|i| i.url.clone()).unwrap_or_default(),
        source,
        project_id,
        file_id,
        note: mod_note(ctx.notes, &file),
        icon: data_url(icon),
        jar_in_jar: item
            .jar_in_jar
            .iter()
            .filter_map(|child| mod_item(child, ctx))
            .collect(),
        file,
    })
}

/// 相对实例的路径：`.minecraft\mods\sodium.jar` 这种形态
///
/// 与游戏根目录（`.minecraft`）做前缀裁剪：整条绝对路径太长（`E:\…\instances\xxx\…`），
/// 而"游戏目录下的哪一层"这点信息是必要的（mods / config 之类）。
/// 裁不掉（不在游戏目录下）时退回完整路径，至少还能看。
fn mod_rel_path(file: &Path, game_path: &Path) -> String {
    let rel = file.strip_prefix(game_path).unwrap_or(file);
    rel.to_string_lossy().to_string()
}

/// 加载侧的名字（前端按 i18n 显示，这里只出稳定枚举名）
fn mod_side_name(side: Option<LoadSideType>) -> String {
    match side {
        Some(LoadSideType::Client) => "client",
        Some(LoadSideType::Server) => "server",
        Some(LoadSideType::Both) => "both",
        _ => "unknown",
    }
    .to_string()
}

/// 备注的定位键：**去掉禁用后缀**的模组文件名
///
/// `Mod.ModName` 的键是文件名（与 ColorMC 互通），而启用 / 禁用只给文件名加减
/// `.disabled`（见 `mml_game::game_mods::add_disable_suffix`）—— 不归一的话
/// "禁用一下就找不到自己的备注了"。
fn mod_note_key(file: &str) -> &str {
    file.strip_suffix(names::DISABLE_DOT_EXT)
        .or_else(|| file.strip_suffix(names::DISABLED_DOT_EXT))
        .unwrap_or(file)
}

/// 取某个模组的备注（没有返回空串）
///
/// 先按归一后的键找，找不到再按**原始**文件名找一次：ColorMC 在禁用状态下写的备注
/// 就存在带后缀的那个键上，别让它读不出来。
fn mod_note(notes: &HashMap<String, Option<String>>, file: &str) -> String {
    if file.is_empty() {
        return String::new();
    }
    notes
        .get(mod_note_key(file))
        .or_else(|| notes.get(file))
        .and_then(|value| value.clone())
        .unwrap_or_default()
}

/// 取文件 SHA1（没有哈希时返回空串）
///
/// 模组的自定义分组按 SHA1 记（与 `guisetting.json` 的 `Mod.Groups` 一致）：
/// 它是**内容哈希**，启用 / 禁用（改文件名）之后不变，而 uuid 会跟着路径变。
fn sha1_of(hash: &FileHash) -> String {
    match hash {
        FileHash::Sha1(value) | FileHash::Sha1Sha256(value, _) | FileHash::Sha1Sha512(value, _) => {
            value.clone()
        }
        _ => String::new(),
    }
}

/// 在 mods 目录扫描结果里按 uuid 找模组（启用 / 禁用 / 删除需要带状态的 ModObj）
///
/// 不报进度（传 `None`）：这是"改一个文件前后的内部查找"，不面向用户，
/// 报了反而会让界面上的进度指示闪一下。
async fn find_mod(instance: GameInstance, mod_uuid: &str) -> Result<ModObj, String> {
    let list = block_on_instance(instance, |game| {
        tokio::runtime::Handle::current().block_on(async { game.read_mod_fast(None).await })
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
        tokio::runtime::Handle::current().block_on(async { game.get_resourcepacks().await })
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
            let icon = item
                .icon
                .as_ref()
                .and_then(|p| path_helper::read_byte(p).ok());
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
        let list =
            tokio::runtime::Handle::current().block_on(async { game.get_shaderpacks().await });
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
        let mut opts = game
            .get_minecraft_options()
            .map_err(|err| err.to_string())?;
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
        tokio::runtime::Handle::current().block_on(async { game.get_schematics().await })
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

// ==================== 模组自定义分组 ====================
//
// 分组**不是这里的数据**：它属于实例的 GUI 设置（`guisetting.json` 的 `Mod.Groups`，
// 分组名 → 模组 SHA1 集合，见 crate::gui_setting），与 ColorMC 互通。
// 这里只做"读-改-写 + 转 DTO"：
// - 每个命令都是 load → 改 → save（那份文件还存着日志设置、方块图标等，不能整份覆盖）
// - 分组顺序 = 用户自己拖出来的（存在 `guisetting.json` 的 `Mod.GroupOrder`；
//   没存过则用默认顺序：识别失败 → 已启用 → 已禁用 → 自建分组按名字）
// - 成员用 SHA1 而不是 uuid：启用/禁用会改文件名，uuid 跟着变，SHA1 不变

/// 三个**状态分组**的固定 uuid
///
/// 它们不是用户数据，所以不进 `guisetting.json` 的 `Mod.Groups`；但要参与"顺序"与
/// "收起状态"（用户能拖、能折叠），所以需要**稳定的键**。用固定 uuid 而不是
/// 原先的 `$on` / `$off` / `$fail` 字符串：
/// - 与自建分组同一套键形状（都是 uuid），前端不必维护"两套键"的映射；
/// - 建组时不可能撞上（用户分组是 `Uuid::new_v4()`）；
/// - 看起来就是分组，不再是一串带 `$` 的魔法字符串。
///
/// 取值刻意用"全 0 / 尾号 1 / 尾号 2"这种一眼能认出的形式（与内核里
/// `DEFAULT_GROUP_UUID = Uuid::nil()` 同一套思路）：调试时看到
/// `00000000-…-000000000001` 就知道是状态分组，不用去查表。
///
/// **一旦发布就不能改** —— 改了等于所有用户的状态分组顺序与折叠状态重置。
/// 前端 `resource/composables/useModGroups.ts` 的 `STATE_GROUP_ID_*` 与此一一对应，
/// 改要一起改。
const STATE_GROUP_FAIL: &str = "00000000-0000-0000-0000-000000000000";
const STATE_GROUP_ON: &str = "00000000-0000-0000-0000-000000000001";
const STATE_GROUP_OFF: &str = "00000000-0000-0000-0000-000000000002";

/// 默认顺序：识别失败 → 已启用 → 已禁用，自建分组排在后面
///
/// 「识别失败」放最前是**用户点名的**：坏包要第一时间看见；其余按"启用 → 禁用"。
fn default_group_order() -> Vec<String> {
    vec![
        STATE_GROUP_FAIL.to_string(),
        STATE_GROUP_ON.to_string(),
        STATE_GROUP_OFF.to_string(),
    ]
}

/// 是不是三个状态分组之一
fn is_state_group(key: &str) -> bool {
    key == STATE_GROUP_ON || key == STATE_GROUP_OFF || key == STATE_GROUP_FAIL
}

/// 分组块的完整顺序：状态分组 + 自建分组，**与真实存在的分组对齐**
///
/// 入参是 `GameModSettingObj`（模组设置本体，也就是 `GameGuiSettingObj::mods`），
/// 不是整份 `guisetting.json`。
///
/// 存下来的顺序可能过时（分组删了 / 换了台机器 / 手改过文件），所以这里以"当前真实存在
/// 的分组"为准做一次规范化：丢掉不存在的、补上没记的（自建分组按名字排在后面）。
fn group_order_of(mods: &crate::gui_setting::GameModSettingObj) -> Vec<String> {
    // 自建分组：按名字排序当兜底顺序（HashMap 无序，总要有个确定的补位规则）
    let mut custom: Vec<(String, String)> = mods
        .groups
        .iter()
        .map(|(uuid, group)| (group.name.clone(), uuid.clone()))
        .collect();
    custom.sort();

    let mut order: Vec<String> = Vec::with_capacity(custom.len() + 3);
    let mut seen: HashSet<String> = HashSet::new();
    for key in mods.group_order.iter() {
        // 状态分组照收（它们不在 Groups 里）；自建分组只认当前存在的那些
        let known = is_state_group(key) || mods.groups.contains_key(key);
        if known && seen.insert(key.clone()) {
            order.push(key.clone());
        }
    }
    for key in default_group_order() {
        if seen.insert(key.clone()) {
            order.push(key);
        }
    }
    for (_, uuid) in custom {
        if seen.insert(uuid.clone()) {
            order.push(uuid);
        }
    }
    order
}

/// 模组的自定义分组（**按用户拖出来的顺序**下发）
///
/// 模组的自定义分组（**按用户拖出来的顺序**下发）
///
/// 顺序取自 [`group_order_of`]：它同时管自建分组与状态分组的排列，
/// 所以这里按它遍历、跳过状态分组（它们不是用户数据，前端自己按固定 uuid 拼）即可。
fn mod_groups_of(instance: &InstanceSettingObj) -> Vec<ModGroupDto> {
    let setting = crate::gui_setting::load(instance);
    let mut list: Vec<ModGroupDto> = group_order_of(&setting.mods)
        .into_iter()
        .filter(|key| !is_state_group(key))
        .filter_map(|uuid| {
            let group = setting.mods.groups.get(&uuid)?;
            let mut mods: Vec<String> = group.mods.iter().cloned().collect();
            // HashSet 迭代顺序不定，排一下让前端展示稳定
            mods.sort();
            Some(ModGroupDto {
                uuid,
                name: group.name.clone(),
                mods,
            })
        })
        .collect();
    list.shrink_to_fit();
    list
}

/// 读出实例设置 → 交给 `edit` 改 → 存回去
///
/// 注意 `guisetting.json` 里除分组外还存着备注、日志设置、方块图标等，
/// 所以每个命令都是 load → 改 → save，**不能整份覆盖**。
fn edit_mod_setting(
    instance: &GameInstance,
    edit: impl FnOnce(&mut crate::gui_setting::GameModSettingObj),
) {
    let game = instance.read().unwrap();
    let mut setting = crate::gui_setting::load(&game);
    edit(&mut setting.mods);
    crate::gui_setting::save(&game, &setting);
}

/// 取某个实例的模组分组
#[tauri::command]
pub fn resource_mod_groups(uuid: String) -> Result<Vec<ModGroupDto>, String> {
    let instance = parse_instance(&uuid)?;
    let game = instance.read().unwrap();

    Ok(mod_groups_of(&game))
}

/// 取**收起**的分组块键集合（与 `resource_mod_groups` 一起在进模组页时读）
///
/// 单独一条命令而不是塞进 `ModGroupDto`：那一份是"分组 → 成员"的数据结构，
/// 收起状态是窗口级的视图状态，混在一起会让 DTO 的语义变浑。
#[tauri::command]
pub fn resource_mod_groups_collapsed(uuid: String) -> Result<Vec<String>, String> {
    let instance = parse_instance(&uuid)?;
    let mods = crate::gui_setting::load(&instance.read().unwrap()).mods;

    // 按当前存在的分组过滤：分组删了之后，它残留的键不该再冒出来
    let known: HashSet<String> = group_order_of(&mods).into_iter().collect();
    Ok(collapsed_of(&mods)
        .into_iter()
        .filter(|key| known.contains(key))
        .collect())
}

/// 收起的分组键：没写过就用初值（「已启用」默认收起），写过就照用户存的来
///
/// 为什么不直接把初值写进 `GameModSettingObj::default`：那个默认值只在**整块 `Mod`
/// 字段缺失**时生效；老文件有 `Mod`、只是没有 `GroupCollapsed`，走的是字段级默认
/// （空表 = 全展开），用户会觉得"我明明收起过"。所以这里按"有没有写过"分情况。
fn collapsed_of(mods: &crate::gui_setting::GameModSettingObj) -> Vec<String> {
    mods.group_collapsed
        .clone()
        .unwrap_or_else(|| vec![STATE_GROUP_ON.to_string()])
}

// ==================== 资源窗口的视图偏好 ====================
//
// 跟**实例**走的那部分界面设置（左侧分类顺序 / 上次类别 / 模组展示方式），
// 存在实例的 `guisetting.json`（`Gui` 字段，见 crate::gui_setting::GameViewSettingObj）。
// 不做成前端本地存储：换个实例就该换一套，本地存储是"每台机器一份"，
// 两处口径不同会出现"切了实例顺序却没变"。

/// 取某实例的资源窗口视图偏好
///
/// 返回的是**原样存下来的值**（可能是空数组 / 空串），排序与默认值由前端补 ——
/// 后端不认识有哪几个分类，硬编码一份的话新增分类就得改两处。
#[tauri::command]
pub fn resource_view_get(uuid: String) -> Result<ResourceViewDto, String> {
    let instance = parse_instance(&uuid)?;
    let view = crate::gui_setting::load(&instance.read().unwrap()).view;

    Ok(ResourceViewDto {
        order: view.resource_order,
        category: view.resource_category,
        mod_view: view.resource_mod_view,
    })
}

/// 保存某实例的资源窗口视图偏好（整份覆盖；未传的项保持原值）
#[tauri::command]
pub fn resource_view_set(
    uuid: String,
    order: Vec<String>,
    category: String,
    mod_view: String,
) -> Result<(), String> {
    let instance = parse_instance(&uuid)?;

    // 与其它 guisetting 写入一样：load → 改这一小块 → 存回整份
    // （那份文件还存着日志设置、模组分组、方块图标，不能整份覆盖）
    let game = instance.read().unwrap();
    let mut setting = crate::gui_setting::load(&game);
    setting.view.resource_order = order;
    setting.view.resource_category = category;
    setting.view.resource_mod_view = mod_view;
    crate::gui_setting::save(&game, &setting);

    Ok(())
}

/// 新建模组分组（重名返回错误，前端提示）
///
/// **返回新建分组的 uuid**：前端拿它拼顺序表 / 折叠集合，也用它继续操作这个分组
/// （不再像以前那样靠"组名"间接指代）。
#[tauri::command]
pub fn resource_mod_group_add(uuid: String, name: String) -> Result<String, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err(String::from("err.groupEmpty"));
    }
    let instance = parse_instance(&uuid)?;
    if mod_groups_of(&instance.read().unwrap())
        .iter()
        .any(|item| item.name == name)
    {
        return Err(String::from("err.groupExists"));
    }

    // uuid v4：分组身份与名字解耦（改名不动键）
    let group_uuid = Uuid::new_v4().to_string();
    let key = group_uuid.clone();
    edit_mod_setting(&instance, |mods| {
        mods.groups.insert(
            key.clone(),
            crate::gui_setting::GameModGroupObj {
                name: name.to_string(),
                mods: Default::default(),
            },
        );
    });

    Ok(group_uuid)
}

/// 删除模组分组（组内模组回到"未分组"，磁盘上的文件一个都不动）
#[tauri::command]
pub fn resource_mod_group_remove(uuid: String, group: String) {
    let Ok(instance) = parse_instance(&uuid) else {
        return;
    };
    edit_mod_setting(&instance, |mods| {
        mods.groups.remove(&group);
        // 顺序表 / 折叠集合里的这个 uuid 一起去掉：留着就是脏数据
        mods.group_order.retain(|key| key != &group);
        if let Some(collapsed) = mods.group_collapsed.as_mut() {
            collapsed.retain(|key| key != &group);
        }
    });
}

/// 重命名模组分组（重名返回错误；**只改名字，键与成员都不动**）
#[tauri::command]
pub fn resource_mod_group_rename(
    uuid: String,
    group: String,
    new_name: String,
) -> Result<(), String> {
    let new_name = new_name.trim();
    if new_name.is_empty() {
        return Err(String::from("err.groupEmpty"));
    }
    let instance = parse_instance(&uuid)?;
    // 重名检查要排除它自己（改回原名 / 只改大小写不该报"已存在"）
    if mod_groups_of(&instance.read().unwrap())
        .iter()
        .any(|item| item.uuid != group && item.name == new_name)
    {
        return Err(String::from("err.groupExists"));
    }

    edit_mod_setting(&instance, |mods| {
        if let Some(target) = mods.groups.get_mut(&group) {
            target.name = new_name.to_string();
        }
    });

    Ok(())
}

/// 保存分组块的显示顺序（用户拖出来的）
///
/// `order` 是**分组 uuid**（状态分组用 [`STATE_GROUP_ON`] 那几个固定 uuid）——
/// 与 [`group_order_of`] 同一套口径。接进来之后先规范化一次再落盘：前端可能因为
/// 版本差异多传 / 少传了键，存脏数据的话下次读出来还得再纠一遍。
#[tauri::command]
pub fn resource_mod_group_order_set(uuid: String, order: Vec<String>) {
    let Ok(instance) = parse_instance(&uuid) else {
        return;
    };

    edit_mod_setting(&instance, |mods| {
        // 借当前的分组表跑一遍规范化再落盘，免得把过时的键存进去
        let order = order.clone();
        let probe = crate::gui_setting::GameModSettingObj {
            groups: mods.groups.clone(),
            mod_name: Default::default(),
            group_order: order,
            group_collapsed: Default::default(),
        };
        mods.group_order = group_order_of(&probe);
    });
}

/// 保存**收起**的分组块键集合（用户点分组头折叠出来的）
///
/// `collapsed` 的键与 [`resource_mod_group_order_set`] 同一套口径（分组 uuid）。
/// 与顺序一样按当前真实存在的分组过滤一遍：分组删了以后，它的键不该留在文件里。
#[tauri::command]
pub fn resource_mod_group_collapsed_set(uuid: String, collapsed: Vec<String>) {
    let Ok(instance) = parse_instance(&uuid) else {
        return;
    };

    edit_mod_setting(&instance, |mods| {
        let known: HashSet<String> = group_order_of(mods).into_iter().collect();
        let mut kept: Vec<String> = collapsed
            .into_iter()
            .filter(|key| known.contains(key))
            .collect();
        kept.dedup();
        // 存 Some：哪怕是空数组也代表"用户明确展开了全部"，
        // 不能写回 None（那会让下次读出来又是"已启用默认收起"）
        mods.group_collapsed = Some(kept);
    });
}

/// 把若干模组移到某个分组；`group` 为空 / null = 移出所有分组（回到"未分组"）
///
/// 移动语义：先从其它组里摘掉，再进目标组 —— 一个模组同时只属于一个组。
#[tauri::command]
pub fn resource_mod_group_set(uuid: String, group: Option<String>, keys: Vec<String>) {
    let Ok(instance) = parse_instance(&uuid) else {
        return;
    };
    let group = group
        .as_deref()
        .map(str::trim)
        .filter(|key| !key.is_empty())
        .map(str::to_string);

    // 目标组不存在就直接返回（别把它们从原组摘出来之后无处可去）；
    // 先判再改，省掉一次没有改动的写盘
    if let Some(key) = &group {
        let exists = crate::gui_setting::load(&instance.read().unwrap())
            .mods
            .groups
            .contains_key(key);
        if !exists {
            return;
        }
    }

    edit_mod_setting(&instance, |mods| {
        for (key, target) in mods.groups.iter_mut() {
            if Some(key) == group.as_ref() {
                continue;
            }
            for sha1 in &keys {
                target.mods.remove(sha1);
            }
        }

        if let Some(key) = &group {
            if let Some(target) = mods.groups.get_mut(key) {
                for sha1 in &keys {
                    target.mods.insert(sha1.clone());
                }
            }
        }
    });
}

/// 写某个模组的备注（传空串 = 删掉这条备注）
///
/// `file` 传列表里的**原始文件名**（可能带 `.disabled`）：落盘时按 [`mod_note_key`]
/// 归一成"启用时的文件名"，这样启用 / 禁用来回切，备注都跟着走。
///
/// 归一前后的键都写不到旧值时，顺手把原始文件名下的那条一起清掉 ——
/// 否则 ColorMC 在禁用状态下写的备注会和新写的并存，"看着改了其实没改"。
#[tauri::command]
pub fn resource_mod_note_set(uuid: String, file: String, note: String) {
    let Ok(instance) = parse_instance(&uuid) else {
        return;
    };
    let note = note.trim().to_string();
    let key = mod_note_key(&file).to_string();
    if key.is_empty() {
        return;
    }

    edit_mod_setting(&instance, |mods| {
        mods.mod_name.remove(&file);
        if note.is_empty() {
            mods.mod_name.remove(&key);
        } else {
            mods.mod_name.insert(key, Some(note));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::{image_mime, mod_note, mod_note_key};
    use std::collections::HashMap;

    /// 按魔术字节认 MIME；认不出来的一律 png（PNG 自己的魔数也走这条）
    #[test]
    fn test_image_mime() {
        assert_eq!(
            image_mime(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]),
            "image/png"
        );
        assert_eq!(image_mime(&[0xFF, 0xD8, 0xFF, 0xE0]), "image/jpeg");
        assert_eq!(image_mime(b"GIF89a..."), "image/gif");
        assert_eq!(image_mime(b"GIF87a..."), "image/gif");
        assert_eq!(image_mime(b"BM\x00\x00"), "image/bmp");
        assert_eq!(image_mime(b"RIFF\x00\x00\x00\x00WEBPVP8 "), "image/webp");
        // 太短 / 认不出来：退回 png，不 panic
        assert_eq!(image_mime(b""), "image/png");
        assert_eq!(image_mime(b"RIFF"), "image/png");
        assert_eq!(image_mime(b"\x00\x01\x02\x03"), "image/png");
    }

    /// 备注的键：禁用后缀要剥掉（启用 / 禁用来回切时备注才是同一份）
    #[test]
    fn test_mod_note_key() {
        assert_eq!(mod_note_key("sodium.jar"), "sodium.jar");
        assert_eq!(mod_note_key("sodium.jar.disabled"), "sodium.jar");
        assert_eq!(mod_note_key("sodium.jar.disable"), "sodium.jar");
        // 名字里本来就带 disabled 的不许误伤（只认结尾）
        assert_eq!(mod_note_key("disabled.jar"), "disabled.jar");
    }

    /// 读备注：归一键优先，退化到原始文件名（ColorMC 在禁用状态下写的）
    #[test]
    fn test_mod_note() {
        let mut notes: HashMap<String, Option<String>> = HashMap::new();
        notes.insert("sodium.jar".into(), Some("优化".into()));
        assert_eq!(mod_note(&notes, "sodium.jar"), "优化");
        assert_eq!(mod_note(&notes, "sodium.jar.disabled"), "优化");

        // 只有带后缀那个键（别的启动器写的）也能读出来
        let mut legacy: HashMap<String, Option<String>> = HashMap::new();
        legacy.insert("old.jar.disabled".into(), Some("旧备注".into()));
        assert_eq!(mod_note(&legacy, "old.jar.disabled"), "旧备注");

        // 没有 / 值为 None / 文件名为空：都返回空串，不 panic
        assert_eq!(mod_note(&notes, "missing.jar"), "");
        assert_eq!(mod_note(&notes, ""), "");
        let mut empty: HashMap<String, Option<String>> = HashMap::new();
        empty.insert("none.jar".into(), None);
        assert_eq!(mod_note(&empty, "none.jar"), "");
    }
}
