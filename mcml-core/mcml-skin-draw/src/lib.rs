pub mod cape_2d_draw;
pub mod head_2d_draw;
pub mod head_3d_draw;
pub mod skin_2d_draw;

pub mod skin_draw {
    use skia_safe::{Bitmap, Color, ImageInfo};

    pub const SCALE_TYPEA: usize = 16;
    pub const SCALE_TYPEB: usize = 2;
    pub const SCALE_TYPEC: usize = 8;

    pub fn draw(
        dest: &mut Bitmap,
        source: &mut Bitmap,
        dest_x: i32,
        dest_y: i32,
        src_x: i32,
        src_y: i32,
        width: i32,
        height: i32,
    ) -> Option<()> {
        // 参数验证
        if width <= 0 || height <= 0 {
            return Some(());
        }

        if dest_x < 0
            || dest_y < 0
            || dest_x + width > dest.width()
            || dest_y + height > dest.height()
            || src_x < 0
            || src_y < 0
            || src_x + width > source.width()
            || src_y + height > source.height()
        {
            return None;
        }

        let src_ptr = source.pixels() as *const u8;
        let dst_ptr = dest.pixels() as *mut u8;
        if src_ptr.is_null() || dst_ptr.is_null() {
            return None;
        }

        // 获取源像素指针（只读）
        let src_ptr = source.pixels();
        let dst_ptr = dest.pixels();
        if src_ptr.is_null() || dst_ptr.is_null() {
            return None;
        }

        let src_ptr = src_ptr as *const u8;
        let dst_ptr = dst_ptr as *mut u8;

        let src_row_bytes = source.row_bytes() as usize;
        let dst_row_bytes = dest.row_bytes() as usize;
        let bytes_per_pixel = (source.bytes_per_pixel()) as usize;

        // 批量复制每行数据
        for y in 0..height {
            let src_offset =
                ((src_y + y) as usize) * src_row_bytes + (src_x as usize) * bytes_per_pixel;
            let dst_offset =
                ((dest_y + y) as usize) * dst_row_bytes + (dest_x as usize) * bytes_per_pixel;

            unsafe {
                let src_slice = std::slice::from_raw_parts(
                    src_ptr.add(src_offset),
                    (width as usize) * bytes_per_pixel,
                );
                let dst_slice = std::slice::from_raw_parts_mut(
                    dst_ptr.add(dst_offset),
                    (width as usize) * bytes_per_pixel,
                );
                dst_slice.copy_from_slice(src_slice);
            }
        }

        Some(())
    }

    // 颜色混合 trait
    pub trait ColorMix {
        fn mix(&self, other: &Color) -> Color;
    }

    impl ColorMix for Color {
        fn mix(&self, other: &Color) -> Color {
            let ap = other.a() as f32 / 255.0;
            let dp = 1.0 - ap;

            let out_r = other.r() as f32 * ap + self.r() as f32 * dp;
            let out_g = other.g() as f32 * ap + self.g() as f32 * dp;
            let out_b = other.b() as f32 * ap + self.b() as f32 * dp;

            if self.a() == 0 && other.a() == 0 {
                Color::from_argb(0, out_r as u8, out_g as u8, out_b as u8)
            } else {
                Color::from_rgb(out_r as u8, out_g as u8, out_b as u8)
            }
        }
    }

