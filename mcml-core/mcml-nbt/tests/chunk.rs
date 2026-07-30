use std::{
    fs::File,
    io::{Cursor, Seek, SeekFrom},
    path::Path,
};

use mcml_nbt::chunk::ChunkData;

#[test]
fn chunk_mca() {
    let path = Path::new("tests").join("r.0.0.mca");
    let mut file = File::open(path).unwrap();

    let mut chunk = ChunkData::read_chunk(&mut file).unwrap();

    assert_eq!(chunk.nbt.len(), 1024);
    assert_eq!(chunk.pos.len(), 1024);

    let mut stream1 = Cursor::new(Vec::<u8>::new());
    chunk.write_chunk(&mut stream1).unwrap();

    stream1.seek(SeekFrom::Start(0)).unwrap();

    let chunk1 = ChunkData::read_chunk(&mut stream1).unwrap();

    assert_eq!(chunk1.nbt.len(), 1024);
    assert_eq!(chunk1.pos.len(), 1024);

    // assert_eq!(chunk.pos, chunk1.pos);
}
