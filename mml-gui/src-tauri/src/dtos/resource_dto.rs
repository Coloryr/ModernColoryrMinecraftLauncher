//! 资源管理窗口 DTO —— 实例的模组 / 材质包 / 存档 / 截图 / 服务器 / 光影包 / 结构 / 数据包列表。

use serde::Serialize;

/// 模组条目
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModItemDto {
    /// 模组稳定标识（文件路径的 uuid v5，启用/禁用/删除按此定位）
    ///
    /// 注意：启用 / 禁用会改文件名 → 这个 uuid 跟着变，**不能拿它存分组**（见 `key`）。
    /// 内置模组（`jar_in_jar` 里的）没有独立文件，这里为空串。
    pub uuid: String,
    /// 文件 SHA1：**自定义分组的键**（与 `guisetting.json` 的 `Mod.Groups` 一致）
    ///
    /// 用内容哈希而不是 uuid：启用 / 禁用只是给文件名加减后缀，uuid（文件路径的 v5）会变，
    /// 而 SHA1 不变 —— 否则一禁用就掉出分组。读不出元数据的坏 jar 没有哈希，为空串（不可归组）。
    pub sha1: String,
    /// mods 目录下的文件名（显示与打开文件夹定位用）
    pub file: String,
    /// 相对实例的路径（如 `.minecraft\mods\sodium.jar`）
    ///
    /// 比文件名多一点定位信息、又不像绝对路径那样长到看不完；
    /// 内置模组（jar-in-jar）没有独立文件，为空串。
    pub path: String,
    /// 是否被禁用
    pub disable: bool,
    /// 是否读取失败（元数据解析出错）
    pub fail: bool,
    /// 是否为 Core 模组
    pub core: bool,
    /// modid（可能为空）
    pub mod_id: String,
    /// 显示名（元数据缺失时为空，前端回退文件名）
    pub name: String,
    /// 版本号
    pub version: String,
    /// 作者（逗号拼接）
    pub author: String,
    /// 描述
    pub description: String,
    /// 支持的加载器（`LoaderType::to_string()`：forge / fabric / quilt / neoforge / …）
    pub loader: String,
    /// 加载侧（`LoadSideType` 小写：client / server / both / unknown）
    pub side: String,
    /// 网页链接（元数据里的 url，可能为空）
    pub url: String,
    /// 下载源（`get_source_type(pid, fid)`：curseforge / modrinth；不是从平台下的为空串）
    pub source: String,
    /// 项目编号（来自实例的 `online_info.json`，按 SHA1 关联；空串 = 不是从平台下的）
    pub project_id: String,
    /// 文件编号（同上）
    pub file_id: String,
    /// 用户写的备注（`guisetting.json` 的 `Mod.ModName`，没有则为空串）
    ///
    /// 键按**去掉禁用后缀的文件名**归一（见 `resource.rs::mod_note_key`）：
    /// 启用 / 禁用只给文件名加减 `.disabled`，不归一的话一禁用备注就丢了。
    pub note: String,
    /// 图标 data URL（无图标为空串）
    pub icon: String,
    /// 内置模组（jar-in-jar / jarjar，递归）
    ///
    /// 它们装在父 jar 里，没有独立文件：前端只展示，不提供启用 / 删除。
    pub jar_in_jar: Vec<ModItemDto>,
}

/// 模组扫描进度（事件 `resource-list-mods-progress` 的负载）
///
/// 解析一个 jar 要读元数据 / 图标 / 扫 class，几百个包要好几秒 ——
/// 界面靠它显示 `x/x`，否则用户面对空列表不知道是在跑还是卡住了。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModScanProgressDto {
    /// 已完成的文件数
    pub done: usize,
    /// 需要处理的文件总数
    pub total: usize,
}

