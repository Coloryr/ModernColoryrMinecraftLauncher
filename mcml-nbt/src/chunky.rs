use crate::nbt_types::NbtCompound;

pub struct ChunkNbt {
    pub nbt: NbtCompound,
    pub point: PointI32,
}

pub struct PointI32 {
    pub x: i32,
    pub y: i32,
}

impl PointI32 {
    
}

pub fn pos_to_chunk(pos: PointI32) -> PointI32 {

}