    pub fn draw_mix(
        dest: &mut Bitmap,
        source: &mut Bitmap,
        dest_x: i32,
        dest_y: i32,
        src_x: i32,
        src_y: i32,
        width: i32,
        height: i32,
    ) -> Option<()> {
        // 参数验证
        if width <= 0 || height <= 0 {
            return Some(());
        }

        if dest_x < 0
            || dest_y < 0
            || dest_x + width > dest.width()
            || dest_y + height > dest.height()
            || src_x < 0
            || src_y < 0
            || src_x + width > source.width()
            || src_y + height > source.height()
        {
            return None;
        }

        let src_ptr = source.pixels() as *const u8;
        let dst_ptr = dest.pixels() as *mut u8;
        if src_ptr.is_null() || dst_ptr.is_null() {
            return None;
        }

        let src_row_bytes = source.row_bytes() as usize;
        let dst_row_bytes = dest.row_bytes() as usize;
        let bytes_per_pixel = source.bytes_per_pixel() as usize;

        // 执行混合
        for j in 0..height {
            for i in 0..width {
                let src_offset = ((src_y + j) as usize) * src_row_bytes
                    + ((src_x + i) as usize) * bytes_per_pixel;
                let dst_offset = ((dest_y + j) as usize) * dst_row_bytes
                    + ((dest_x + i) as usize) * bytes_per_pixel;

                unsafe {
                    let src_slice =
                        std::slice::from_raw_parts(src_ptr.add(src_offset), bytes_per_pixel);
                    let dst_slice =
                        std::slice::from_raw_parts_mut(dst_ptr.add(dst_offset), bytes_per_pixel);

                    let src_color =
                        Color::from_argb(src_slice[3], src_slice[2], src_slice[1], src_slice[0]);
                    let dst_color =
                        Color::from_argb(dst_slice[3], dst_slice[2], dst_slice[1], dst_slice[0]);

                    let mixed = dst_color.mix(&src_color);

                    dst_slice[0] = mixed.b();
                    dst_slice[1] = mixed.g();
                    dst_slice[2] = mixed.r();
                    dst_slice[3] = mixed.a();
                }
            }
        }

        Some(())
    }

    pub fn scale(source: &mut Bitmap, scale: usize) -> Option<Bitmap> {
        let src_width = source.width() as usize;
        let src_height = source.height() as usize;
        let dst_width = src_width * scale;
        let dst_height = src_height * scale;

        // 获取源像素数据
        let src_ptr = source.pixels();
        let src_row_bytes = source.row_bytes() as usize;
        let bytes_per_pixel = source.bytes_per_pixel() as usize;

        // 创建目标 Bitmap
        let mut dst = Bitmap::new();
        let image_info = ImageInfo::new(
            (dst_width as i32, dst_height as i32),
            source.color_type(),
            source.alpha_type(),
            source.color_space().map(|cs| cs.clone()),
        );

        if !dst.set_info(&image_info, None) {
            return None;
        }
        dst.alloc_pixels();

        let dst_ptr = dst.pixels();
        let dst_row_bytes = dst.row_bytes() as usize;

        if src_ptr.is_null() || dst_ptr.is_null() {
            return None;
        }

        let src_ptr = src_ptr as *const u8;
        let dst_ptr = dst_ptr as *mut u8;

        let total_src_size = src_row_bytes * src_height;
        let total_dst_size = dst_row_bytes * dst_height;

        // 最近邻缩放（批量复制）
        unsafe {
            let src_data = std::slice::from_raw_parts(src_ptr, total_src_size);
            let dst_data = std::slice::from_raw_parts_mut(dst_ptr, total_dst_size);

            for src_y in 0..src_height {
                // 源行数据
                let src_row_offset = src_y * src_row_bytes;

                // 目标行范围（每个源行重复 scale 次）
                for repeat_y in 0..scale {
                    let dst_y = src_y * scale + repeat_y;
                    let dst_row_offset = dst_y * dst_row_bytes;

                    // 处理当前行的每个像素
                    for src_x in 0..src_width {
                        let src_offset = src_row_offset + src_x * bytes_per_pixel;
                        let color_slice = &src_data[src_offset..src_offset + bytes_per_pixel];

                        // 每个源像素重复 scale 次
                        for repeat_x in 0..scale {
                            let dst_x = src_x * scale + repeat_x;
                            let dst_offset = dst_row_offset + dst_x * bytes_per_pixel;

                            dst_data[dst_offset..dst_offset + bytes_per_pixel]
                                .copy_from_slice(color_slice);
                        }
                    }
                }
            }
        }

        Some(dst)
    }

