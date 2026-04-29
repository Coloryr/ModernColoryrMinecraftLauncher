pub mod cape_2d_draw;
pub mod head_2d_draw;

pub mod skin_draw {
    use skia_safe::{Bitmap, Color, IRect, ImageInfo};

    pub const SCALE_TYPEA: usize = 16;
    pub const SCALE_TYPEB: usize = 2;

    pub fn draw(dest: &mut Bitmap, source: &mut Bitmap, subset: IRect) -> Option<()> {
        // 边界检查
        if subset.left < 0
            || subset.top < 0
            || subset.right > source.width()
            || subset.bottom > source.height()
            || subset.left >= subset.right
            || subset.top >= subset.bottom
        {
            return None;
        }

        let width = subset.width();
        let height = subset.height();

        // 检查 dest 大小是否足够
        if dest.width() < width || dest.height() < height {
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
            let src_offset = ((subset.top + y) as usize) * src_row_bytes
                + (subset.left as usize) * bytes_per_pixel;
            let dst_offset = (y as usize) * dst_row_bytes;

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
            // 标准的 Alpha 混合 (SrcOver)
            let src_a = other.a() as f32 / 255.0;
            let dst_a = self.a() as f32 / 255.0;

            let out_a = src_a + dst_a * (1.0 - src_a);

            if out_a <= 0.0 {
                return Color::TRANSPARENT;
            }

            let out_r =
                (other.r() as f32 * src_a + self.r() as f32 * dst_a * (1.0 - src_a)) / out_a;
            let out_g =
                (other.g() as f32 * src_a + self.g() as f32 * dst_a * (1.0 - src_a)) / out_a;
            let out_b =
                (other.b() as f32 * src_a + self.b() as f32 * dst_a * (1.0 - src_a)) / out_a;

            Color::from_argb(
                (out_a.clamp(0.0, 255.0) * 255.0) as u8,
                (out_r.clamp(0.0, 255.0) * 255.0) as u8,
                (out_g.clamp(0.0, 255.0) * 255.0) as u8,
                (out_b.clamp(0.0, 255.0) * 255.0) as u8,
            )
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

                    // 读取源和目标颜色
                    let src_color =
                        Color::from_argb(src_slice[3], src_slice[2], src_slice[1], src_slice[0]);
                    let dst_color =
                        Color::from_argb(dst_slice[3], dst_slice[2], dst_slice[1], dst_slice[0]);

                    let mixed = dst_color.mix(&src_color);

                    // 写入混合结果 (BGRA order)
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
