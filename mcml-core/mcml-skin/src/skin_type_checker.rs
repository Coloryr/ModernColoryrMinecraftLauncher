use skia_safe::{Bitmap, Color, IPoint};

use crate::SkinType;

/// 获取皮肤类型
pub fn get_skin_type(image: &Bitmap) -> SkinType {
    let width = image.width();
    let height = image.height();

    if width >= 64 && height >= 64 && width == height {
        if is_slim_skin(image) {
            SkinType::NewSlim
        } else {
            SkinType::New
        }
    } else if width == height * 2 {
        SkinType::Old
    } else {
        SkinType::Unknown
    }
}

/// 是否为1.8新版皮肤（纤细手臂）
fn is_slim_skin(image: &Bitmap) -> bool {
    let scale = image.width() / 64;

    // 检查右臂上方的透明像素
    check_pixel_area(image, 50 * scale, 16 * scale, 2 * scale, 4 * scale, &[Color::TRANSPARENT])
            // 检查右臂下方的透明像素
            && check_pixel_area(image, 54 * scale, 20 * scale, 2 * scale, 12 * scale, &[Color::TRANSPARENT])
            // 检查左臂上方的透明像素
            && check_pixel_area(image, 42 * scale, 48 * scale, 2 * scale, 4 * scale, &[Color::TRANSPARENT])
            // 检查左臂下方的透明像素
            && check_pixel_area(image, 46 * scale, 52 * scale, 2 * scale, 12 * scale, &[Color::TRANSPARENT])
}

/// 检查像素区域是否所有像素都匹配指定颜色
fn check_pixel_area(image: &Bitmap, x: i32, y: i32, w: i32, h: i32, colors: &[Color]) -> bool {
    // 边界检查
    if x < 0 || y < 0 || x + w > image.width() || y + h > image.height() {
        return false;
    }

    for wi in 0..w {
        for hi in 0..h {
            let pixel = image.get_color(IPoint::new(x + wi, y + hi));
            if !colors.contains(&pixel) {
                return false;
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use skia_safe::{AlphaType, ColorType, ImageInfo};

    /// 创建指定尺寸的位图，并对每个像素调用填充函数得到颜色
    fn make_bitmap(w: i32, h: i32, fill: impl Fn(i32, i32) -> Color) -> Bitmap {
        let info = ImageInfo::new((w, h), ColorType::RGBA8888, AlphaType::Premul, None);
        let mut bm = Bitmap::new();
        assert!(bm.set_info(&info, None), "set_info 失败");
        bm.alloc_pixels();
        let row = bm.row_bytes() as usize;
        let bpp = bm.bytes_per_pixel() as usize;
        let ptr = bm.pixels() as *mut u8;
        assert!(!ptr.is_null(), "位图像素未分配");
        unsafe {
            for y in 0..h {
                for x in 0..w {
                    let off = y as usize * row + x as usize * bpp;
                    let p = std::slice::from_raw_parts_mut(ptr.add(off), bpp);
                    let c = fill(x, y);
                    // 写入字节按 RGBA 顺序（RGBA8888 内存布局）
                    p[0] = c.r();
                    p[1] = c.g();
                    p[2] = c.b();
                    p[3] = c.a();
                }
            }
        }
        bm
    }

    /// 64x64 全不透明：右臂/左臂标记区域不透明，应识别为新版皮肤
    #[test]
    fn test_get_skin_type_new() {
        let bm = make_bitmap(64, 64, |_, _| Color::from_rgb(255, 0, 0));
        assert_eq!(get_skin_type(&bm), SkinType::New);
    }

    /// 64x64 全透明：纤细标记区域全透明，应识别为新版纤细
    #[test]
    fn test_get_skin_type_new_slim() {
        let bm = make_bitmap(64, 64, |_, _| Color::TRANSPARENT);
        assert_eq!(get_skin_type(&bm), SkinType::NewSlim);
    }

    /// 128x128 全透明：放大两倍的新版纤细皮肤
    #[test]
    fn test_get_skin_type_new_slim_scaled() {
        let bm = make_bitmap(128, 128, |_, _| Color::TRANSPARENT);
        assert_eq!(get_skin_type(&bm), SkinType::NewSlim);
    }

    /// 纤细标记区域不透明、其余透明：应识别为新版（非纤细）
    #[test]
    fn test_get_skin_type_slim_marker_opaque_is_new() {
        let bm = make_bitmap(64, 64, |x, y| {
            // 纤维判定区域：右臂 (50,16,2,4)、(54,20,2,12)，左臂 (42,48,2,4)、(46,52,2,12)
            let in_marker = (x >= 50 && x < 52 && y >= 16 && y < 20)
                || (x >= 54 && x < 56 && y >= 20 && y < 32)
                || (x >= 42 && x < 44 && y >= 48 && y < 52)
                || (x >= 46 && x < 48 && y >= 52 && y < 64);
            if in_marker {
                Color::from_rgb(0, 255, 0)
            } else {
                Color::TRANSPARENT
            }
        });
        assert_eq!(get_skin_type(&bm), SkinType::New);
    }

    /// 64x32（宽为高两倍）：应识别为 1.7 旧版皮肤
    #[test]
    fn test_get_skin_type_old() {
        let bm = make_bitmap(64, 32, |_, _| Color::from_rgb(0, 0, 255));
        assert_eq!(get_skin_type(&bm), SkinType::Old);
    }

    /// 尺寸不符合任何已知格式（32x64）应返回 Unknown
    #[test]
    fn test_get_skin_type_unknown() {
        let bm = make_bitmap(32, 64, |_, _| Color::from_rgb(0, 0, 255));
        assert_eq!(get_skin_type(&bm), SkinType::Unknown);
    }

    /// 63x63 小于 64 的正方形也不匹配旧版比例，应为 Unknown
    #[test]
    fn test_get_skin_type_unknown_small() {
        let bm = make_bitmap(63, 63, |_, _| Color::from_rgb(0, 0, 255));
        assert_eq!(get_skin_type(&bm), SkinType::Unknown);
    }

    /// 测试 check_pixel_area 的边界检查：区域越界应返回 false
    #[test]
    fn test_check_pixel_area_out_of_bounds() {
        let bm = make_bitmap(8, 8, |_, _| Color::from_rgb(255, 255, 255));
        assert!(!check_pixel_area(&bm, 6, 6, 4, 4, &[Color::from_rgb(255, 255, 255)]));
        assert!(!check_pixel_area(&bm, -1, 0, 2, 2, &[Color::from_rgb(255, 255, 255)]));
        assert!(!check_pixel_area(&bm, 0, -1, 2, 2, &[Color::from_rgb(255, 255, 255)]));
        // 区域完全在界内且颜色匹配应返回 true
        assert!(check_pixel_area(&bm, 0, 0, 4, 4, &[Color::from_rgb(255, 255, 255)]));
        // 颜色不匹配应返回 false
        assert!(!check_pixel_area(&bm, 0, 0, 2, 2, &[Color::TRANSPARENT]));
    }
}