    /// 在指定区域填充颜色
    pub fn fill_image(
        dest: &mut Bitmap,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        pix: Color,
    ) -> Option<()> {
        if width <= 0 || height <= 0 {
            return Some(());
        }

        if x < 0 || y < 0 || x + width > dest.width() || y + height > dest.height() {
            return None;
        }

        let dst_ptr = dest.pixels() as *mut u8;
        if dst_ptr.is_null() {
            return None;
        }

        let dst_row_bytes = dest.row_bytes() as usize;
        let bytes_per_pixel = dest.bytes_per_pixel() as usize;

        // 将 Color 转换为 BGRA 字节
        let color_bytes = [pix.b(), pix.g(), pix.r(), pix.a()];

        unsafe {
            for j in 0..height {
                let dst_offset =
                    ((y + j) as usize) * dst_row_bytes + (x as usize) * bytes_per_pixel;
                let dst_slice = std::slice::from_raw_parts_mut(
                    dst_ptr.add(dst_offset),
                    (width as usize) * bytes_per_pixel,
                );
                for i in 0..width {
                    let offset = (i as usize) * bytes_per_pixel;
                    dst_slice[offset..offset + bytes_per_pixel].copy_from_slice(&color_bytes);
                }
            }
        }

        Some(())
    }

    /// 带混合的填充
    pub fn fill_image_mix(
        dest: &mut Bitmap,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        pix: Color,
    ) -> Option<()> {
        if width <= 0 || height <= 0 {
            return Some(());
        }

        if x < 0 || y < 0 || x + width > dest.width() || y + height > dest.height() {
            return None;
        }

        let dst_ptr = dest.pixels() as *mut u8;
        if dst_ptr.is_null() {
            return None;
        }

        let dst_row_bytes = dest.row_bytes() as usize;
        let bytes_per_pixel = dest.bytes_per_pixel() as usize;

        unsafe {
            for j in 0..height {
                for i in 0..width {
                    let dst_offset =
                        ((y + j) as usize) * dst_row_bytes + ((x + i) as usize) * bytes_per_pixel;
                    let dst_slice =
                        std::slice::from_raw_parts_mut(dst_ptr.add(dst_offset), bytes_per_pixel);

                    let dst_color =
                        Color::from_argb(dst_slice[3], dst_slice[2], dst_slice[1], dst_slice[0]);

                    let mixed = dst_color.mix(&pix);

                    dst_slice[0] = mixed.b();
                    dst_slice[1] = mixed.g();
                    dst_slice[2] = mixed.r();
                    dst_slice[3] = mixed.a();
                }
            }
        }

        Some(())
    }

    /// 复制像素到指定区域，同时每个像素都以填充方式填充
    pub fn draw_with_fill_image(
        dest: &mut Bitmap,
        source: &mut Bitmap,
        x: i32,
        y: i32,
        sx: i32,
        sy: i32,
        swidth: i32,
        sheight: i32,
        width: i32,
        height: i32,
    ) -> Option<()> {
        if swidth <= 0 || sheight <= 0 || width <= 0 || height <= 0 {
            return Some(());
        }

        if x < 0
            || y < 0
            || sx < 0
            || sy < 0
            || sx + swidth > source.width()
            || sy + sheight > source.height()
            || x + swidth * width > dest.width()
            || y + sheight * height > dest.height()
        {
            return None;
        }

        let src_ptr = source.pixels() as *const u8;
        let dst_ptr = dest.pixels() as *mut u8;
        if src_ptr.is_null() || dst_ptr.is_null() {
            return None;
        }

        let src_row_bytes = source.row_bytes() as usize;
        let dst_row_bytes = dest.row_bytes() as usize;
        let bytes_per_pixel = source.bytes_per_pixel() as usize;

        unsafe {
            for i in 0..swidth {
                for j in 0..sheight {
                    // 读取源像素
                    let src_offset =
                        ((sy + j) as usize) * src_row_bytes + ((sx + i) as usize) * bytes_per_pixel;
                    let src_slice =
                        std::slice::from_raw_parts(src_ptr.add(src_offset), bytes_per_pixel);

                    // 在目标区域填充 width x height 块
                    let dest_x = i * width + x;
                    let dest_y = j * height + y;
                    for fy in 0..height {
                        let dst_offset = ((dest_y + fy) as usize) * dst_row_bytes
                            + (dest_x as usize) * bytes_per_pixel;
                        let dst_slice = std::slice::from_raw_parts_mut(
                            dst_ptr.add(dst_offset),
                            (width as usize) * bytes_per_pixel,
                        );
                        for fx in 0..width {
                            let offset = (fx as usize) * bytes_per_pixel;
                            dst_slice[offset..offset + bytes_per_pixel].copy_from_slice(src_slice);
                        }
                    }
                }
            }
        }

        Some(())
    }

