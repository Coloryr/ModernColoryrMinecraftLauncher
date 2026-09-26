use std::path::Path;

use mml_skin_draw::{cape_2d_draw, head_2d_draw, head_3d_draw, skin_2d_draw, skin_3d_draw};
use tiny_skia::Pixmap;

/// 比较两个位图的内容是否完全一致
fn assert_bitmap_eq(a: &Pixmap, b: &Pixmap) {
    assert_eq!(a.width(), b.width(), "bitmap width mismatch: {} != {}", a.width(), b.width());
    assert_eq!(
        a.height(),
        b.height(),
        "bitmap height mismatch: {} != {}",
        a.height(),
        b.height()
    );
    assert_eq!(a.data().len(), b.data().len(), "bitmap data length mismatch");
    assert_eq!(a.data(), b.data(), "bitmap pixel data mismatch");
}

/// 读取参考图
fn load_reference(name: &str) -> Pixmap {
    let file = Path::new("tests").join(name);
    let out = mml_skin::open_bitmap(file.as_path());
    assert!(out.is_some(), "{name} 应能读取");

    out.unwrap()
}

/// 读取测试用皮肤
fn load_skin(name: &str) -> Pixmap {
    let file = Path::new("tests").join(name);
    let image = mml_skin::open_bitmap(file.as_path());
    assert!(image.is_some(), "{name} 应能读取");

    image.unwrap()
}

#[test]
fn test_cape_draw() {
    let image = load_skin("cape.png");

    let res = cape_2d_draw::draw_cape_2d(&image);
    assert!(res.is_some());
    let res = res.unwrap();

    assert_bitmap_eq(&res, &load_reference("out_cape.png"));
}

#[test]
fn test_cape_back_draw() {
    let image = load_skin("cape.png");

    let res = cape_2d_draw::draw_cape_back_2d(&image);
    assert!(res.is_some());
    let res = res.unwrap();

    assert_bitmap_eq(&res, &load_reference("out_cape_back.png"));
}

#[test]
fn test_head_draw_typea() {
    let image = load_skin("skin_slim.png");

    let res = head_2d_draw::head_2d_draw_typea(&image);
    assert!(res.is_some());
    let res = res.unwrap();

    assert_bitmap_eq(&res, &load_reference("out_head_a.png"));
}

#[test]
fn test_head_draw_typeb() {
    let image = load_skin("skin_slim.png");

    let res = head_2d_draw::head_2d_draw_typeb(&image);
    assert!(res.is_some());
    let res = res.unwrap();

    assert_bitmap_eq(&res, &load_reference("out_head_b.png"));
}

#[test]
fn test_skin_draw_typea() {
    let image = load_skin("skin_slim.png");

    let res = skin_2d_draw::skin_2d_draw_typea(&image, None);
    assert!(res.is_some());
    let res = res.unwrap();

    assert_bitmap_eq(&res, &load_reference("out_skin_2d_a.png"));
}

#[test]
fn test_skin_draw_typeb() {
    let image = load_skin("skin_slim.png");

    let res = skin_2d_draw::skin_2d_draw_typeb(&image, None);
    assert!(res.is_some());
    let res = res.unwrap();

    assert_bitmap_eq(&res, &load_reference("out_skin_2d_b.png"));
}

#[test]
fn test_head_3d_draw_typea() {
    let image = load_skin("skin_slim.png");

    let res = head_3d_draw::draw_head_3d_typea(&image, false);
    assert!(res.is_some());
    let res = res.unwrap();

    assert_bitmap_eq(&res, &load_reference("out_head_3d_a.png"));
}

#[test]
fn test_head_3d_draw_typeb() {
    let image = load_skin("skin_slim.png");

    let res = head_3d_draw::draw_head_3d_typeb(&image, 15.0, 65.0);
    assert!(res.is_some());
    let res = res.unwrap();

    assert_bitmap_eq(&res, &load_reference("out_head_3d_b.png"));
}

/// 重新生成 3D 参考图
///
/// 3D 输出带抗锯齿，换光栅器后不可能与旧图逐字节一致，所以这两张图由本实现重新生成：
/// `cargo test -p mml-skin-draw --test skin_draw_tests regen_3d_refs -- --ignored`
#[test]
#[ignore]
fn regen_3d_refs() {
    let image = load_skin("skin_slim.png");

    let a = head_3d_draw::draw_head_3d_typea(&image, false).expect("3d typea 应成功");
    mml_skin::save_bitmap(&a, Path::new("tests").join("out_head_3d_a.png").as_path());

    let b = head_3d_draw::draw_head_3d_typeb(&image, 15.0, 65.0).expect("3d typeb 应成功");
    mml_skin::save_bitmap(&b, Path::new("tests").join("out_head_3d_b.png").as_path());
}

