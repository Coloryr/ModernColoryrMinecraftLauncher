use std::fs;

use mcml_base::path_helper;
use mcml_game::game_schematics::{self, SchematicType};
use mcml_nbt::nbt_file::NbtFile;

#[ignore = "reason"]
#[test]
fn dump_litematic() {
    let mut file = path_helper::open_read("./tests/schematics/test.litematic").unwrap();

    let nbt = NbtFile::read(&mut file).unwrap();
    let output = format!("{}", nbt);
    fs::write("./tests/schematics/test.litematic.txt", output).unwrap();
}

#[ignore = "reason"]
#[test]
fn dump_schematic() {
    let mut file = path_helper::open_read("./tests/schematics/test.schematic").unwrap();

    let nbt = NbtFile::read(&mut file).unwrap();
    let output = format!("{}", nbt);
    fs::write("./tests/schematics/test.schematic.txt", output).unwrap();
}

#[ignore = "reason"]
#[test]
fn dump_schem() {
    let mut file = path_helper::open_read("./tests/schematics/test.schem").unwrap();

    let nbt = NbtFile::read(&mut file).unwrap();
    let output = format!("{}", nbt);
    fs::write("./tests/schematics/test.schem.txt", output).unwrap();
}

#[ignore = "reason"]
#[test]
fn dump_nbt() {
    let mut file = path_helper::open_read("./tests/schematics/test.nbt").unwrap();

    let nbt = NbtFile::read(&mut file).unwrap();
    let output = format!("{}", nbt);
    fs::write("./tests/schematics/test.nbt.txt", output).unwrap();
}

#[test]
fn read_litematic() {
    let mut file = path_helper::open_read("./tests/schematics/test.litematic").unwrap();

    let sche = game_schematics::read_schematic_file(&mut file, SchematicType::Litematic).unwrap();

    println!("{:?}", sche);
}

#[test]
fn read_schematic() {
    let mut file = path_helper::open_read("./tests/schematics/test.schematic").unwrap();

    let sche = game_schematics::read_schematic_file(&mut file, SchematicType::Minecraft).unwrap();

    println!("{:?}", sche);
}

#[test]
fn read_schem() {
    let mut file = path_helper::open_read("./tests/schematics/test.schem").unwrap();

    let sche = game_schematics::read_schematic_file(&mut file, SchematicType::WorldEdit).unwrap();

    println!("{:?}", sche);
}

#[test]
fn read_nbt() {
    let mut file = path_helper::open_read("./tests/schematics/test.nbt").unwrap();

    let sche = game_schematics::read_schematic_file(&mut file, SchematicType::Create).unwrap();

    println!("{:?}", sche);
}