/// 资源窗口的视图偏好（存在**实例**的 `guisetting.json` 里，跟着实例走）
///
/// 与 `gui_config.json` 那份全局界面状态分开：这一份是"这个实例我习惯怎么看"，
/// 换个实例就该换一套（见 `gui_setting::GameViewSettingObj`）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceViewDto {
    /// 左侧分类的显示顺序（分类 id 字符串；空数组 = 没存过，前端补默认顺序）
    pub order: Vec<String>,
    /// 上次打开的类别（空串 = 没记过）
    pub category: String,
    /// 模组的展示方式：list / table / tree（空串 = 没记过）
    pub mod_view: String,
}

/// 模组自定义分组（存在实例的 `guisetting.json` 的 `Mod.Groups` 里）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModGroupDto {
    /// 分组 uuid（**稳定标识**：改名不影响它，也是顺序 / 折叠状态用的键）
    pub uuid: String,
    /// 分组名（用户可见，可改）
    pub name: String,
    /// 组内模组的 **SHA1** 列表（有序：下发前排过序，底层是无序集合）
    pub mods: Vec<String>,
}

/// 材质包条目
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackItemDto {
    /// resourcepacks 目录下的文件名
    pub file: String,
    /// 简介
    pub description: String,
    /// 版本号
    pub pack_format: i64,
    /// 最小版本
    pub min_format: i64,
    /// 最大版本号
    pub max_format: i64,
    /// 是否读取失败
    pub fail: bool,
    /// 图标 data URL（无图标为空串）
    pub icon: String,
}

/// 存档条目
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveItemDto {
    /// saves 目录下的目录名（删除/备份按此定位）
    pub dir: String,
    /// 世界名字
    pub level_name: String,
    /// 上次游玩（Unix 毫秒，未知为 0）
    pub last_played: i64,
    /// 游戏类型（0 生存 / 1 创造 / 2 冒险）
    pub game_type: i32,
    /// 极限模式
    pub hard_core: bool,
    /// 难度（0 和平 / 1 简单 / 2 普通 / 3 困难）
    pub difficulty: u8,
    /// 是否损坏（level.dat 解析失败）
    pub broken: bool,
    /// 图标 data URL（无图标为空串）
    pub icon: String,
}

/// 截图条目
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenshotItemDto {
    /// 文件名（预览 / 删除按此定位）
    pub name: String,
}

/// 服务器条目
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerItemDto {
    /// 服务器名（与 ip 一起构成定位键）
    pub name: String,
    /// 服务器地址
    pub ip: String,
    /// 是否接受服务器资源包
    pub accept_textures: bool,
    /// 图标 data URL（无图标为空串）
    pub icon: String,
}

/// 光影包条目
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShaderItemDto {
    /// shaderpacks 目录下的文件名
    pub file: String,
    /// 显示名（语言文件解析，缺失时回退文件名）
    pub name: String,
    /// 描述
    pub comment: String,
    /// 是否为当前启用的光影包（options.txt 的 shaderPack 键）
    pub selected: bool,
}

/// 结构文件条目
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SchematicItemDto {
    /// schematics 目录下的文件名（删除按此定位）
    pub file: String,
    /// 名称
    pub name: String,
    /// 作者
    pub author: String,
    /// 描述
    pub description: String,
    /// 类型标签（Minecraft / Litematic / WorldEdit / Create）
    pub type_name: String,
    /// 宽
    pub width: i32,
    /// 高
    pub height: i32,
    /// 长
    pub length: i32,
    /// 方块总数
    pub block_count: u64,
    /// 方块种类数
    pub block_types: u32,
    /// 是否解析失败
    pub fail: bool,
}

/// 数据包条目（存档子页）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DataPackItemDto {
    /// NBT 里的登记名（file/xxx，toggle / 删除按此定位）
    pub name: String,
    /// datapacks 目录下的文件或目录名
    pub file: String,
    /// 描述（pack.mcmeta）
    pub description: String,
    /// 格式版本号
    pub pack_format: i64,
    /// 是否启用（null = 未登记进 level.dat）
    pub enable: Option<bool>,
}
