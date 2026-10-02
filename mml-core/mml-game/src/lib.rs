//! 游戏实例管理模块
//!
//! 本 crate 是启动器的游戏实例层，负责实例的创建 / 管理 / 启动，以及
//! 实例内各类资源（模组、资源包、光影包、存档、截图、结构文件等）的读写。
//!
//! # 核心概念
//!
//! - **实例** — 一个可启动的 Minecraft 游戏环境（版本 + 加载器 + 独立游戏目录），
//!   对应 [`launcher::instance_setting_obj::InstanceSettingObj`]，
//!   全局以 [`GameInstance`]（`Arc<RwLock<...>>`）形式共享
//! - **分组** — 实例的分组展示，组名到实例 UUID 的映射保存在内存
//! - **运行句柄** — 启动后的实例由 [`game_launch::InstanceHandle`] 跟踪，
//!   后台线程每秒轮询退出状态并触发 [`InstanceExit`] 事件
//! - **事件** — 实例退出 / 变更 / 运行日志分别通过
//!   [`add_exit`] / [`add_change`] / [`add_run_log`] 订阅
//!
//! # 子模块
//!
//! | 模块 | 用途 |
//! |------|------|
//! | [`add_game`] | 添加游戏实例（导入版本 / 文件夹 / 压缩包） |
//! | [`class_scan`] | 模组 Side 扫描（1.12.2 时代 mod 的 client/server 判定） |
//! | [`curseforge`] | CurseForge 整合包下载安装 |
//! | [`data_res`] | 实例内资源下载位置与类型切换（资源包 / 光影包 / 存档 / 数据包） |
//! | [`game_arg`] | 启动参数生成 |
//! | [`game_check`] | 实例文件校验 |
//! | [`game_count`] | 启动与游戏时长统计 |
//! | [`game_export`] | 实例导出 |
//! | [`game_lan`] | 局域网联机 |
//! | [`game_launch`] | 游戏实例启动 |
//! | [`game_libraries`] | 实例运行库处理 |
//! | [`game_log`] | 实例日志（运行日志与过往日志） |
//! | [`game_mods`] | 实例模组管理 |
//! | [`game_options`] | options.txt 配置读写 |
//! | [`game_resourcepacks`] | 实例资源包管理 |
//! | [`game_saves`] | 实例存档管理（含存档内数据包） |
//! | [`game_schematics`] | 结构文件读取 |
//! | [`game_screenshots`] | 实例截图管理 |
//! | [`game_server`] | 实例服务器管理 |
//! | [`game_shaderpacks`] | 实例光影包管理 |
//! | [`gui_hook`] | GUI 回调钩子（启动 / 安装进度等界面交互） |
//! | [`launcher`] | 实例设置与运行配置 |
//! | [`launcher_path`] | 各类目录路径管理（版本 / 库 / 资源 / 实例） |
//! | [`loader`] | 加载器安装（Forge / Fabric / Quilt / OptiFine / LiteLoader / 自定义） |
//! | [`modpack`] | 整合包安装框架（Modrinth / CurseForge worker） |
//! | [`modrinth`] | Modrinth 整合包下载安装 |
//! | [`mojang`] | Mojang 版本数据处理（版本清单 / 下载项） |
//! | [`other_launcher`] | 其他启动器实例导入（HMCL / PCL / 官方 / MMC） |
//! | [`path_watch`] | 实例目录文件监视 |
//! | [`player_skin`] | 玩家皮肤获取 |
//! | [`scan_game`] | 扫描游戏版本 |
//! | [`serverpack`] | 服务端整合包生成 |

use std::{
    collections::{HashMap, VecDeque},
    path::{Path, PathBuf},
    sync::{Arc, LazyLock, RwLock},
    thread,
    time::Duration,
};

use mml_base::events::EventArgHandler;
use mml_names::{
    i18_items::error_type::{ArgEmptyData, CoreResult, ErrorType, FileSystemErrorData},
    names,
};

use mml_net::input_file::InputFile;
use mml_sys::{Os, path_helper};
use uuid::Uuid;

use crate::{
    game_launch::InstanceHandle,
    game_log::{GameLog, GameLogItemObj, InstanceRuntimeLog},
    gui_hook::{AddInstanceGui, ProgressGui},
    launcher::{LogEncoding, game_time_obj::GameTimeObj, instance_setting_obj::InstanceSettingObj},
    launcher_path::{
        assets_path,
        instance_path::{self},
        version_path,
    },
};

