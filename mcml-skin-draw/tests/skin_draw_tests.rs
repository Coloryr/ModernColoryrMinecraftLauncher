use std::{ path::Path};

use mcml_skin::skin::{self};
use mcml_skin_draw::{cape_2d_draw::cape_2d, head_2d_draw::head_2d, skin_2d_draw::skin_2d, head_3d_draw::head_3d};
use skia_safe::{Bitmap};

/// 比较两个 Bitmap 的内容是否完全一致
fn assert_bitmap_eq(a: &mut Bitmap, b: &mut Bitmap) {
    assert_eq!(
        a.width(),
        b.width(),
        "bitmap width mismatch: {} != {}",
        a.width(),
        b.width()
    );
    assert_eq!(
        a.height(),
        b.height(),
        "bitmap height mismatch: {} != {}",
        a.height(),
        b.height()
    );
    assert_eq!(a.color_type(), b.color_type(), "bitmap color_type mismatch");
    assert_eq!(a.alpha_type(), b.alpha_type(), "bitmap alpha_type mismatch");
    assert_eq!(a.row_bytes(), b.row_bytes(), "bitmap row_bytes mismatch");

    let a_bytes = a.pixels();
    let b_bytes = b.pixels();
    assert!(!a_bytes.is_null(), "first bitmap pixels is null");
    assert!(!b_bytes.is_null(), "second bitmap pixels is null");

    let total_size = (a.height() as usize) * (a.row_bytes() as usize);
    unsafe {
        let a_slice = std::slice::from_raw_parts(a_bytes as *const u8, total_size);
        let b_slice = std::slice::from_raw_parts(b_bytes as *const u8, total_size);
        assert_eq!(a_slice, b_slice, "bitmap pixel data mismatch");
    }
}

#[test]
fn test_cape_draw() {
    let file = Path::new("tests").join("cape.png");
    let file = file.as_path();
    let image = skin::open_bitmap(file);
    assert!(image.is_some());
    let mut image = image.unwrap();
    let res = cape_2d::draw_cape_2d(&mut image);
    assert!(res.is_some());
    let mut res = res.unwrap();

    let file = Path::new("tests").join("out_cape.png");
    let file = file.as_path();

    let out = skin::open_bitmap(file);
    assert!(out.is_some());
    let mut out = out.unwrap();

    assert_bitmap_eq(&mut res, &mut out);

    // skin::save_bitmap(&res, file);
}

#[test]
fn test_cape_back_draw() {
    let file = Path::new("tests").join("cape.png");
    let file = file.as_path();
    let image = skin::open_bitmap(file);
    assert!(image.is_some());
    let mut image = image.unwrap();
    let res = cape_2d::draw_cape_back_2d(&mut image);
    assert!(res.is_some());
    let mut res = res.unwrap();

    let file = Path::new("tests").join("out_cape_back.png");
    let file = file.as_path();

    let out = skin::open_bitmap(file);
    assert!(out.is_some());
    let mut out = out.unwrap();

    assert_bitmap_eq(&mut res, &mut out);

    // skin::save_bitmap(&res, file);
}

#[test]
fn test_head_draw_typea() {
    let file = Path::new("tests").join("skin_slim.png");
    let file = file.as_path();
    let image = skin::open_bitmap(file);
    assert!(image.is_some());
    let mut image = image.unwrap();
    let res = head_2d::head_2d_draw_typea(&mut image);
    assert!(res.is_some());
    let mut res = res.unwrap();

    let file = Path::new("tests").join("out_head_a.png");
    let file = file.as_path();

    let out = skin::open_bitmap(file);
    assert!(out.is_some());
    let mut out = out.unwrap();

    assert_bitmap_eq(&mut res, &mut out);

    // skin::save_bitmap(&res, file);
}

#[test]
fn test_head_draw_typeb() {
    let file = Path::new("tests").join("skin_slim.png");
    let file = file.as_path();
    let image = skin::open_bitmap(file);
    assert!(image.is_some());
    let mut image = image.unwrap();
    let res = head_2d::head_2d_draw_typeb(&mut image);
    assert!(res.is_some());
    let mut res = res.unwrap();

    let file = Path::new("tests").join("out_head_b.png");
    let file = file.as_path();

    let out = skin::open_bitmap(file);
    assert!(out.is_some());
    let mut out = out.unwrap();

    assert_bitmap_eq(&mut res, &mut out);

    // skin::save_bitmap(&res, file);
}

#[test]
fn test_skin_draw_typea() {
    let file = Path::new("tests").join("skin_slim.png");
    let file = file.as_path();
    let image = skin::open_bitmap(file);
    assert!(image.is_some());
    let mut image = image.unwrap();
    let res = skin_2d::skin_2d_draw_typea(&mut image, None);
    assert!(res.is_some());
    let mut res = res.unwrap();

    let file = Path::new("tests").join("out_skin_2d_a.png");
    let file = file.as_path();

    let out = skin::open_bitmap(file);
    assert!(out.is_some());
    let mut out = out.unwrap();

    assert_bitmap_eq(&mut res, &mut out);

    // skin::save_bitmap(&res, file);
}

#[test]
fn test_skin_draw_typeb() {
    let file = Path::new("tests").join("skin_slim.png");
    let file = file.as_path();
    let image = skin::open_bitmap(file);
    assert!(image.is_some());
    let mut image = image.unwrap();
    let res = skin_2d::skin_2d_draw_typeb(&mut image, None);
    assert!(res.is_some());
    let mut res = res.unwrap();

    let file = Path::new("tests").join("out_skin_2d_b.png");
    let file = file.as_path();

    let out = skin::open_bitmap(file);
    assert!(out.is_some());
    let mut out = out.unwrap();

    assert_bitmap_eq(&mut res, &mut out);

    // skin::save_bitmap(&res, file);
}

#[test]
fn test_head_3d_draw_typea() {
    let file = Path::new("tests").join("skin_slim.png");
    let file = file.as_path();
    let image = skin::open_bitmap(file);
    assert!(image.is_some());
    let mut image = image.unwrap();
    let res = head_3d::draw_head_3d(&mut image);
    assert!(res.is_some());
    let mut res = res.unwrap();

    let file = Path::new("tests").join("out_head_3d_a.png");
    let file = file.as_path();

    // let out = skin::open_bitmap(file);
    // assert!(out.is_some());
    // let mut out = out.unwrap();

    // assert_bitmap_eq(&mut res, &mut out);

    skin::save_bitmap(&res, file);
}