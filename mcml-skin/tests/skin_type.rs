use std::path::Path;

use mcml_skin::{SkinType, skin, skin_type_checker::skin_type_checker};

#[test]
fn test_skin_old() {
    let file = Path::new("tests").join("skin_old.png");
    let file = file.as_path();
    let skin = skin::open_bitmap(file);
    assert!(skin.is_some());
    assert_eq!(
        skin_type_checker::get_skin_type(&skin.unwrap()),
        SkinType::Old
    );
}

#[test]
fn test_skin_new() {
    let file = Path::new("tests").join("skin_new.png");
    let file = file.as_path();
    let skin = skin::open_bitmap(file);
    assert!(skin.is_some());
    assert_eq!(
        skin_type_checker::get_skin_type(&skin.unwrap()),
        SkinType::New
    );
}

#[test]
fn test_skin_slim() {
    let file = Path::new("tests").join("skin_slim.png");
    let file = file.as_path();
    let skin = skin::open_bitmap(file);
    assert!(skin.is_some());
    assert_eq!(
        skin_type_checker::get_skin_type(&skin.unwrap()),
        SkinType::NewSlim
    );
}

#[test]
fn test_skin_unknown() {
    let file = Path::new("tests").join("skin_unknown.png");
    let file = file.as_path();
    let skin = skin::open_bitmap(file);
    assert!(skin.is_some());
    assert_eq!(
        skin_type_checker::get_skin_type(&skin.unwrap()),
        SkinType::Unknown
    );
}