    /// 复制像素到指定区域，同时每个像素都以填充方式填充混合
    pub fn draw_with_fill_image_mix(
        dest: &mut Bitmap,
        source: &mut Bitmap,
        x: i32,
        y: i32,
        sx: i32,
        sy: i32,
        swidth: i32,
        sheight: i32,
        width: i32,
        height: i32,
    ) -> Option<()> {
        if swidth <= 0 || sheight <= 0 || width <= 0 || height <= 0 {
            return Some(());
        }

        if x < 0
            || y < 0
            || sx < 0
            || sy < 0
            || sx + swidth > source.width()
            || sy + sheight > source.height()
            || x + swidth * width > dest.width()
            || y + sheight * height > dest.height()
        {
            return None;
        }

        let src_ptr = source.pixels() as *const u8;
        let dst_ptr = dest.pixels() as *mut u8;
        if src_ptr.is_null() || dst_ptr.is_null() {
            return None;
        }

        let src_row_bytes = source.row_bytes() as usize;
        let dst_row_bytes = dest.row_bytes() as usize;
        let bytes_per_pixel = source.bytes_per_pixel() as usize;

        unsafe {
            for i in 0..swidth {
                for j in 0..sheight {
                    // 读取源像素
                    let src_offset =
                        ((sy + j) as usize) * src_row_bytes + ((sx + i) as usize) * bytes_per_pixel;
                    let src_slice =
                        std::slice::from_raw_parts(src_ptr.add(src_offset), bytes_per_pixel);
                    let src_color =
                        Color::from_argb(src_slice[3], src_slice[2], src_slice[1], src_slice[0]);

                    // 在目标区域填充混合 width x height 块
                    let dest_x = i * width + x;
                    let dest_y = j * height + y;
                    for fy in 0..height {
                        for fx in 0..width {
                            let dst_offset = ((dest_y + fy) as usize) * dst_row_bytes
                                + ((dest_x + fx) as usize) * bytes_per_pixel;
                            let dst_slice = std::slice::from_raw_parts_mut(
                                dst_ptr.add(dst_offset),
                                bytes_per_pixel,
                            );

                            let dst_color = Color::from_argb(
                                dst_slice[3],
                                dst_slice[2],
                                dst_slice[1],
                                dst_slice[0],
                            );

                            let mixed = dst_color.mix(&src_color);

                            dst_slice[0] = mixed.b();
                            dst_slice[1] = mixed.g();
                            dst_slice[2] = mixed.r();
                            dst_slice[3] = mixed.a();
                        }
                    }
                }
            }
        }

        Some(())
    }
}

/// 注意：下面的混合函数按 BGRA 字节序解释像素内存（byte0 = B），
/// 而 RGBA8888 位图的内存布局是 R,G,B,A（byte0 = R），
/// 两者通道命名相反，但由于所有混合运算都是按字节逐通道对称进行的，
/// 除了 alpha（第 4 字节）外不会产生错误结果。
/// 因此本文件中的测试一律直接比较原始字节，而不是比较语义上的 R/G/B 颜色。
#[cfg(test)]
mod tests {
    use skia_safe::{AlphaType, Bitmap, Color, ColorType, ImageInfo};