/// TEMP：输出皮肤 3D 渲染到 target/temp/out_skin_3d.png 供人工检查，验证后删除
/// `cargo test -p mml-skin-draw --test skin_draw_tests dump_skin_3d -- --ignored`
#[test]
#[ignore]
fn dump_skin_3d() {
    let image = load_skin("skin_slim.png");
    let res = skin_3d_draw::draw_skin_3d_typea(&image, None).expect("渲染应成功");
    let dir = Path::new("../../target/temp");
    std::fs::create_dir_all(dir).unwrap();
    mml_skin::save_bitmap(&res, dir.join("out_skin_3d.png").as_path());
    // TEMP：低头模式（俯仰 +30°）
    let down = skin_3d_draw::draw_skin_3d_typeb(&image, None, 30.0, 45.0).expect("渲染应成功");
    mml_skin::save_bitmap(&down, dir.join("out_skin_3d_down.png").as_path());
}

/// TEMP：与 head_3d 同角度输出，对比头部贴图；验证后删除
#[test]
#[ignore]
fn dump_head_compare() {
    let image = load_skin("skin_slim.png");
    let dir = Path::new("../../target/temp");
    std::fs::create_dir_all(dir).unwrap();
    // 与 head 3D typeb 参考图同参数
    let skin = skin_3d_draw::draw_skin_3d_typeb(&image, None, 15.0, 65.0).expect("渲染应成功");
    mml_skin::save_bitmap(&skin, dir.join("out_skin_3d_headcmp.png").as_path());
    let head = head_3d_draw::draw_head_3d_typeb(&image, 15.0, 65.0).expect("渲染应成功");
    mml_skin::save_bitmap(&head, dir.join("out_head_3d_headcmp.png").as_path());
}

/// TEMP：typea 原角度对比皮肤 3D 与头 3D；验证后删除
#[test]
#[ignore]
fn dump_head_compare_a() {
    let image = load_skin("skin_slim.png");
    let dir = Path::new("../../target/temp");
    std::fs::create_dir_all(dir).unwrap();
    let skin = skin_3d_draw::draw_skin_3d_typea(&image, None).expect("渲染应成功");
    mml_skin::save_bitmap(&skin, dir.join("out_skin_3d_cmpa.png").as_path());
    let head = head_3d_draw::draw_head_3d_typea(&image, false).expect("渲染应成功");
    mml_skin::save_bitmap(&head, dir.join("out_head_3d_cmpa.png").as_path());
}

/// TEMP：输出面朝上的头像 3D 供检查，验证后删除
#[test]
#[ignore]
fn dump_head_typec() {
    let image = load_skin("skin_slim.png");
    let dir = Path::new("../../target/temp");
    std::fs::create_dir_all(dir).unwrap();
    let res = head_3d_draw::draw_head_3d_typea(&image, true).expect("渲染应成功");
    mml_skin::save_bitmap(&res, dir.join("out_head_3d_typec.png").as_path());
}

/// TEMP：只渲染外层（帽层）——把基础层头部区域清成透明再画面朝上视角，验证后删除
#[test]
#[ignore]
fn dump_head_overlay_only() {
    let image = load_skin("skin_slim.png");
    // 基础层头部占 (0..32, 0..16)，整体清透明，剩下的就是帽层贴图
    let mut overlay_skin = image.clone();
    let data = overlay_skin.data_mut();
    for y in 0..16usize {
        for x in 0..32usize {
            data[(y * 64 + x) * 4..(y * 64 + x) * 4 + 4].fill(0);
        }
    }

    let dir = Path::new("../../target/temp");
    std::fs::create_dir_all(dir).unwrap();
    let up = head_3d_draw::draw_head_3d_typea(&overlay_skin, true).expect("渲染应成功");
    mml_skin::save_bitmap(&up, dir.join("out_head_3d_overlay_up.png").as_path());
    let down = head_3d_draw::draw_head_3d_typea(&overlay_skin, false).expect("渲染应成功");
    mml_skin::save_bitmap(&down, dir.join("out_head_3d_overlay_down.png").as_path());
}

/// TEMP：统计 typea(false) 与参考图差异像素，验证后删除
#[test]
#[ignore]
fn diff_typea() {
    let image = load_skin("skin_slim.png");
    let res = head_3d_draw::draw_head_3d_typea(&image, false).expect("渲染应成功");
    let reference = load_reference("out_head_3d_a.png");
    let (rw, rh) = (reference.width() as usize, reference.height() as usize);
    let mut count = 0;
    for y in 0..rh {
        for x in 0..rw {
            let i = (y * rw + x) * 4;
            if res.data()[i..i + 4] != reference.data()[i..i + 4] {
                count += 1;
                if count <= 20 {
                    println!(
                        "({x},{y}) got {:?} want {:?}",
                        &res.data()[i..i + 4],
                        &reference.data()[i..i + 4]
                    );
                }
            }
        }
    }
    println!("total diff pixels: {count}");
}
