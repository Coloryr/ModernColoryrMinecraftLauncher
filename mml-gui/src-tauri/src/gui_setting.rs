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

/// 一个模组自定义分组
///
/// 分组用 **uuid 作键**（见 [`GameModSettingObj::groups`]）：
/// - 改名不影响归属（名字可以随便改，键不动）；
/// - 分组名允许任意文本，也不会再和"状态分组"这种内部块撞名（原先自建分组用组名作键，
///   得靠 `$` 前缀把状态分组区分开）。
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(default)]
pub struct GameModGroupObj {
    /// 分组名（用户可见；可重名检查在 `resource.rs` 里做）
    #[serde(rename = "Name")]
    pub name: String,
    /// 该组的模组 **SHA1** 列表
    ///
    /// 用内容哈希而不是文件名 / uuid：启用 / 禁用只改文件名，SHA1 不变
    /// （见 `resource.rs` 的说明）。
    #[serde(rename = "Mods")]
    pub mods: HashSet<String>,
}

/// 模组显示设置
///
/// **不再与 ColorMC 互通**（用户明确要求）：`Groups` 从原来的
/// 「分组名 → SHA1 集合」改成「分组 uuid → [`GameModGroupObj`]」，
/// 分组名挪进对象里当一个普通字段。ColorMC 按名字查 `Groups[name]`，读这份文件会
/// 看不到分组 —— 这是有意为之，换来的是"改名不改键、不会与内部块撞名"。
///
/// 注意 `#[serde(default)]` 的语义：整个 `Mod` 字段缺失时用 [`Default`]，
/// 而**单个**字段缺失时用该字段类型的默认值（`Vec::default()` = 空）。
/// 所以"已启用默认收起"不能只写在 [`Default`] 里 —— 老文件有 `Mod` 但没有
/// `GroupCollapsed`，那个分支会给出空表。缺字段的兜底见 `resource.rs` 的读取处。
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(default)]
pub struct GameModSettingObj {
    /// 模组自定义分组：**分组 uuid → 分组**（名字 + 模组 SHA1 列表）
    ///
    /// 状态分组（已启用 / 已禁用 / 识别失败）**不在这一列**：它们不是用户数据，
    /// 由 `resource.rs` 用固定 uuid 现拼（见那里的 `STATE_GROUP_*`）。
    #[serde(rename = "Groups")]
    pub groups: HashMap<String, GameModGroupObj>,
    /// 备注：模组SHA1 → 用户写的说明
    #[serde(rename = "ModName")]
    pub mod_name: HashMap<String, Option<String>>,
    /// 分组块的显示顺序（**分组 uuid**，含三个状态分组的固定 uuid）
    ///
    /// `Groups` 是 `HashMap`、没有"建立顺序"可言，所以顺序单独存一份。
    #[serde(rename = "GroupOrder")]
    pub group_order: Vec<String>,
    /// **收起**的分组块键集合（分组 uuid，口径与 `GroupOrder` 一致）
    ///
    /// 存"收起的那些"而不是"展开的那些"：默认全展开，空数组即"全部展开"。
    ///
    /// `Option` 是为了区分**两种情况**（两者的行为不同，所以不能都用空表表示）：
    /// - `None`：这个字段从来没写过（老文件 / 新实例）→ 用"已启用默认收起"的初值；
    /// - `Some(vec![])`：用户手动把「已启用」展开了 → 尊重用户，保持展开。
    #[serde(rename = "GroupCollapsed")]
    pub group_collapsed: Option<Vec<String>>,
}

/// 实例的**界面视图**设置（跟着实例走的那部分）
///
/// 只放"换实例就该换一套"的东西：资源窗口的左侧分类顺序 / 上次打开的类别 / 模组展示方式。
/// 全局性的界面设置（主题、颜色、字体、语言、窗口模式…）**不在这里** ——
/// 它们与实例无关，放在 `gui_config.json`（跨窗口那份"界面状态"）里。
///
/// 与 ColorMC 的互认：这是本启动器自己的扩展字段（对方结构里没有），
/// 存成 `Gui`；对方解析到不认识的字段会忽略，不影响共用同一个实例目录。
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(default)]
pub struct GameViewSettingObj {
    /// 资源窗口：左侧分类的显示顺序（用户拖出来的）
    ///
    /// 存分类 id 字符串（saves / mods / …）。**没存过就是空数组** ——
    /// 排序口径（新增分类补末尾、未知分类丢弃）由前端 `useResourceView` 负责，
    /// 后端只当它是"一串字符串"保管，不认识具体分类。
    #[serde(rename = "ResourceOrder")]
    pub resource_order: Vec<String>,
    /// 资源窗口：上次打开的类别（空串 = 没记过，前端用默认值）
    #[serde(rename = "ResourceCategory")]
    pub resource_category: String,
    /// 资源窗口：模组的展示方式 list / table / tree（空串 = 没记过）
    #[serde(rename = "ResourceModView")]
    pub resource_mod_view: String,
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
    /// 界面视图设置（跟实例走的那部分，见 [`GameViewSettingObj`]）
    #[serde(rename = "Gui")]
    pub view: GameViewSettingObj,
}

impl Default for GameGuiSettingObj {
    fn default() -> Self {
        Self {
            log: Default::default(),
            mods: Default::default(),
            block: None,
            log_auto_show: false,
            view: Default::default(),
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
