//! 资源管理窗口 DTO —— 实例的模组 / 材质包 / 存档 / 截图 / 服务器 / 光影包 / 结构 / 数据包列表。

use serde::Serialize;

/// 模组条目
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModItemDto {
    /// 模组稳定标识（文件路径的 uuid v5，启用/禁用/删除按此定位）
    pub uuid: String,
    /// mods 目录下的文件名（显示与打开文件夹定位用）
    pub file: String,
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
    /// 图标 data URL（无图标为空串）
    pub icon: String,
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
