use std::path::PathBuf;

use mcml_nbt::nbt_types::NbtCompound;

/// 游戏存档
pub struct SaveObj {
    /// 地图种子
    pub random_seed: i64,
    /// 游戏类型
    pub game_type: i32,
    /// 极限模式
    pub hard_core: i8,
    /// 世界名字
    pub level_name: String,
    /// 难度
    pub difficulty: i8,
    /// 生成器名字
    pub generator_name: String,
    /// 路径
    pub path: PathBuf,
    /// 图标
    pub icon: PathBuf,
    /// 是否损坏
    pub broken: bool,
    /// 存档NBT
    pub nbt: NbtCompound,
}