pub mod add_game;
pub mod class_scan;
pub mod curseforge;
pub mod data_res;
pub mod game_arg;
pub mod game_check;
pub mod game_count;
pub mod game_export;
pub mod game_lan;
pub mod game_launch;
pub mod game_libraries;
pub mod game_log;
pub mod game_mods;
pub mod game_options;
pub mod game_motd;
pub mod game_resourcepacks;
pub mod game_saves;
pub mod game_schematics;
pub mod game_screenshots;
pub mod game_server;
pub mod game_shaderpacks;
pub mod gui_hook;
pub mod launcher;
pub mod launcher_path;
pub mod loader;
pub mod modpack;
pub mod modrinth;
pub mod mojang;
pub mod other_launcher;
pub mod path_watch;
pub mod player_skin;
pub mod scan_game;
pub mod serverpack;

/// 实例共享句柄（全局以 `Arc<RwLock>` 形式持有实例设置）
pub type GameInstance = Arc<RwLock<InstanceSettingObj>>;

/// 实例结束运行事件
pub struct InstanceExit {
    /// 实例 UUID
    pub uuid: Uuid,
    /// 进程退出码
    pub code: i32,
}

/// 实例修改事件参数
pub enum InstanceChange {
    /// 添加实例
    AddInstance(Uuid),
    /// 删除实例
    RemoveInstance(Uuid),
    /// 移动分组
    MoveGroup(Uuid, Option<String>),
}

/// 实例日志事件参数
pub enum InstanceLogType {
    /// 新增一条日志
    AddLog(GameLogItemObj),
    /// 清空日志
    ClearLog,
}

/// 实例日志事件
pub struct InstanceLog {
    /// 实例 UUID
    pub uuid: Uuid,
    /// 日志内容
    pub log: InstanceLogType,
}

/// 保存的运行日志
static RUNTIME_LOGS: LazyLock<RwLock<HashMap<Uuid, InstanceRuntimeLog>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

/// 保存的实例句柄
static HANDELS: LazyLock<RwLock<HashMap<Uuid, InstanceHandle>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

/// 分组信息
static GROUPS: LazyLock<RwLock<HashMap<String, Vec<Uuid>>>> = LazyLock::new(|| {
    let mut group = HashMap::new();
    group.insert(names::DEFAULT_GROUP.to_string(), Vec::new());
    RwLock::new(group)
});

/// 实例列表
static INSTANCES: LazyLock<RwLock<HashMap<Uuid, GameInstance>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

/// 实例退出事件
static EXIT_EVENT: LazyLock<EventArgHandler<InstanceExit>> =
    LazyLock::new(|| EventArgHandler::new());
/// 实例修改事件
static CHANGE_EVENT: LazyLock<EventArgHandler<InstanceChange>> =
    LazyLock::new(|| EventArgHandler::new());
/// 实例运行日志事件
static LOG_EVENT: LazyLock<EventArgHandler<InstanceLog>> = LazyLock::new(|| EventArgHandler::new());

/// 订阅实例退出事件
///
/// - `handler`: 事件回调
///
/// # 返回值
///
/// 返回回调 ID（`remove_exit` 用）
pub fn add_exit<F>(handler: F) -> u64
where
    F: Fn(&InstanceExit) + Send + Sync + 'static,
{
    EXIT_EVENT.add_handler(handler)
}

/// 订阅实例变更事件
///
/// - `handler`: 事件回调
///
/// # 返回值
///
/// 返回回调 ID（`remove_change` 用）
pub fn add_change<F>(handler: F) -> u64
where
    F: Fn(&InstanceChange) + Send + Sync + 'static,
{
    CHANGE_EVENT.add_handler(handler)
}

/// 订阅实例运行日志事件
///
/// - `handler`: 事件回调
///
/// # 返回值
///
/// 返回回调 ID（`remove_run_log` 用）
pub fn add_run_log<F>(handler: F) -> u64
where
    F: Fn(&InstanceLog) + Send + Sync + 'static,
{
    LOG_EVENT.add_handler(handler)
}

/// 取消订阅实例退出事件
///
/// - `id`: `add_exit` 返回的回调 ID
pub fn remove_exit(id: u64) {
    EXIT_EVENT.remove_handel(id);
}

/// 取消订阅实例变更事件
///
/// - `id`: `add_change` 返回的回调 ID
pub fn remove_change(id: u64) {
    CHANGE_EVENT.remove_handel(id);
}

/// 取消订阅实例运行日志事件
///
/// - `id`: `add_run_log` 返回的回调 ID
pub fn remove_run_log(id: u64) {
    LOG_EVENT.remove_handel(id);
}

pub(crate) fn invoke_exit(uuid: Uuid, code: i32) {
    EXIT_EVENT.emit(InstanceExit { uuid, code });
}

pub(crate) fn invoke_change(change: InstanceChange) {
    CHANGE_EVENT.emit(change);
}

pub(crate) fn invoke_run_log(uuid: Uuid, log: InstanceLogType) {
    LOG_EVENT.emit(InstanceLog { uuid, log });
}

