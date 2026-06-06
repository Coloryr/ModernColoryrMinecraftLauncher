use std::{
    fs::File,
    io::{Cursor, Read, Seek, SeekFrom},
    path::{Path, PathBuf},
};

use mcml_nbt::{
    NBT_COMPOUND_ORDER, NbtType, nbt_file::{CompressType, NbtFile}, nbt_types
};

#[test]
fn nbt_gzip() {
    let mut stream = Cursor::new(Vec::<u8>::new());

    let mut nbt = nbt_types::compound();
    nbt.data.insert(String::new(), NbtType::compound());
    nbt.data
        .insert(String::from("byte"), nbt_types::byte(20).to_nbt());
    nbt.data
        .insert(String::from("int"), nbt_types::int(30).to_nbt_type());
    nbt.data
        .insert(String::from("long"), nbt_types::long(1).to_nbt());

    let nbt = NbtFile::new(nbt.to_nbt(), CompressType::GZip);

    nbt.save(&mut stream).unwrap();

    stream.seek(SeekFrom::Start(0)).unwrap();

    let nbt1 = NbtFile::read(&mut stream).unwrap();

    assert_eq!(nbt.compress, nbt1.compress);
    assert!(nbt.nbt.eq(&nbt1.nbt));
}

#[test]
fn nbt_zlib() {
    let mut stream = Cursor::new(Vec::<u8>::new());

    let mut nbt = nbt_types::compound();
    nbt.data.insert(String::new(), NbtType::compound());
    nbt.data
        .insert(String::from("byte"), nbt_types::byte(20).to_nbt());
    nbt.data
        .insert(String::from("int"), nbt_types::int(30).to_nbt_type());
    nbt.data
        .insert(String::from("long"), nbt_types::long(1).to_nbt());

    let nbt = NbtFile::new(nbt.to_nbt(), CompressType::Zlib);

    nbt.save(&mut stream).unwrap();

    stream.seek(SeekFrom::Start(0)).unwrap();

    let nbt1 = NbtFile::read(&mut stream).unwrap();

    assert_eq!(nbt.compress, nbt1.compress);
    assert!(nbt.nbt.eq(&nbt1.nbt));
}

#[test]
fn nbt_lz4() {
    let mut stream = Cursor::new(Vec::<u8>::new());

    let mut nbt = nbt_types::compound();
    nbt.data.insert(String::new(), NbtType::compound());
    nbt.data
        .insert(String::from("byte"), nbt_types::byte(20).to_nbt());
    nbt.data
        .insert(String::from("int"), nbt_types::int(30).to_nbt_type());
    nbt.data
        .insert(String::from("long"), nbt_types::long(1).to_nbt());

    let nbt = NbtFile::new(nbt.to_nbt(), CompressType::Lz4);

    nbt.save(&mut stream).unwrap();

    stream.seek(SeekFrom::Start(0)).unwrap();

    let nbt1 = NbtFile::read(&mut stream).unwrap();

    assert_eq!(nbt.compress, nbt1.compress);
    assert!(nbt.nbt.eq(&nbt1.nbt));
}

#[test]
fn load_dat() {
    let path = Path::new("tests").join("level.dat");
    let mut stream = File::open(path).unwrap();

    let nbt = NbtFile::read(&mut stream).unwrap();

    assert_eq!(nbt.nbt.get_num(), NBT_COMPOUND_ORDER);

    let mut stream1 = Cursor::new(Vec::<u8>::new());
    nbt.save(&mut stream1).unwrap();

    stream.seek(SeekFrom::Start(0)).unwrap();

    let mut temp = Vec::<u8>::new();
    stream.read_to_end(&mut temp).unwrap();
}
