//! 实例 GUI 设置（guisetting.json）
//!
//! **归属 GUI，不属于内核**：日志窗口开关、模组分组与备注、方块背景、启动后自开日志窗、
//! 排列顺序 —— 这些都是"界面怎么看"，与启动流程无关，所以内核不读也不写它
//! （`InstanceSettingObj` 里没有对应字段，内核只提供 `get_gui_setting_file()` 定位路径）。
//!
//! 存在实例目录下、与 `game.json` 并列的独立文件 `guisetting.json`，
//! 结构与 ColorMC 的 `GameGuiSettingObj` 保持一致（字段名大写开头），
//! 这样两个启动器的实例目录可以互认。

use std::collections::{HashMap, HashSet};

use mml_base::serialize_tools;
use mml_config::config_save;
use mml_game::launcher::instance_setting_obj::InstanceSettingObj;
use serde::{Deserialize, Serialize};

/// 游戏日志窗口设置（对应 ColorMC `GameLogSettingObj`）
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct GameLogSettingObj {
    /// 自动换行
    #[serde(rename = "WordWrap")]
    pub word_wrap: bool,
    /// 自动下拉（跟随最新日志滚动）
    #[serde(rename = "Auto")]
    pub auto: bool,
    /// 日志等级开关：None / Info / Warn / Error / Debug
    #[serde(rename = "EnableNone")]
    pub enable_none: bool,
    #[serde(rename = "EnableInfo")]
    pub enable_info: bool,
    #[serde(rename = "EnableWarn")]
    pub enable_warn: bool,
    #[serde(rename = "EnableError")]
    pub enable_error: bool,
    #[serde(rename = "EnableDebug")]
    pub enable_debug: bool,
}

impl Default for GameLogSettingObj {
    /// 默认：换行开、自动下拉关、五个等级全开
    ///
    /// 等级默认全开是刻意的 —— 日志窗口的价值就在于"出事时能看到全部"，
    /// 默认关掉某些等级会让用户以为日志里没有那条记录。
    fn default() -> Self {
        Self {
            word_wrap: true,
            auto: false,
            enable_none: true,
            enable_info: true,
            enable_warn: true,
            enable_error: true,
            enable_debug: true,
        }
    }
}

/// 模组显示设置（对应 ColorMC `GameModSettingObj`）
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(default)]
pub struct GameModSettingObj {
    /// 模组分组列表：分组名 → 该组下的模组SHA1集合
    #[serde(rename = "Groups")]
    pub groups: HashMap<String, HashSet<String>>,
    /// 备注：模组文件名 → 用户写的说明
    #[serde(rename = "ModName")]
    pub mod_name: HashMap<String, Option<String>>,
}

/// 实例 GUI 设置（对应 ColorMC `GameGuiSettingObj`）
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct GameGuiSettingObj {
    /// 日志设置
    #[serde(rename = "Log")]
    pub log: GameLogSettingObj,
    /// 模组显示设置
    #[serde(rename = "Mod")]
    pub mods: GameModSettingObj,
    /// 显示方块：实例图标的方块 ID（与 `InstanceSettingObj.icon` 二选一）
    #[serde(rename = "Block")]
    pub block: Option<String>,
    /// 自动打开日志窗口（游戏启动后）
    #[serde(rename = "LogAutoShow")]
    pub log_auto_show: bool,
    /// 排列顺序
    ///
    /// **本启动器已不再使用**：组内次序改由内核分组表的数组顺序表达
    /// （`group_save.json`，见 `mml_game::game_group`），项目里既不读也不写它。
    /// 字段保留是为了与 ColorMC 互认同一个 `guisetting.json`
    /// —— 去掉它的话，下次保存会把 ColorMC 写的 `Order` 抹掉。
    #[serde(rename = "Order")]
    pub order: i32,
}

impl Default for GameGuiSettingObj {
    fn default() -> Self {
        Self {
            log: Default::default(),
            mods: Default::default(),
            block: None,
            log_auto_show: false,
            order: 0,
        }
    }
}

/// 读取某实例的 GUI 设置
///
/// 文件不存在 / 解析失败时返回默认值而不是报错：GUI 设置是**可选**的附属数据，
/// 老实例、手改坏的、别的启动器建的目录都可能没有这个文件，
/// 不该因为它缺了就影响实例的使用。
///
/// - `instance`: 实例配置（用它的目录定位文件）
pub fn load(instance: &InstanceSettingObj) -> GameGuiSettingObj {
    let file = instance.get_gui_setting_file();
    if !file.exists() || !file.is_file() {
        return GameGuiSettingObj::default();
    }

    serialize_tools::json_from_file::<GameGuiSettingObj>(&file).unwrap_or_default()
}

/// 保存某实例的 GUI 设置（异步落盘）
///
/// - `instance`: 实例配置（用它的目录定位文件）
/// - `obj`: 待保存的设置
pub fn save(instance: &InstanceSettingObj, obj: &GameGuiSettingObj) {
    config_save::save(instance.uuid, obj, &instance.get_gui_setting_file());
}

/// 读-改-写：把方块 ID 设为 `block`，并按二选一规则清掉 `Icon`
///
/// 图标与方块 ID 二选一：
/// - 设方块 ID → 写 `Block`、清 `InstanceSettingObj.icon`（不再用 icon.png）；
/// - 上传图片 → 写 `icon.png`、清 `Block`（见 [`clear_block`]）。
///
/// 两步必须一起做：只写一个会让图标读取走错分支。
pub fn set_block(instance: &InstanceSettingObj, block: String) {
    let mut obj = load(instance);
    obj.block = Some(block);
    save(instance, &obj);
}

/// 清掉方块 ID（上传自定义图标图片时调用）
pub fn clear_block(instance: &InstanceSettingObj) {
    let mut obj = load(instance);
    if obj.block.is_some() {
        obj.block = None;
        save(instance, &obj);
    }
}

/// 该实例的图标是否应当由方块 ID 现渲染
///
/// 给 `image_manager` 的图标取图分支用。
pub fn block_icon_id(instance: &InstanceSettingObj) -> Option<String> {
    load(instance).block
}