/// 初始化
/// - `dir`: 运行路径
///
/// # 返回值
///
/// 成功返回 `Ok(())`；路径初始化失败返回对应错误
pub fn init<P: AsRef<Path>>(dir: P) -> CoreResult<()> {
    launcher_path::init(dir)
}

/// 开始加载数据
///
/// 加载版本目录、启动目录监视线程与实例退出轮询线程，
/// 并把已存在的实例目录载入实例列表。
///
/// # 返回值
///
/// 成功返回 `Ok(())`；实例目录加载失败返回对应错误
pub fn load() -> CoreResult<()> {
    version_path::load();

    path_watch::init_watch()?;

    thread::spawn(|| {
        loop {
            let to_remove: Vec<(Uuid, i32)> = {
                let handels = HANDELS.read().unwrap();
                handels
                    .iter()
                    .filter_map(|(k, v)| {
                        v.tick();
                        if v.is_exit() {
                            Some((*k, v.code()))
                        } else {
                            None
                        }
                    })
                    .collect()
            };

            if !to_remove.is_empty() {
                {
                    let mut handels = HANDELS.write().unwrap();
                    for (uuid, _) in &to_remove {
                        handels.remove(uuid);
                    }
                }
                for (uuid, code) in &to_remove {
                    invoke_exit(*uuid, *code);
                }
            }

            thread::sleep(Duration::from_secs(1));
        }
    });

    let installs = instance_path::load_instance_dir()?;

    for item in installs {
        add_to_group(item);
    }

    Ok(())
}

/// 获取所有实例
///
/// # 返回值
///
/// 返回实例列表
pub fn get_instances() -> Vec<GameInstance> {
    let mut list = Vec::new();

    for (_, value) in INSTANCES.read().unwrap().iter() {
        list.push(value.clone());
    }

    list
}

/// 从uuid获取实例
///
/// - `uuid`: 实例 UUID
///
/// # 返回值
///
/// 返回实例；不存在返回 `None`
pub fn get_instance(uuid: &Uuid) -> Option<GameInstance> {
    let list = INSTANCES.read().unwrap();

    Some(list.get(uuid)?.clone())
}

/// 游戏内语言（语言代码 + 显示名）
pub struct InstanceLangObj {
    /// 语言代码：资源索引里 `minecraft/lang/<code>.json` 的 `<code>`，如 `zh_cn`
    pub code: String,
    /// 显示名：该语言文件里的 `language.name`（如“简体中文”）；读不到时为语言代码
    pub name: String,
}

/// 获取实例的游戏内语言列表（从资源索引里查 minecraft/lang/*.json）
///
/// 每项的显示名就写在该语言文件里（`language.name`，如 zh_cn → “简体中文”），
/// 与游戏内语言菜单同一份数据；资源没下全 / 文件里没有这个键时回落为语言代码本身。
///
/// 资源索引未下载 / 版本数据缺失时返回空列表，由前端回退默认语言（中文 / 英文）。
///
/// - `uuid`: 实例 UUID
///
/// # 返回值
///
/// 返回按语言代码排序的语言列表（代码 + 显示名）
pub fn get_instance_langs(uuid: &Uuid) -> Vec<InstanceLangObj> {
    let Some(instance) = get_instance(uuid) else {
        return Vec::new();
    };
    let version = instance.read().unwrap().version.clone();
    let Ok(obj) = version_path::get_version(&version) else {
        return Vec::new();
    };
    let Some(index) = &obj.asset_index else {
        return Vec::new();
    };
    let Ok(assets) = assets_path::get_index(index) else {
        return Vec::new();
    };
    let mut langs: Vec<InstanceLangObj> = assets
        .objects
        .iter()
        .filter_map(|(key, item)| {
            let code = key
                .strip_prefix(names::LANG_KEY1)?
                .strip_suffix(".json")?
                .to_string();
            let name = read_lang_name(&item.hash, &code).unwrap_or_else(|| code.clone());
            Some(InstanceLangObj { code, name })
        })
        .collect();
    langs.sort_by(|a, b| a.code.cmp(&b.code));
    langs
}

/// 已解析出的语言显示名缓存（语言代码 → 显示名）
///
/// 打开一次实例设置要把上百个语言文件的 `language.name` 读出来（单个 500KB 上下），
/// 而这份名单在所有实例之间是共用的 —— 读到的名字缓存起来，之后再开就直接命中。
/// **只缓存真读到名字的项**：回落成代码的不进缓存，等资源下全了还能再补上。
static LANG_NAMES: LazyLock<RwLock<HashMap<String, String>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

