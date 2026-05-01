use mcml_skin_render::skin_type_checker::skin_type_checker;
use mcml_skin_render::texture::texture::SkinType;
use skia_safe::{Bitmap, Color, ImageInfo, IRect};

/// 创建一个指定大小和颜色的 Bitmap 用于测试
fn create_test_bitmap(width: i32, height: i32, color: Color) -> Bitmap {
    let mut bitmap = Bitmap::new();
    let image_info = ImageInfo::new(
        (width, height),
        skia_safe::ColorType::RGBA8888,
        skia_safe::AlphaType::Premul,
        None,
    );
    bitmap.set_info(&image_info, None);
    bitmap.alloc_pixels();
    bitmap.erase(color, IRect::new(0, 0, width, height));
    bitmap
}

/// 在 Bitmap 的指定区域设置颜色（直接操作像素）
fn set_pixel_area(bitmap: &mut Bitmap, x: i32, y: i32, w: i32, h: i32, color: Color) {
    let row_bytes = bitmap.row_bytes() as usize;
    let bytes_per_pixel = bitmap.bytes_per_pixel() as usize;
    let pixels = bitmap.pixels() as *mut u8;

    unsafe {
        for dy in 0..h {
            for dx in 0..w {
                let offset = ((y + dy) as usize) * row_bytes + ((x + dx) as usize) * bytes_per_pixel;
                let pixel = std::slice::from_raw_parts_mut(pixels.add(offset), bytes_per_pixel);
                pixel[0] = color.b();
                pixel[1] = color.g();
                pixel[2] = color.r();
                pixel[3] = color.a();
            }
        }
    }
}

#[test]
fn test_get_text_type_old_skin() {
    // 旧版皮肤: 64x32
    let bitmap = create_test_bitmap(64, 32, Color::WHITE);
    let result = skin_type_checker::get_text_type(&bitmap);
    assert_eq!(result, SkinType::Old);
}

#[test]
fn test_get_text_type_new_skin() {
    // 新版皮肤: 64x64，但不是纤细手臂
    let mut bitmap = create_test_bitmap(64, 64, Color::WHITE);

    // 在纤细手臂检测区域填充非透明色（表示不是纤细）
    let scale = 1;
    set_pixel_area(&mut bitmap, 50 * scale, 16 * scale, 2 * scale, 4 * scale, Color::WHITE);
    set_pixel_area(&mut bitmap, 54 * scale, 20 * scale, 2 * scale, 12 * scale, Color::WHITE);
    set_pixel_area(&mut bitmap, 42 * scale, 48 * scale, 2 * scale, 4 * scale, Color::WHITE);
    set_pixel_area(&mut bitmap, 46 * scale, 52 * scale, 2 * scale, 12 * scale, Color::WHITE);

    let result = skin_type_checker::get_text_type(&bitmap);
    assert_eq!(result, SkinType::New);
}

#[test]
fn test_get_text_type_slim_skin() {
    // 纤细手臂皮肤: 64x64，手臂区域透明
    let mut bitmap = create_test_bitmap(64, 64, Color::WHITE);

    // 在纤细手臂检测区域设置透明色
    let scale = 1;
    set_pixel_area(&mut bitmap, 50 * scale, 16 * scale, 2 * scale, 4 * scale, Color::TRANSPARENT);
    set_pixel_area(&mut bitmap, 54 * scale, 20 * scale, 2 * scale, 12 * scale, Color::TRANSPARENT);
    set_pixel_area(&mut bitmap, 42 * scale, 48 * scale, 2 * scale, 4 * scale, Color::TRANSPARENT);
    set_pixel_area(&mut bitmap, 46 * scale, 52 * scale, 2 * scale, 12 * scale, Color::TRANSPARENT);

    let result = skin_type_checker::get_text_type(&bitmap);
    assert_eq!(result, SkinType::NewSlim);
}

#[test]
fn test_get_text_type_unknown() {
    // 未知尺寸
    let bitmap = create_test_bitmap(32, 32, Color::WHITE);
    let result = skin_type_checker::get_text_type(&bitmap);
    assert_eq!(result, SkinType::Unknown);
}

#[test]
fn test_get_text_type_small_unknown() {
    // 非常小的尺寸
    let bitmap = create_test_bitmap(16, 16, Color::WHITE);
    let result = skin_type_checker::get_text_type(&bitmap);
    assert_eq!(result, SkinType::Unknown);
}

#[test]
fn test_get_text_type_large_old() {
    // 大尺寸旧版: 128x64
    let bitmap = create_test_bitmap(128, 64, Color::WHITE);
    let result = skin_type_checker::get_text_type(&bitmap);
    assert_eq!(result, SkinType::Old);
}

#[test]
fn test_get_text_type_large_new() {
    // 大尺寸新版: 128x128
    let mut bitmap = create_test_bitmap(128, 128, Color::WHITE);

    // 填充手臂区域为非透明
    let scale = 2;
    set_pixel_area(&mut bitmap, 50 * scale, 16 * scale, 2 * scale, 4 * scale, Color::WHITE);
    set_pixel_area(&mut bitmap, 54 * scale, 20 * scale, 2 * scale, 12 * scale, Color::WHITE);
    set_pixel_area(&mut bitmap, 42 * scale, 48 * scale, 2 * scale, 4 * scale, Color::WHITE);
    set_pixel_area(&mut bitmap, 46 * scale, 52 * scale, 2 * scale, 12 * scale, Color::WHITE);

    let result = skin_type_checker::get_text_type(&bitmap);
    assert_eq!(result, SkinType::New);
}

#[test]
fn test_get_text_type_large_slim() {
    // 大尺寸纤细手臂: 128x128
    let mut bitmap = create_test_bitmap(128, 128, Color::WHITE);

    // 在纤细手臂检测区域设置透明色
    let scale = 2;
    set_pixel_area(&mut bitmap, 50 * scale, 16 * scale, 2 * scale, 4 * scale, Color::TRANSPARENT);
    set_pixel_area(&mut bitmap, 54 * scale, 20 * scale, 2 * scale, 12 * scale, Color::TRANSPARENT);
    set_pixel_area(&mut bitmap, 42 * scale, 48 * scale, 2 * scale, 4 * scale, Color::TRANSPARENT);
    set_pixel_area(&mut bitmap, 46 * scale, 52 * scale, 2 * scale, 12 * scale, Color::TRANSPARENT);

    let result = skin_type_checker::get_text_type(&bitmap);
    assert_eq!(result, SkinType::NewSlim);
}

#[test]
fn test_get_text_type_64x32_old() {
    // 64x32 是旧版
    let bitmap = create_test_bitmap(64, 32, Color::WHITE);
    assert_eq!(skin_type_checker::get_text_type(&bitmap), SkinType::Old);
}

#[test]
fn test_get_text_type_64x64_new() {
    // 64x64 是新版
    let mut bitmap = create_test_bitmap(64, 64, Color::WHITE);
    // 填充手臂区域为非透明
    set_pixel_area(&mut bitmap, 50, 16, 2, 4, Color::WHITE);
    set_pixel_area(&mut bitmap, 54, 20, 2, 12, Color::WHITE);
    set_pixel_area(&mut bitmap, 42, 48, 2, 4, Color::WHITE);
    set_pixel_area(&mut bitmap, 46, 52, 2, 12, Color::WHITE);
    assert_eq!(skin_type_checker::get_text_type(&bitmap), SkinType::New);
}
