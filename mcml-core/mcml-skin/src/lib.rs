use skia_safe::image::CachingHint;
use skia_safe::{AlphaType, Bitmap, ColorType, Data, EncodedImageFormat, Image, ImageInfo};
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::slice;

pub mod skin_type_checker;

/// 皮肤类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkinType {
    /// 1.7旧版
    Old,
    /// 1.8新版
    New,
    /// 1.8新版纤细
    NewSlim,
    /// 未知的类型
    Unknown,
}

pub fn open_bitmap(file: &Path) -> Option<Bitmap> {
    let data = Data::from_filename(file)?;
    let image = Image::from_encoded(data)?;
    let info = ImageInfo::new(
        image.dimensions(),
        ColorType::RGBA8888,
        AlphaType::Premul,
        None,
    );
    let mut bitmap = Bitmap::new();
    if !bitmap.set_info(&info, None) {
        return None;
    }
    bitmap.alloc_pixels();
    let size = bitmap.compute_byte_size();
    let pixels = unsafe { slice::from_raw_parts_mut(bitmap.pixels() as *mut u8, size) };
    if !image.read_pixels(
        &info,
        pixels,
        bitmap.row_bytes(),
        (0, 0),
        CachingHint::Disallow,
    ) {
        return None;
    }
    Some(bitmap)
}

pub fn save_bitmap(image: &Bitmap, file: &Path) {
    let file = File::create(file);
    assert!(file.is_ok());
    let mut file = file.unwrap();
    let data = image.encode(EncodedImageFormat::PNG, 100);
    assert!(data.is_some());
    let data = data.unwrap();
    assert!(file.write_all(&data).is_ok());
}

pub fn save_image(image: &Image, file: &Path) {
    let file = File::create(file);
    assert!(file.is_ok());
    let mut file = file.unwrap();
    let data = image.encode(None, EncodedImageFormat::PNG, 100);
    assert!(data.is_some());
    let data = data.unwrap();
    assert!(file.write_all(&data).is_ok());
}

#[cfg(test)]
mod tests {
    use super::*;
    use skia_safe::{AlphaType, Color, ColorType, IPoint, ImageInfo};

    /// 创建一个纯色填充的 RGBA8888 位图（全平台可跑，不依赖 GPU）
    fn make_bitmap(w: i32, h: i32, color: Color) -> Bitmap {
        let info = ImageInfo::new((w, h), ColorType::RGBA8888, AlphaType::Premul, None);
        let mut bm = Bitmap::new();
        assert!(bm.set_info(&info, None), "set_info 失败");
        bm.alloc_pixels();
        let row = bm.row_bytes() as usize;
        let bpp = bm.bytes_per_pixel() as usize;
        let ptr = bm.pixels() as *mut u8;
        assert!(!ptr.is_null(), "位图像素未分配");
        // 不透明颜色的 premul 值与原色一致，可以直接写入
        assert_eq!(color.a(), 255, "测试用颜色必须不透明，避免 premul 干扰");
        unsafe {
            for y in 0..h {
                for x in 0..w {
                    let off = y as usize * row + x as usize * bpp;
                    let p = std::slice::from_raw_parts_mut(ptr.add(off), bpp);
                    p[0] = color.r();
                    p[1] = color.g();
                    p[2] = color.b();
                    p[3] = color.a();
                }
            }
        }
        bm
    }

    /// 测试 SkinType 枚举的基本相等性
    #[test]
    fn test_skin_type_equality() {
        assert_ne!(SkinType::Old, SkinType::New);
        assert_ne!(SkinType::New, SkinType::NewSlim);
        assert_eq!(SkinType::Unknown, SkinType::Unknown);
    }

    /// 测试 save_bitmap / open_bitmap 的 PNG 存取往返
    #[test]
    fn test_save_and_open_bitmap_round_trip() {
        let red = Color::from_rgb(255, 0, 0);
        let bm = make_bitmap(64, 32, red);

        // 临时目录中的唯一文件名（按进程 ID 区分，测完清理）
        let path = std::env::temp_dir().join(format!("mcml_skin_roundtrip_{}.png", std::process::id()));
        save_bitmap(&bm, &path);

        let loaded = open_bitmap(&path);
        // 无论断言结果如何都清理临时文件
        let result = loaded.map(|b| {
            assert_eq!(b.width(), 64);
            assert_eq!(b.height(), 32);
            // 重新读出的像素颜色应与写入时一致
            assert_eq!(b.get_color(IPoint::new(10, 10)), red);
            assert_eq!(b.get_color(IPoint::new(63, 31)), red);
        });
        let _ = std::fs::remove_file(&path);

        // open_bitmap 应成功返回
        assert!(result.is_some(), "open_bitmap 应能读回保存的 PNG");

        // open_bitmap 对不存在的文件应返回 None
        let missing = std::env::temp_dir().join(format!("mcml_skin_missing_{}.png", std::process::id()));
        assert!(open_bitmap(&missing).is_none());
    }

    /// 测试 open_bitmap 对非图片文件返回 None
    #[test]
    fn test_open_bitmap_invalid_file() {
        let path = std::env::temp_dir().join(format!("mcml_skin_invalid_{}.txt", std::process::id()));
        std::fs::write(&path, b"this is not an image").unwrap();
        let result = open_bitmap(&path);
        let _ = std::fs::remove_file(&path);
        assert!(result.is_none(), "非图片内容应返回 None");
    }
}