/// 从资源对象里读出语言文件的显示名（`language.name`）
///
/// 资源对象按 sha1 前两位分目录存放。**读不到就静默返回 None**，不走
/// [`assets_path::read_assets_text`] —— 那个失败时会记一条错误日志，而这里一次要读上百个
/// 语言文件，资源没下全就会把日志刷满。
///
/// - `hash`: 语言文件在资源索引里的 sha1
///
/// # 返回值
///
/// 返回显示名；两处都读不到 / 解析失败 / 没有该键返回 `None`
fn read_lang_name(hash: &str, code: &str) -> Option<String> {
    if let Some(name) = LANG_NAMES.read().unwrap().get(code) {
        return Some(name.clone());
    }

    // 1) 游戏资源对象（`assets/…/objects/<sha1>`）—— 与游戏内语言菜单同一来源，
    //    资源下全时最权威；
    // 2) 本地语言文件缓存 `langs/<code>.json`：mml-tex-draw 从客户端 jar 提取 /
    //    按资源索引下载的那份（方块与物品名单也用它）。**玩家还没启动过游戏时
    //    资源对象是空的**，就得靠它 —— 否则下拉里只会看到一堆 zh_cn 这样的代码。
    let name = read_object_lang_name(hash).or_else(|| read_cached_lang_name(code))?;
    LANG_NAMES
        .write()
        .unwrap()
        .insert(code.to_string(), name.clone());
    Some(name)
}

/// 从游戏资源对象里读语言名
///
/// 对象按 sha1 前两位分目录存放。**读不到就静默返回 None**，不走
/// [`assets_path::read_assets_text`] —— 那个失败时会记一条错误日志，而这里一次要读上百个
/// 语言文件，资源没下全就会把日志刷满。
fn read_object_lang_name(hash: &str) -> Option<String> {
    if hash.len() < 2 {
        return None;
    }
    let file = assets_path::get_obj_dir().join(&hash[..2]).join(hash);
    lang_name_from_file(&file)
}

/// 从 `langs/<code>.json`（mml-tex-draw 的语言文件缓存）里读语言名
fn read_cached_lang_name(code: &str) -> Option<String> {
    let file = mml_base::get_base_dir()
        .join(names::LANG_DIR)
        .join(format!("{code}.json"));
    lang_name_from_file(&file)
}

/// 读一个语言 json 里的 `language.name`
///
/// - `file`: 语言文件路径
///
/// # 返回值
///
/// 返回显示名；文件不存在 / 解析失败 / 没有该键返回 `None`
fn lang_name_from_file(file: &Path) -> Option<String> {
    if !file.is_file() {
        return None;
    }
    let text = path_helper::read_text(file).ok()?;
    // 按 Value 解析而不是直接 HashMap<String, String>：语言文件里万一有个非字符串的值，
    // 整份解析就废了 —— 这里只关心 language.name 一项，别被别处带崩
    let map = mml_base::serialize_tools::json_from_str::<HashMap<String, serde_json::Value>>(&text)
        .ok()?;
    map.get(names::LANG_NAME_KEY)
        .and_then(|item| item.as_str())
        .map(String::from)
}

/// 获取所有分组名字
///
/// # 返回值
///
/// 返回分组名列表
pub fn get_group_keys() -> Vec<String> {
    let mut list = Vec::new();

    for (key, _) in GROUPS.read().unwrap().iter() {
        list.push(key.clone());
    }

    list
}

/// 获取游戏版本类型列表（Mojang 版本清单的 type 字段取值）
///
/// 返回独立 ID：release / snapshot / old_beta / old_alpha，
/// 显示名由前端 i18n 翻译。
///
/// # 返回值
///
/// 返回版本类型 ID 列表
pub fn get_version_types() -> Vec<&'static str> {
    vec!["release", "snapshot", "old_beta", "old_alpha"]
}

/// 从分组名字获取对应的实例
/// - `key`: 分组名字
///
/// # 返回值
///
/// 返回该分组下的实例列表
pub fn get_group(key: &str) -> Vec<GameInstance> {
    let mut list = Vec::new();

    let group = GROUPS.read().unwrap();
    if let Some(group) = group.get(key) {
        for item in group {
            if let Some(instance) = get_instance(item) {
                list.push(instance.clone());
            }
        }
    }

    list
}

/// 添加分组
/// - `name`: 分组名
///
/// # 返回值
///
/// 添加成功返回 `true`；分组已存在返回 `false`
pub fn add_group(name: &str) -> bool {
    let mut groups = GROUPS.write().unwrap();
    if groups.contains_key(name) {
        false
    } else {
        groups.insert(name.to_string(), Vec::new());
        true
    }
}

/// 删除分组
/// - `name`: 分组名
///
/// # 返回值
///
/// 删除成功返回 `true`（组内实例移入默认分组）；分组不存在返回 `false`
pub fn remove_group(name: &str) -> bool {
    let mut groups = GROUPS.write().unwrap();
    let items = groups.remove(name);
    match items {
        Some(items) => {
            let def = groups.get_mut(names::DEFAULT_GROUP).unwrap();
            def.extend(items);
            true
        }
        None => false,
    }
}

