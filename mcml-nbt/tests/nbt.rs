use std::io::{Cursor, Seek, SeekFrom};

use mcml_nbt::{
    NbtType,
    nbt_file::{CompressType, NbtFile},
    nbt_types,
};

#[test]
fn nbt_end() {
    let mut stream = Cursor::new(Vec::<u8>::new());

    let nbt = NbtFile::new(nbt_types::end().to_nbt(), CompressType::GZip);

    nbt.save(&mut stream).unwrap();

    stream.seek(SeekFrom::Start(0)).unwrap();

    let nbt1 = NbtFile::read(&mut stream).unwrap();

    assert_eq!(nbt.compress, nbt1.compress);
    assert!(nbt.nbt.eq(&nbt1.nbt));
}

#[test]
fn nbt_byte() {
    let mut stream = Cursor::new(Vec::<u8>::new());

    let mut nbt = nbt_types::byte();
    nbt.data = 1;

    let nbt = NbtFile::new(nbt.to_nbt(), CompressType::GZip);

    nbt.save(&mut stream).unwrap();

    stream.seek(SeekFrom::Start(0)).unwrap();

    let nbt1 = NbtFile::read(&mut stream).unwrap();

    assert_eq!(nbt.compress, nbt1.compress);
    assert!(nbt.nbt.eq(&nbt1.nbt));
}

#[test]
fn nbt_short() {
    let mut stream = Cursor::new(Vec::<u8>::new());

    let mut nbt = nbt_types::short();
    nbt.data = 1;

    let nbt = NbtFile::new(nbt.to_nbt(), CompressType::GZip);

    nbt.save(&mut stream).unwrap();

    stream.seek(SeekFrom::Start(0)).unwrap();

    let nbt1 = NbtFile::read(&mut stream).unwrap();

    assert_eq!(nbt.compress, nbt1.compress);
    assert!(nbt.nbt.eq(&nbt1.nbt));
}

#[test]
fn nbt_long() {
    let mut stream = Cursor::new(Vec::<u8>::new());

    let mut nbt = nbt_types::long();
    nbt.data = 1;

    let nbt = NbtFile::new(nbt.to_nbt(), CompressType::GZip);

    nbt.save(&mut stream).unwrap();

    stream.seek(SeekFrom::Start(0)).unwrap();

    let nbt1 = NbtFile::read(&mut stream).unwrap();

    assert_eq!(nbt.compress, nbt1.compress);
    assert!(nbt.nbt.eq(&nbt1.nbt));
}

#[test]
fn nbt_float() {
    let mut stream = Cursor::new(Vec::<u8>::new());

    let mut nbt = nbt_types::float();
    nbt.data = 1.0;

    let nbt = NbtFile::new(nbt.to_nbt(), CompressType::GZip);

    nbt.save(&mut stream).unwrap();

    stream.seek(SeekFrom::Start(0)).unwrap();

    let nbt1 = NbtFile::read(&mut stream).unwrap();

    assert_eq!(nbt.compress, nbt1.compress);
    assert!(nbt.nbt.eq(&nbt1.nbt));
}

#[test]
fn nbt_double() {
    let mut stream = Cursor::new(Vec::<u8>::new());

    let mut nbt = nbt_types::double();
    nbt.data = 1.0;

    let nbt = NbtFile::new(nbt.to_nbt(), CompressType::GZip);

    nbt.save(&mut stream).unwrap();

    stream.seek(SeekFrom::Start(0)).unwrap();

    let nbt1 = NbtFile::read(&mut stream).unwrap();

    assert_eq!(nbt.compress, nbt1.compress);
    assert!(nbt.nbt.eq(&nbt1.nbt));
}

#[test]
fn nbt_string() {
    let mut stream = Cursor::new(Vec::<u8>::new());

    let mut nbt = nbt_types::string();
    nbt.data = String::from("value");

    let nbt = NbtFile::new(nbt.to_nbt(), CompressType::GZip);

    nbt.save(&mut stream).unwrap();

    stream.seek(SeekFrom::Start(0)).unwrap();

    let nbt1 = NbtFile::read(&mut stream).unwrap();

    assert_eq!(nbt.compress, nbt1.compress);
    assert!(nbt.nbt.eq(&nbt1.nbt));
}

