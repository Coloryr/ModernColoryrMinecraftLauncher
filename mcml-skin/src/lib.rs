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

pub mod skin {
    use skia_safe::image::CachingHint;
    use skia_safe::{AlphaType, Bitmap, ColorType, Data, EncodedImageFormat, Image, ImageInfo};
    use std::fs::File;
    use std::io::Write;
    use std::path::Path;
    use std::slice;

    pub fn open_bitmap(file: &Path) -> Option<Bitmap> {
        let data = Data::from_filename(file);
        if data.is_none() {
            return None;
        }
        let data = data.unwrap();
        let image = Image::from_encoded(data);
        if image.is_none() {
            return None;
        }
        let image = image.unwrap();
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
}