/// 移动分组
/// - `list`: 需要移动的列表
/// - `new`: 新分组名字
///
/// # 返回值
///
/// 无返回值；移动完成后保存各实例并发 `MoveGroup` 事件
pub fn move_group(list: Vec<Uuid>, new: Option<String>) {
    let mut changes: Vec<(Uuid, Option<String>)> = Vec::new();

    {
        let mut groups = GROUPS.write().unwrap();
        let instances = INSTANCES.read().unwrap();

        for item in list.iter() {
            let game_arc = match instances.get(item) {
                Some(game) => game.clone(),
                None => continue,
            };

            let mut game = game_arc.write().unwrap();

            if let Some(name) = &game.group {
                let group = groups.get_mut(name);
                if let Some(group) = group {
                    group.retain(|g| g != &game.uuid);
                }
            } else {
                let group = groups.get_mut(names::DEFAULT_GROUP).unwrap();
                group.retain(|g| g != &game.uuid);
            }

            match new {
                Some(ref name) => {
                    if !groups.contains_key(name) {
                        groups.insert(name.to_string(), Vec::new());
                    }
                    let group = groups.get_mut(name).unwrap();
                    group.push(game.uuid);
                }
                None => {
                    let group = groups.get_mut(names::DEFAULT_GROUP).unwrap();
                    group.push(game.uuid);
                }
            }

            game.group = new.clone();
            changes.push((game.uuid, game.group.clone()));
        }
    }

    for (uuid, group) in changes {
        if let Some(instance) = get_instance(&uuid) {
            instance.write().unwrap().save();
        }
        invoke_change(InstanceChange::MoveGroup(uuid, group));
    }
}

/// 从实例名字获取实例
/// - `name`: 实例名字
///
/// # 返回值
///
/// 返回实例；不存在返回 `None`
pub fn get_instance_by_name(name: &str) -> Option<GameInstance> {
    let list = INSTANCES.read().unwrap();
    let temp = list
        .iter()
        .filter(|(_, value)| value.read().unwrap().name.eq_ignore_ascii_case(name))
        .next()?;

    Some(temp.1.clone())
}

/// 是否存在这个名字的实例
/// - `name`: 实例名字
///
/// # 返回值
///
/// 存在返回 `true`
pub fn have_instance_name(name: &str) -> bool {
    let list = INSTANCES.read().unwrap();
    let temp = list
        .iter()
        .filter(|(_, value)| value.read().unwrap().name.eq_ignore_ascii_case(name))
        .next();
    !temp.is_none()
}

/// 是否存在这个名字的实例
/// - `uuid`: 实例标识
///
/// # 返回值
///
/// 存在返回 `true`
pub fn have_instance_uuid(uuid: &Uuid) -> bool {
    let list = INSTANCES.read().unwrap();
    list.contains_key(uuid)
}

/// 将实例添加到分组中
///
/// UUID 为空或冲突时自动重新生成。
///
/// - `obj`: 实例设置
///
/// # 返回值
///
/// 返回加入列表后的实例句柄
fn add_to_group(mut obj: InstanceSettingObj) -> GameInstance {
    while obj.uuid.is_nil() || matches!(get_instance(&obj.uuid), Some(_)) {
        obj.uuid = Uuid::new_v4();
    }

    obj.save();
    let key = obj.uuid.clone();
    let group = obj.group.clone();
    let game: Arc<RwLock<InstanceSettingObj>> = Arc::new(RwLock::new(obj));

    let mut groups = GROUPS.write().unwrap();
    if let Some(ref g) = group {
        if !groups.contains_key(g) {
            groups.insert(g.clone(), Vec::new());
        }
    } else {
        if !groups.contains_key(names::DEFAULT_GROUP) {
            groups.insert(names::DEFAULT_GROUP.to_string(), Vec::new());
        }
    }

    let mut instances = INSTANCES.write().unwrap();
    instances.insert(key, game.clone());

    // ---- 更新分组列表 ----
    if let Some(ref g) = group {
        if let Some(list) = groups.get_mut(g) {
            list.push(key);
        }
    } else {
        if let Some(list) = groups.get_mut(names::DEFAULT_GROUP) {
            list.push(key);
        }
    }

    drop(groups);
    drop(instances);

    invoke_change(InstanceChange::AddInstance(key));
    game
}

/// 删除实例
///
/// - `uuid`: 实例 UUID
///
/// # 返回值
///
/// 成功返回 `Ok(())`（实例文件移入回收站）；实例不存在或删除失败返回对应错误
pub fn delete_instance(uuid: &Uuid) -> CoreResult<()> {
    let instance = get_instance(uuid).ok_or(ErrorType::ArgEmpty(ArgEmptyData::UUID))?;

    // 先删实例文件（回收站），失败则保留实例数据，避免 UI 消失但文件还在
    instance.read().unwrap().delete_files()?;

    remove_instance_record(uuid);
    Ok(())
}

