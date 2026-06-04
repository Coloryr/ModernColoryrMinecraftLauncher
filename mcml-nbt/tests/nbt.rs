use std::io::{Cursor, Seek, SeekFrom};

use mcml_nbt::{
    get_nbt,
    nbt::{CompressType, Nbt},
};

#[test]
fn nbt_byte() {
    let mut stream = Cursor::new(Vec::<u8>::new());

    let nbt = Nbt::new(get_nbt(1).unwrap(), CompressType::GZip);

    nbt.save(&mut stream).unwrap();

    stream.seek(SeekFrom::Start(0)).unwrap();

    let nbt1 = Nbt::read(&mut stream).unwrap();

    assert_eq!(nbt.compress, nbt1.compress);
    assert!(nbt.nbt.eq(&nbt1.nbt));
}

#[test]
fn nbt_short() {
    let mut stream = Cursor::new(Vec::<u8>::new());

    let nbt = Nbt::new(get_nbt(1).unwrap(), CompressType::GZip);

    nbt.save(&mut stream).unwrap();

    stream.seek(SeekFrom::Start(0)).unwrap();

    let nbt1 = Nbt::read(&mut stream).unwrap();

    assert_eq!(nbt.compress, nbt1.compress);
    assert!(nbt.nbt.eq(&nbt1.nbt));
}