    use super::skin_draw::*;

    /// 创建 RGBA8888 位图，并对每个像素调用填充函数得到 4 字节（按内存顺序）
    fn make_bitmap(w: i32, h: i32, fill: impl Fn(i32, i32) -> [u8; 4]) -> Bitmap {
        let info = ImageInfo::new((w, h), ColorType::RGBA8888, AlphaType::Premul, None);
        let mut bm = Bitmap::new();
        assert!(bm.set_info(&info, None), "set_info 失败");
        bm.alloc_pixels();
        let row = bm.row_bytes() as usize;
        let bpp = bm.bytes_per_pixel() as usize;
        assert_eq!(bpp, 4, "测试假设每像素 4 字节");
        let ptr = bm.pixels() as *mut u8;
        assert!(!ptr.is_null(), "位图像素未分配");
        unsafe {
            for y in 0..h {
                for x in 0..w {
                    let off = y as usize * row + x as usize * bpp;
                    let p = std::slice::from_raw_parts_mut(ptr.add(off), bpp);
                    p.copy_from_slice(&fill(x, y));
                }
            }
        }
        bm
    }

    /// 读取位图某像素的原始 4 字节
    fn get_bytes(bm: &mut Bitmap, x: i32, y: i32) -> [u8; 4] {
        let row = bm.row_bytes() as usize;
        let bpp = bm.bytes_per_pixel() as usize;
        let ptr = bm.pixels() as *const u8;
        assert!(!ptr.is_null());
        unsafe {
            let off = y as usize * row + x as usize * bpp;
            let p = std::slice::from_raw_parts(ptr.add(off), bpp);
            [p[0], p[1], p[2], p[3]]
        }
    }

    /// 根据坐标生成一个唯一且不透明的字节模式（避免全图同色掩盖拷贝错位）
    fn pattern(x: i32, y: i32) -> [u8; 4] {
        [(x * 7 + 1) as u8, (y * 11 + 2) as u8, (x * 13 + 3) as u8, 255]
    }

    /// draw 应按行复制源区域像素到目标位置
    #[test]
    fn test_draw_copies_region() {
        let mut src = make_bitmap(8, 8, pattern);
        let mut dst = make_bitmap(8, 8, |_, _| [0, 0, 0, 0]);

        // 整图复制
        let r = draw(&mut dst, &mut src, 0, 0, 0, 0, 8, 8);
        assert!(r.is_some());
        for y in 0..8 {
            for x in 0..8 {
                assert_eq!(get_bytes(&mut dst, x, y), pattern(x, y), "整图复制 ({x},{y})");
            }
        }
    }

    /// draw 带偏移的子区域复制
    #[test]
    fn test_draw_sub_region_with_offset() {
        let mut src = make_bitmap(8, 8, pattern);
        let mut dst = make_bitmap(8, 8, |_, _| [0, 0, 0, 0]);

        // 源区域 (2,3) 4x5 复制到目标 (1,1)
        let r = draw(&mut dst, &mut src, 1, 1, 2, 3, 4, 5);
        assert!(r.is_some());
        for j in 0..5 {
            for i in 0..4 {
                assert_eq!(
                    get_bytes(&mut dst, 1 + i, 1 + j),
                    pattern(2 + i, 3 + j),
                    "子区域复制 ({i},{j})"
                );
            }
        }
        // 目标其他位置应保持为 0
        assert_eq!(get_bytes(&mut dst, 0, 0), [0, 0, 0, 0]);
        assert_eq!(get_bytes(&mut dst, 7, 7), [0, 0, 0, 0]);
    }