/// 从实例列表移除记录（不删除文件，实例目录监视用）
///
/// # 参数
///
/// - `uuid`: 实例 UUID
pub(crate) fn remove_instance_record(uuid: &Uuid) {
    {
        let mut groups = GROUPS.write().unwrap();
        let mut instances = INSTANCES.write().unwrap();

        // 从所有分组中移除 uuid
        for (_, list) in groups.iter_mut() {
            list.retain(|&u| u != *uuid);
        }
        groups.retain(|_, list| !list.is_empty());
        instances.remove(uuid);
    }

    invoke_change(InstanceChange::RemoveInstance(*uuid));
}

/// 重命名实例（名字与实例目录一起改）
///
/// 新名字与其他实例重复时拒绝；成功后发 `MoveGroup` 事件刷新前端列表。
///
/// - `uuid`: 实例 UUID
/// - `name`: 新名字
///
/// # 返回值
///
/// 成功返回 `Ok(())`；名字为空 / 重复或目录改名失败返回对应错误
pub fn rename_instance(uuid: &Uuid, name: &str) -> CoreResult<()> {
    let name = name.trim();
    if name.is_empty() {
        return Err(ErrorType::ArgEmpty(ArgEmptyData::Name));
    }

    let instance = get_instance(uuid).ok_or(ErrorType::ArgEmpty(ArgEmptyData::UUID))?;

    // 新名字与别的实例重复（自己保持原名除外）则拒绝
    if have_instance_name(name) && instance.read().unwrap().name != name {
        return Err(ErrorType::InstanceNameExists(name.to_string()));
    }

    let old_base = instance.read().unwrap().get_base_path();

    // 实例目录跟着改名（目录名由名字派生），改名成功后再更新名字并保存
    let new_dir = path_helper::replace_path_name(name);
    let new_base = old_base
        .parent()
        .map(|p| p.join(&new_dir))
        .unwrap_or_else(|| old_base.clone());
    if old_base != new_base && old_base.exists() {
        std::fs::rename(&old_base, &new_base).map_err(|e| {
            ErrorType::FileSystemError(FileSystemErrorData {
                path: old_base,
                error: e.to_string(),
            })
        })?;
    }

    {
        let mut obj = instance.write().unwrap();
        obj.name = name.to_string();
        obj.dir = new_dir;
        obj.save();
    }

    invoke_change(InstanceChange::MoveGroup(*uuid, instance.read().unwrap().group.clone()));
    Ok(())
}

/// 添加运行日志
///
/// - `uuid`: 实例 UUID
/// - `data`: 日志原文
pub(crate) fn add_game_log(uuid: &Uuid, data: &str) {
    let mut logs = RUNTIME_LOGS.write().unwrap();
    if let Some(log) = logs.get_mut(uuid) {
        log.add_game_log(&data);
    } else {
        let mut log = InstanceRuntimeLog::new();
        let item = log.add_game_log(&data);

        logs.insert(uuid.clone(), log);

        invoke_run_log(uuid.clone(), InstanceLogType::AddLog(item));
    }
}

/// 添加运行日志
///
/// - `uuid`: 实例 UUID
/// - `data`: 已解析的日志项
pub(crate) fn add_game_log_item(uuid: &Uuid, data: GameLog) {
    let mut logs = RUNTIME_LOGS.write().unwrap();
    if let Some(log) = logs.get_mut(uuid) {
        log.add_log_item(data);
    } else {
        let mut log = InstanceRuntimeLog::new();
        let item = log.add_log_item(data);

        logs.insert(uuid.clone(), log);
        invoke_run_log(uuid.clone(), InstanceLogType::AddLog(item));
    }
}

/// 清理日志
///
/// - `uuid`: 实例 UUID
pub(crate) fn clear_game_log(uuid: &Uuid) {
    let mut logs = RUNTIME_LOGS.write().unwrap();
    if let Some(log) = logs.get_mut(uuid) {
        log.clear();
    } else {
        let log = InstanceRuntimeLog::new();
        logs.insert(uuid.clone(), log);
    }

    invoke_run_log(uuid.clone(), InstanceLogType::ClearLog);
}

/// 添加启动的游戏实例
///
/// - `handel`: 实例运行句柄
pub(crate) fn add_run_game(handel: InstanceHandle) {
    let mut games = HANDELS.write().unwrap();
    games.insert(handel.uuid, handel);
}

