pub mod skin_type_checker {
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
}