    /// draw 越界应返回 None，尺寸为 0 是无操作返回 Some
    #[test]
    fn test_draw_bounds_and_zero_size() {
        let mut src = make_bitmap(8, 8, pattern);
        let mut dst = make_bitmap(8, 8, |_, _| [0, 0, 0, 0]);

        // 目标越界
        assert!(draw(&mut dst, &mut src, 5, 5, 0, 0, 4, 4).is_none());
        // 源越界
        assert!(draw(&mut dst, &mut src, 0, 0, 5, 5, 4, 4).is_none());
        // 负坐标
        assert!(draw(&mut dst, &mut src, -1, 0, 0, 0, 2, 2).is_none());
        assert!(draw(&mut dst, &mut src, 0, 0, -1, 0, 2, 2).is_none());
        // 尺寸为 0：无操作，返回 Some
        assert!(draw(&mut dst, &mut src, 0, 0, 0, 0, 0, 8).is_some());
        // 目标未被修改
        assert_eq!(get_bytes(&mut dst, 3, 3), [0, 0, 0, 0]);
    }

    /// ColorMix：不透明源完全覆盖，半透明按比例混合，双方都透明时结果透明
    #[test]
    fn test_color_mix() {
        // 不透明源覆盖：黑底 + 白源 = 白
        let mixed = Color::BLACK.mix(&Color::WHITE);
        assert_eq!(mixed, Color::WHITE);

        // 半透明混合：黑底 + 50% 白源，各通道约为 128
        let half = Color::from_argb(128, 255, 255, 255);
        let mixed = Color::BLACK.mix(&half);
        assert!(
            (mixed.r() as i32 - 128).abs() <= 1,
            "半透明混合 r = {}",
            mixed.r()
        );
        assert_eq!(mixed.a(), 255, "混合后应为不透明");

        // 双方都透明：结果 alpha 为 0
        let t1 = Color::from_argb(0, 10, 20, 30);
        let t2 = Color::from_argb(0, 40, 50, 60);
        let mixed = t1.mix(&t2);
        assert_eq!(mixed.a(), 0, "双方透明时结果应透明");
    }

    /// draw_mix：不透明源覆盖目标，透明源保持目标不变
    #[test]
    fn test_draw_mix_overwrite_and_keep() {
        let mut src = make_bitmap(4, 4, |_, _| [10, 20, 30, 255]);
        let mut dst = make_bitmap(4, 4, |_, _| [200, 210, 220, 255]);

        // 不透明源覆盖目标
        assert!(draw_mix(&mut dst, &mut src, 0, 0, 0, 0, 4, 4).is_some());
        assert_eq!(get_bytes(&mut dst, 2, 2), [10, 20, 30, 255]);

        // 透明源不改变目标
        let mut transparent = make_bitmap(4, 4, |_, _| [0, 0, 0, 0]);
        assert!(draw_mix(&mut dst, &mut transparent, 0, 0, 0, 0, 4, 4).is_some());
        assert_eq!(get_bytes(&mut dst, 2, 2), [10, 20, 30, 255]);
    }

    /// draw_mix 的 alpha 混合：50% 透明源在黑底上得到约半值
    #[test]
    fn test_draw_mix_half_alpha() {
        let mut src = make_bitmap(1, 1, |_, _| [255, 255, 255, 128]);
        let mut dst = make_bitmap(1, 1, |_, _| [0, 0, 0, 255]);

        assert!(draw_mix(&mut dst, &mut src, 0, 0, 0, 0, 1, 1).is_some());
        let b = get_bytes(&mut dst, 0, 0);
        // 255 * (1 - 128/255) ≈ 127
        for ch in b.iter().take(3) {
            assert!(
                (*ch as i32 - 127).abs() <= 1,
                "半透明混合通道值 = {ch}"
            );
        }
        assert_eq!(b[3], 255, "混合在黑底上结果应不透明");
    }

    /// draw_mix 边界与 draw 一致
    #[test]
    fn test_draw_mix_bounds() {
        let mut src = make_bitmap(4, 4, pattern);
        let mut dst = make_bitmap(4, 4, |_, _| [0, 0, 0, 0]);
        assert!(draw_mix(&mut dst, &mut src, 3, 3, 0, 0, 2, 2).is_none());
        assert!(draw_mix(&mut dst, &mut src, 0, 0, 0, 0, 0, 0).is_some());
    }