/// 获取正在运行的实例UUID列表
///
/// # 返回值
///
/// 返回运行中实例的 UUID 列表
pub fn get_running_instances() -> Vec<Uuid> {
    HANDELS.read().unwrap().keys().cloned().collect()
}

/// 判断实例是否正在运行
///
/// - `uuid`: 实例 UUID
///
/// # 返回值
///
/// 正在运行返回 `true`
pub fn is_running(uuid: &Uuid) -> bool {
    HANDELS.read().unwrap().contains_key(uuid)
}

/// 强制结束正在运行的实例
///
/// - `uuid`: 实例 UUID
pub fn stop_game(uuid: &Uuid) {
    if let Some(handle) = HANDELS.read().unwrap().get(uuid) {
        handle.kill();
    }
}

impl InstanceSettingObj {
    /// 创建实例
    ///
    /// 重名实例经 GUI 询问覆盖或改名；创建实例目录结构并保存设置。
    ///
    /// - `gui`: 添加实例界面回调（无界面时重名实例自动改名）
    ///
    /// # 返回值
    ///
    /// 返回新实例句柄；名字为空或用户取消返回对应错误
    pub async fn create_instance(mut self, gui: AddInstanceGui) -> CoreResult<GameInstance> {
        path_watch::stop_watch();

        let old = get_instance_by_name(&self.name);
        // 是否覆盖重名实例（无界面 / 用户拒绝覆盖时改为自动改名，保留原实例）
        let mut overwrite = false;
        if let Some(instance) = &old {
            if let Some(gui) = &gui {
                overwrite = gui.overwrite(instance.clone()).await;
                if !overwrite && !gui.name_replace(&self.name).await {
                    return Err(ErrorType::TaskCancel);
                }
            }

            if !overwrite {
                let mut a = 1;
                let mut name = format!("{}{a}", self.name);
                while have_instance_name(&name) {
                    a += 1;
                    name = format!("{}{a}", self.name);
                }

                self.name = name;
            }
        }

        if self.name.is_empty() {
            return Err(ErrorType::ArgEmpty(ArgEmptyData::Name));
        }

        // 只在用户选择覆盖时才删除重名的原实例
        if overwrite {
            if let Some(instance) = old {
                let uuid = {
                    let r = instance.read().unwrap();
                    r.uuid
                };

                delete_instance(&uuid)?;
            }
        }

        self.dir = path_helper::replace_path_name(&self.name);

        let dir = self.get_base_path();
        if dir.exists() {
            path_helper::move_to_trash(&dir)?
        }

        path_helper::create_dir_all(dir)?;
        path_helper::create_dir_all(self.get_game_path())?;
        path_helper::create_dir_all(self.get_mods_path())?;
        path_helper::create_dir_all(self.get_config_path())?;
        path_helper::create_dir_all(self.get_logs_path())?;
        path_helper::create_dir_all(self.get_saves_path())?;
        path_helper::create_dir_all(self.get_resourcepacks_path())?;

        self.save_online_info(&HashMap::new());
        self.save_launch_count_data(&GameTimeObj::new());

        if mml_sys::get_system_info().os == Os::Windows {
            self.encoding = LogEncoding::GBK;
        }

        self.save();

        path_watch::start_watch();

        Ok(add_to_group(self))
    }

    /// 删除实例文件
    ///
    /// # 返回值
    ///
    /// 成功返回 `Ok(())`（实例目录移入回收站）；失败返回对应错误
    pub fn delete_files(&self) -> CoreResult<()> {
        path_helper::move_to_trash(self.get_base_path())
    }

    /// 复制实例设置为新的对象。
    ///
    /// `InstanceSettingObj` 刻意不实现 `Clone`，避免被随意整对象复制；
    /// 仅在确有需要构建一个独立新对象时（如复制实例）显式调用本方法。
    fn copy_self(&self) -> Self {
        Self {
            uuid: self.uuid,
            name: self.name.clone(),
            group: self.group.clone(),
            dir: self.dir.clone(),
            version: self.version.clone(),
            loader: self.loader,
            loader_version: self.loader_version.clone(),
            jvm_arg: self.jvm_arg.clone(),
            jvm_name: self.jvm_name.clone(),
            jvm_local: self.jvm_local.clone(),
            window: self.window.clone(),
            start_server: self.start_server.clone(),
            proxy_host: self.proxy_host.clone(),
            advance_jvm: self.advance_jvm.clone(),
            is_modpack: self.is_modpack,
            modpack_type: self.modpack_type,
            game_type: self.game_type,
            pid: self.pid.clone(),
            fid: self.fid.clone(),
            icon: self.icon.clone(),
            server_url: self.server_url.clone(),
            custom_loader: self.custom_loader.clone(),
            encoding: self.encoding,
        }
    }