#[test]
fn nbt_byte_array() {
    let mut stream = Cursor::new(Vec::<u8>::new());

    let mut nbt = nbt_types::byte_array();
    nbt.data.push(0);
    nbt.data.push(1);
    nbt.data.push(2);
    nbt.data.push(3);

    let nbt = NbtFile::new(nbt.to_nbt(), CompressType::GZip);

    nbt.save(&mut stream).unwrap();

    stream.seek(SeekFrom::Start(0)).unwrap();

    let nbt1 = NbtFile::read(&mut stream).unwrap();

    assert_eq!(nbt.compress, nbt1.compress);
    assert!(nbt.nbt.eq(&nbt1.nbt));
}

#[test]
fn nbt_int_array() {
    let mut stream = Cursor::new(Vec::<u8>::new());

    let mut nbt = nbt_types::int_array();
    nbt.data.push(0);
    nbt.data.push(1);
    nbt.data.push(2);
    nbt.data.push(3);

    let nbt = NbtFile::new(nbt.to_nbt(), CompressType::GZip);

    nbt.save(&mut stream).unwrap();

    stream.seek(SeekFrom::Start(0)).unwrap();

    let nbt1 = NbtFile::read(&mut stream).unwrap();

    assert_eq!(nbt.compress, nbt1.compress);
    assert!(nbt.nbt.eq(&nbt1.nbt));
}

#[test]
fn nbt_long_array() {
    let mut stream = Cursor::new(Vec::<u8>::new());

    let mut nbt = nbt_types::long_array();
    nbt.data.push(0);
    nbt.data.push(1);
    nbt.data.push(2);
    nbt.data.push(3);

    let nbt = NbtFile::new(nbt.to_nbt(), CompressType::GZip);

    nbt.save(&mut stream).unwrap();

    stream.seek(SeekFrom::Start(0)).unwrap();

    let nbt1 = NbtFile::read(&mut stream).unwrap();

    assert_eq!(nbt.compress, nbt1.compress);
    assert!(nbt.nbt.eq(&nbt1.nbt));
}

#[test]
fn nbt_list() {
    let mut stream = Cursor::new(Vec::<u8>::new());

    let mut nbt = nbt_types::list();
    nbt.set_type(NbtType::byte());

    assert!(!nbt.add_item(NbtType::int()));
    assert!(nbt.add_item(nbt_types::byte().to_nbt()));

    let mut temp = nbt_types::byte();
    temp.data = 1;
    assert!(nbt.add_item(temp.to_nbt()));

    let nbt = NbtFile::new(nbt.to_nbt(), CompressType::GZip);

    nbt.save(&mut stream).unwrap();

    stream.seek(SeekFrom::Start(0)).unwrap();

    let nbt1 = NbtFile::read(&mut stream).unwrap();

    assert_eq!(nbt.compress, nbt1.compress);
    assert!(nbt.nbt.eq(&nbt1.nbt));
}

#[test]
fn nbt_compound() {
    let mut stream = Cursor::new(Vec::<u8>::new());

    let mut nbt = nbt_types::compound();
    nbt.data.insert(String::new(), NbtType::compound());
    nbt.data.insert(String::from("byte"), NbtType::byte());
    nbt.data.insert(String::from("int"), NbtType::int());

    let mut temp = nbt_types::long();
    temp.data = 1;
    nbt.data.insert(String::from("long"), temp.to_nbt());

    let nbt = NbtFile::new(nbt.to_nbt(), CompressType::GZip);

    nbt.save(&mut stream).unwrap();

    stream.seek(SeekFrom::Start(0)).unwrap();

    let nbt1 = NbtFile::read(&mut stream).unwrap();

    assert_eq!(nbt.compress, nbt1.compress);
    assert!(nbt.nbt.eq(&nbt1.nbt));
}