    /// scale 最近邻缩放：每个源像素被复制为 scale x scale 块
    #[test]
    fn test_scale_nearest_neighbor() {
        let mut src = make_bitmap(2, 2, |x, y| {
            if x == 0 && y == 0 {
                [1, 2, 3, 255]
            } else if x == 1 && y == 0 {
                [4, 5, 6, 255]
            } else if x == 0 && y == 1 {
                [7, 8, 9, 255]
            } else {
                [10, 11, 12, 255]
            }
        });

        let mut dst = scale(&mut src, 2).expect("scale 应成功");
        assert_eq!(dst.width(), 4);
        assert_eq!(dst.height(), 4);

        // 每个源像素放大为 2x2 块
        assert_eq!(get_bytes(&mut dst, 0, 0), [1, 2, 3, 255]);
        assert_eq!(get_bytes(&mut dst, 1, 1), [1, 2, 3, 255]);
        assert_eq!(get_bytes(&mut dst, 2, 0), [4, 5, 6, 255]);
        assert_eq!(get_bytes(&mut dst, 3, 1), [4, 5, 6, 255]);
        assert_eq!(get_bytes(&mut dst, 0, 2), [7, 8, 9, 255]);
        assert_eq!(get_bytes(&mut dst, 1, 3), [7, 8, 9, 255]);
        assert_eq!(get_bytes(&mut dst, 2, 2), [10, 11, 12, 255]);
        assert_eq!(get_bytes(&mut dst, 3, 3), [10, 11, 12, 255]);

        // scale 为 1 时应得到内容相同的副本
        let mut same = scale(&mut src, 1).expect("scale 1 应成功");
        assert_eq!(same.width(), 2);
        assert_eq!(get_bytes(&mut same, 1, 0), [4, 5, 6, 255]);
    }

    /// fill_image 按字节填充区域（注意：函数按 BGRA 顺序写入颜色字节）
    #[test]
    fn test_fill_image() {
        let mut dst = make_bitmap(8, 8, |_, _| [0, 0, 0, 0]);
        let pix = Color::from_argb(255, 10, 20, 30);

        let r = fill_image(&mut dst, 2, 3, 4, 2, pix);
        assert!(r.is_some());

        // fill_image 写入的字节顺序为 [b, g, r, a]
        let expect = [pix.b(), pix.g(), pix.r(), pix.a()];
        for y in 3..5 {
            for x in 2..6 {
                assert_eq!(get_bytes(&mut dst, x, y), expect, "填充区域 ({x},{y})");
            }
        }
        // 区域外保持为 0
        assert_eq!(get_bytes(&mut dst, 0, 0), [0, 0, 0, 0]);
        assert_eq!(get_bytes(&mut dst, 7, 7), [0, 0, 0, 0]);
    }

    /// fill_image 边界检查
    #[test]
    fn test_fill_image_bounds() {
        let mut dst = make_bitmap(8, 8, |_, _| [0, 0, 0, 0]);
        let pix = Color::WHITE;
        assert!(fill_image(&mut dst, 6, 6, 4, 4, pix).is_none());
        assert!(fill_image(&mut dst, -1, 0, 2, 2, pix).is_none());
        assert!(fill_image(&mut dst, 0, 0, 0, 0, pix).is_some());
    }

    /// fill_image_mix：不透明填充覆盖，半透明填充按比例混合
    #[test]
    fn test_fill_image_mix() {
        let mut dst = make_bitmap(4, 4, |_, _| [0, 0, 0, 255]);
        let pix = Color::from_argb(255, 255, 255, 255);

        // 不透明白色填充黑底 → 白
        assert!(fill_image_mix(&mut dst, 0, 0, 4, 4, pix).is_some());
        assert_eq!(get_bytes(&mut dst, 1, 1), [255, 255, 255, 255]);

        // 50% 黑填充白底 → 约半值
        let half_black = Color::from_argb(128, 0, 0, 0);
        assert!(fill_image_mix(&mut dst, 0, 0, 4, 4, half_black).is_some());
        for ch in get_bytes(&mut dst, 2, 2).iter().take(3) {
            assert!((*ch as i32 - 127).abs() <= 1, "半透明填充通道值 = {ch}");
        }
        assert_eq!(get_bytes(&mut dst, 2, 2)[3], 255);
    }