    /// 复制数据到新的实例
    /// - `name`: 新的实例名字
    /// - `gui`: 添加实例界面回调
    ///
    /// # 返回值
    ///
    /// 返回新实例句柄
    pub async fn copy_to_other(&self, name: &str, gui: AddInstanceGui) -> CoreResult<GameInstance> {
        let mut instance = self.copy_self();
        instance.name = name.to_string();
        let instance = instance.create_instance(gui).await?;

        let online = self.read_online_info();
        let custom = self.read_custom_json();

        let read = instance.read().unwrap();
        read.save_custom_json(&custom)?;
        read.save_online_info(&online);

        Ok(instance.clone())
    }

    /// 更新在线文件信息
    ///
    /// 清理掉磁盘上已不存在的文件记录（`.jar.disabled` 的视作仍存在）。
    pub async fn update_online(&self) {
        let mut online = self.read_online_info();
        let dir = self.get_game_path();
        online.retain(|_, value| {
            let file = dir.join(&value.path).join(&value.file);
            if file.exists() {
                false
            } else {
                if let Some(ext) = file.extension()
                    && ext.eq_ignore_ascii_case(names::JAR_EXT)
                {
                    let disabled_file =
                        PathBuf::from(format!("{}{}", file.display(), names::DISABLED_DOT_EXT));
                    if disabled_file.exists() {
                        return false;
                    }
                }
                true
            }
        });

        self.save_online_info(&online);
    }

    /// 将文件复制到其他地方
    ///
    /// - `path`: 目标目录
    /// - `skip`: 需要跳过的相对路径列表
    /// - `is_base`: `true` 复制实例整个目录，`false` 只复制游戏目录
    /// - `gui`: 进度界面回调
    ///
    /// # 返回值
    ///
    /// 成功返回 `Ok(())`；复制失败返回对应错误
    pub async fn copy_files<P: AsRef<Path>>(
        &self,
        path: P,
        skip: Option<Vec<PathBuf>>,
        is_base: bool,
        gui: ProgressGui,
    ) -> CoreResult<()> {
        let dir = if is_base {
            self.get_base_path()
        } else {
            self.get_game_path()
        };

        path_helper::create_dir_all(path.as_ref())?;

        let mut index = 0usize;
        let list = path_helper::get_all_files(&dir);
        if let Some(gui) = &gui {
            gui.set_progress_now(index, Some(list.len()));
        }
        for item in list.iter() {
            let file = item.strip_prefix(&dir).map_err(|err| {
                ErrorType::FileSystemError(FileSystemErrorData {
                    path: item.clone(),
                    error: err.to_string(),
                })
            })?;

            if let Some(skip) = &skip {
                if skip.contains(&file.to_path_buf()) {
                    index += 1;
                    if let Some(gui) = &gui {
                        gui.set_progress_now(index, Some(list.len()));
                    }
                    continue;
                }
            }

            if let Some(gui) = &gui {
                gui.set_progress_text(Some(item.display().to_string()));
            }

            let now = path.as_ref().join(file);
            path_helper::copy_file_async(item, &now).await?;
        }

        Ok(())
    }

    /// 写入实例图标文件，并把实际写进去的文件名记回实例
    ///
    /// 故意不做成 `&mut self` 方法：那样调用方就得在 `.await` 期间持有实例写锁，
    /// 而实例锁的守卫不是 `Send`，在 `#[async_trait]`（要求 `future: Send`）里编译不过。
    /// 这里自己控制加锁范围 —— 只在锁内问出"图标写到哪个文件"，写盘时不持锁。
    ///
    /// 调用方也不要带着实例锁进来（`std` 的 `RwLock` 不可重入，会死锁）。
    ///
    /// # 参数
    ///
    /// - `game`: 目标实例
    /// - `icon`: 图标输入源
    ///
    /// # 返回值
    ///
    /// 成功返回 `Ok(())`；写盘失败返回对应错误
    pub async fn save_icon(game: &GameInstance, icon: InputFile) -> CoreResult<()> {
        let (file, name) = {
            let game = game.read().unwrap();
            let file = game.get_icon_file();
            // 字段原本可能指向别的文件名，先按实际路径记下来，写盘成功后原样写回
            let name = file
                .file_name()
                .map(|item| item.to_string_lossy().into_owned());
            (file, name)
        };

        icon.save_file(file).await?;

        let mut game = game.write().unwrap();
        game.icon = name;
        game.save();

        Ok(())
    }

    /// 获取运行中的日志
    ///
    /// # 返回值
    ///
    /// 返回当前运行日志快照；没有运行日志返回 `None`
    pub fn get_runtime_log(&self) -> Option<Arc<VecDeque<GameLogItemObj>>> {
        let logs = RUNTIME_LOGS.read().unwrap();
        logs.get(&self.uuid)
            .map(|log| log.logs.read().unwrap().clone())
    }
}