    /// fill_image_mix 边界检查
    #[test]
    fn test_fill_image_mix_bounds() {
        let mut dst = make_bitmap(4, 4, |_, _| [0, 0, 0, 0]);
        let pix = Color::WHITE;
        assert!(fill_image_mix(&mut dst, 3, 3, 2, 2, pix).is_none());
        assert!(fill_image_mix(&mut dst, 0, 0, 0, 0, pix).is_some());
    }

    /// draw_with_fill_image：每个源像素填充为 width x height 的块
    #[test]
    fn test_draw_with_fill_image() {
        let mut src = make_bitmap(2, 2, pattern);
        let mut dst = make_bitmap(8, 8, |_, _| [0, 0, 0, 0]);

        // 源 2x2，每个像素填充为 4x4 块，铺满目标 8x8
        let r = draw_with_fill_image(&mut dst, &mut src, 0, 0, 0, 0, 2, 2, 4, 4);
        assert!(r.is_some());

        for j in 0..2 {
            for i in 0..2 {
                let expect = pattern(i, j);
                for fy in 0..4 {
                    for fx in 0..4 {
                        assert_eq!(
                            get_bytes(&mut dst, i * 4 + fx, j * 4 + fy),
                            expect,
                            "填充块 ({i},{j}) 内 ({fx},{fy})"
                        );
                    }
                }
            }
        }
    }

    /// draw_with_fill_image：块大小与边界检查
    #[test]
    fn test_draw_with_fill_image_bounds() {
        let mut src = make_bitmap(2, 2, pattern);
        let mut dst = make_bitmap(8, 8, |_, _| [0, 0, 0, 0]);

        // 2 个 4x4 块超出 8 宽度
        assert!(draw_with_fill_image(&mut dst, &mut src, 1, 0, 0, 0, 2, 2, 4, 4).is_none());
        // 源越界
        assert!(draw_with_fill_image(&mut dst, &mut src, 0, 0, 0, 0, 3, 2, 1, 1).is_none());
        // 尺寸为 0 是无操作
        assert!(draw_with_fill_image(&mut dst, &mut src, 0, 0, 0, 0, 2, 2, 0, 4).is_some());
    }

    /// draw_with_fill_image_mix：不透明源填充覆盖目标
    #[test]
    fn test_draw_with_fill_image_mix() {
        let mut src = make_bitmap(2, 2, |_, _| [10, 20, 30, 255]);
        let mut dst = make_bitmap(8, 8, |_, _| [200, 200, 200, 255]);

        let r = draw_with_fill_image_mix(&mut dst, &mut src, 0, 0, 0, 0, 2, 2, 4, 4);
        assert!(r.is_some());
        // 不透明源完全覆盖
        for j in 0..8 {
            for i in 0..8 {
                assert_eq!(get_bytes(&mut dst, i, j), [10, 20, 30, 255], "({i},{j})");
            }
        }
    }

    /// draw_with_fill_image_mix 边界检查
    #[test]
    fn test_draw_with_fill_image_mix_bounds() {
        let mut src = make_bitmap(2, 2, pattern);
        let mut dst = make_bitmap(8, 8, |_, _| [0, 0, 0, 0]);
        assert!(draw_with_fill_image_mix(&mut dst, &mut src, 0, 1, 0, 0, 2, 2, 4, 4).is_none());
        assert!(draw_with_fill_image_mix(&mut dst, &mut src, 0, 0, 0, 0, 0, 2, 4, 4).is_some());
    }
}
