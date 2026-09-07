use mcml_skin::{SkinType, skin_type_checker};
use skia_safe::{Bitmap, Color, ImageInfo};

use crate::skin_draw::{
    ColorMix, SCALE_TYPEB, SCALE_TYPEC, draw_mix, draw_with_fill_image, draw_with_fill_image_mix,
    fill_image, scale,
};

/// 获取指定像素的颜色（使用原始指针，避免借用冲突）
fn get_pixel_raw(ptr: *const u8, row_bytes: usize, bpp: usize, x: i32, y: i32) -> Color {
    let offset = (y as usize) * row_bytes + (x as usize) * bpp;
    let slice = unsafe { std::slice::from_raw_parts(ptr.add(offset), bpp) };
    Color::from_argb(slice[3], slice[2], slice[1], slice[0])
}

/// 设置指定像素的颜色（使用原始指针，避免借用冲突）
fn set_pixel_raw(ptr: *mut u8, row_bytes: usize, bpp: usize, x: i32, y: i32, color: Color) {
    let offset = (y as usize) * row_bytes + (x as usize) * bpp;
    let slice = unsafe { std::slice::from_raw_parts_mut(ptr.add(offset), bpp) };
    slice[0] = color.b();
    slice[1] = color.g();
    slice[2] = color.r();
    slice[3] = color.a();
}

/// 创建皮肤图片（TypeA 风格）
/// `image` - 原始皮肤贴图
/// `skin_type` - 皮肤类型，None 为自动检测
/// 返回缩放后的 2D 皮肤图片 (128x256)
pub fn skin_2d_draw_typea(image: &mut Bitmap, skin_type: Option<SkinType>) -> Option<Bitmap> {
    // 创建中间 16x32 画布
    let mut image1 = Bitmap::new();
    let info1 = ImageInfo::new(
        (16, 32),
        image.color_type(),
        image.alpha_type(),
        image.color_space().map(|cs| cs.clone()),
    );
    if !image1.set_info(&info1, None) {
        return None;
    }
    image1.alloc_pixels();

    let skintype = skin_type.unwrap_or_else(|| skin_type_checker::get_skin_type(image));

    // head (8,8,8,8) -> (4,0)
    draw_mix(&mut image1, image, 4, 0, 8, 8, 8, 8)?;
    // head top (40,8,8,8) -> (4,0) 混合
    draw_mix(&mut image1, image, 4, 0, 40, 8, 8, 8)?;
    // body (20,20,8,12) -> (4,8)
    draw_mix(&mut image1, image, 4, 8, 20, 20, 8, 12)?;

    if skintype == SkinType::New || skintype == SkinType::NewSlim {
        // body over (20,36,8,12) -> (4,8) 混合
        draw_mix(&mut image1, image, 4, 8, 20, 36, 8, 12)?;
    }

    // right hand
    if skintype == SkinType::NewSlim {
        // (44,20,3,12) -> (1,8)
        draw_mix(&mut image1, image, 1, 8, 44, 20, 3, 12)?;
        // top (44,36,3,12) -> (1,8) 混合
        draw_mix(&mut image1, image, 1, 8, 44, 36, 3, 12)?;
    } else {
        // (44,20,4,12) -> (0,8)
        draw_mix(&mut image1, image, 0, 8, 44, 20, 4, 12)?;
        if skintype != SkinType::Old {
            // top (44,36,4,12) -> (0,8) 混合
            draw_mix(&mut image1, image, 0, 8, 44, 36, 4, 12)?;
        }
    }

    // left hand
    if skintype == SkinType::NewSlim {
        // (36,52,3,12) -> (12,8)
        draw_mix(&mut image1, image, 12, 8, 36, 52, 3, 12)?;
        // top (52,52,3,12) -> (12,8) 混合
        draw_mix(&mut image1, image, 12, 8, 52, 52, 3, 12)?;
    } else {
        if skintype == SkinType::Old {
            // 旧版镜像：从 image1 的右手区域镜像到左手位置，并混合 overlay 纹理
            let src_ptr = image1.pixels() as *const u8;
            let src_row = image1.row_bytes() as usize;
            let src_bpp = image1.bytes_per_pixel() as usize;
            let img_ptr = image.pixels() as *const u8;
            let img_row = image.row_bytes() as usize;
            let img_bpp = image.bytes_per_pixel() as usize;
            let dst_ptr = image1.pixels() as *mut u8;

            for i in (0..4).rev() {
                for j in 0..12 {
                    let src_color = get_pixel_raw(src_ptr, src_row, src_bpp, i, j + 8);
                    let mix_color = get_pixel_raw(img_ptr, img_row, img_bpp, i + 44, j + 20);
                    let mixed = src_color.mix(&mix_color);
                    set_pixel_raw(dst_ptr, src_row, src_bpp, i + 12, j + 8, mixed);
                }
            }
        } else {
            // (36,52,4,12) -> (12,8)
            draw_mix(&mut image1, image, 12, 8, 36, 52, 4, 12)?;
            // top (52,52,4,12) -> (12,8) 混合
            draw_mix(&mut image1, image, 12, 8, 52, 52, 4, 12)?;
        }
    }

    // right leg (4,20,4,12) -> (4,20)
    draw_mix(&mut image1, image, 4, 20, 4, 20, 4, 12)?;
    if skintype == SkinType::New || skintype == SkinType::NewSlim {
        // top (4,36,4,12) -> (4,20) 混合
        draw_mix(&mut image1, image, 4, 20, 4, 36, 4, 12)?;
    }

    // left leg
    if skintype == SkinType::Old {
        // 旧版镜像：从 image1 的右腿区域镜像到左腿位置，并混合 overlay 纹理
        let src_ptr = image1.pixels() as *const u8;
        let src_row = image1.row_bytes() as usize;
        let src_bpp = image1.bytes_per_pixel() as usize;
        let img_ptr = image.pixels() as *const u8;
        let img_row = image.row_bytes() as usize;
        let img_bpp = image.bytes_per_pixel() as usize;
        let dst_ptr = image1.pixels() as *mut u8;

        for i in (0..4).rev() {
            for j in 0..12 {
                let src_color = get_pixel_raw(src_ptr, src_row, src_bpp, i, j + 20);
                let mix_color = get_pixel_raw(img_ptr, img_row, img_bpp, i + 4, j + 20);
                let mixed = src_color.mix(&mix_color);
                set_pixel_raw(dst_ptr, src_row, src_bpp, i + 8, j + 20, mixed);
            }
        }
    } else {
        // (20,52,4,12) -> (8,20)
        draw_mix(&mut image1, image, 8, 20, 20, 52, 4, 12)?;
        // top (4,52,4,12) -> (8,20) 混合
        draw_mix(&mut image1, image, 8, 20, 4, 52, 4, 12)?;
    }

    // 缩放 8x
    scale(&mut image1, SCALE_TYPEC)
}

/// 创建皮肤图片（TypeB 风格）
/// `image` - 原始皮肤贴图
/// `skin_type` - 皮肤类型，None 为自动检测
/// 返回缩放后的 2D 皮肤图片 (272x532)
pub fn skin_2d_draw_typeb(image: &mut Bitmap, skin_type: Option<SkinType>) -> Option<Bitmap> {
    // 创建中间 136x266 画布
    let mut image1 = Bitmap::new();
    let info1 = ImageInfo::new(
        (136, 266),
        image.color_type(),
        image.alpha_type(),
        image.color_space().map(|cs| cs.clone()),
    );
    if !image1.set_info(&info1, None) {
        return None;
    }
    image1.alloc_pixels();

    let skintype = skin_type.unwrap_or_else(|| skin_type_checker::get_skin_type(image));

    // head: 从 (4+8*4, 4) 提取 (8,8)，填充到 (8,8) 块，目标位置 (4+8*4, 4)
    draw_with_fill_image(&mut image1, image, 4 + 8 * 4, 4, 8, 8, 8, 8, 8, 8)?;

    // body: 从 (4+8*8, 4+8*8) 提取 (20,20)，填充到 (8,12) 块，目标位置 (4+8*4, 4+8*8)
    draw_with_fill_image(
        &mut image1,
        image,
        4 + 8 * 4,
        4 + 8 * 8,
        20,
        20,
        8,
        12,
        8,
        8,
    )?;

    // right hand
    if skintype == SkinType::NewSlim {
        draw_with_fill_image(
            &mut image1,
            image,
            4 + 1 * 8,
            4 + 8 * 8,
            44,
            20,
            3,
            12,
            8,
            8,
        )?;
    } else {
        draw_with_fill_image(&mut image1, image, 4, 4 + 8 * 8, 44, 20, 4, 12, 8, 8)?;
    }

    // left hand
    if skintype == SkinType::NewSlim {
        draw_with_fill_image(
            &mut image1,
            image,
            4 + 12 * 8,
            4 + 8 * 8,
            36,
            52,
            3,
            12,
            8,
            8,
        )?;
    } else {
        if skintype == SkinType::Old {
            // 旧版镜像：从源图右手区域镜像到左手位置
            let img_ptr = image.pixels() as *const u8;
            let img_row = image.row_bytes() as usize;
            let img_bpp = image.bytes_per_pixel() as usize;

            for i in (0..4).rev() {
                for j in 0..12 {
                    let pix = get_pixel_raw(img_ptr, img_row, img_bpp, i + 44, j + 20);
                    fill_image(
                        &mut image1,
                        4 + 12 * 8 + i * 8,
                        4 + 8 * 8 + j * 8,
                        8,
                        8,
                        pix,
                    )?;
                }
            }
        } else {
            draw_with_fill_image(
                &mut image1,
                image,
                4 + 12 * 8,
                4 + 8 * 8,
                36,
                52,
                4,
                12,
                8,
                8,
            )?;
        }
    }

    // right leg
    draw_with_fill_image(
        &mut image1,
        image,
        4 + 4 * 8,
        4 + 20 * 8,
        4,
        20,
        4,
        12,
        8,
        8,
    )?;

    // left leg
    if skintype == SkinType::Old {
        // 旧版镜像：从源图右腿区域镜像到左腿位置
        let img_ptr = image.pixels() as *const u8;
        let img_row = image.row_bytes() as usize;
        let img_bpp = image.bytes_per_pixel() as usize;

        for i in (0..4).rev() {
            for j in 0..12 {
                let pix = get_pixel_raw(img_ptr, img_row, img_bpp, i + 4, j + 20);
                fill_image(
                    &mut image1,
                    4 + 8 * 8 + i * 8,
                    4 + 20 * 8 + j * 8,
                    8,
                    8,
                    pix,
                )?;
            }
        }
    } else {
        draw_with_fill_image(
            &mut image1,
            image,
            4 + 8 * 8,
            4 + 20 * 8,
            20,
            52,
            4,
            12,
            8,
            8,
        )?;
    }

    // body over (仅 New / NewSlim)
    if skintype == SkinType::New || skintype == SkinType::NewSlim {
        draw_with_fill_image_mix(&mut image1, image, 4 * 8, 8 * 8 - 2, 20, 36, 8, 12, 9, 9)?;
    }

    // head top
    draw_with_fill_image_mix(&mut image1, image, 4 * 9 - 4, 0, 40, 8, 8, 8, 9, 9)?;

    if skintype == SkinType::NewSlim {
        // top: 右手 overlay
        draw_with_fill_image_mix(
            &mut image1,
            image,
            1 * 8 + 1,
            8 * 8 + 2,
            44,
            36,
            3,
            12,
            9,
            9,
        )?;
        // top: 左手 overlay
        draw_with_fill_image_mix(
            &mut image1,
            image,
            12 * 8 + 4,
            8 * 8 + 2,
            52,
            52,
            3,
            12,
            9,
            9,
        )?;
    } else if skintype == SkinType::New {
        // top: 右手 overlay
        draw_with_fill_image_mix(&mut image1, image, 0, 8 * 8 + 2, 44, 36, 4, 12, 9, 9)?;
        // top: 左手 overlay
        draw_with_fill_image_mix(
            &mut image1,
            image,
            12 * 8 + 4,
            8 * 8 + 2,
            52,
            52,
            4,
            12,
            9,
            9,
        )?;
    }

    if skintype == SkinType::New || skintype == SkinType::NewSlim {
        // top: 右腿 overlay
        draw_with_fill_image_mix(
            &mut image1,
            image,
            4 * 8 + 2,
            20 * 8 - 2,
            4,
            36,
            4,
            12,
            9,
            9,
        )?;
        // top: 左腿 overlay
        draw_with_fill_image_mix(
            &mut image1,
            image,
            8 * 8 + 2,
            20 * 8 - 2,
            4,
            52,
            4,
            12,
            9,
            9,
        )?;
    }

    scale(&mut image1, SCALE_TYPEB)
}

/// 注意：draw_mix 按 BGRA 字节序解释像素内存，且整个管线（mix -> scale -> PNG 存取）
/// 都保持同一套字节语义，所以本测试直接在字节层面做断言，不比较语义上的 R/G/B。
#[cfg(test)]
mod tests {
    use skia_safe::{AlphaType, ColorType, ImageInfo};

    use super::*;

    /// 创建 RGBA8888 位图并对每个像素调用填充函数写入 4 字节
    fn make_bitmap(w: i32, h: i32, fill: impl Fn(i32, i32) -> [u8; 4]) -> Bitmap {
        let info = ImageInfo::new((w, h), ColorType::RGBA8888, AlphaType::Premul, None);
        let mut bm = Bitmap::new();
        assert!(bm.set_info(&info, None), "set_info 失败");
        bm.alloc_pixels();
        let row = bm.row_bytes() as usize;
        let bpp = bm.bytes_per_pixel() as usize;
        let ptr = bm.pixels() as *mut u8;
        assert!(!ptr.is_null());
        unsafe {
            for y in 0..h {
                for x in 0..w {
                    let off = y as usize * row + x as usize * bpp;
                    std::slice::from_raw_parts_mut(ptr.add(off), bpp)
                        .copy_from_slice(&fill(x, y));
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

    /// 新版 (1.8+) 皮肤的 2D 展开：各部件应落在 16x32 中间画布的对应区域，
    /// 缩放 8 倍后为 128x256。透明 overlay 不应破坏底层的颜色。
    #[test]
    fn test_skin_2d_draw_typea_new_layout() {
        // 构造 64x64 新版皮肤：每个部件用唯一的字节模式标记，overlay 区域全透明
        let image = make_bitmap(64, 64, |x, y| {
            let in_area = |ax: i32, ay: i32, aw: i32, ah: i32| {
                x >= ax && x < ax + aw && y >= ay && y < ay + ah
            };
            // 底层部件
            if in_area(8, 8, 8, 8) {
                [10, 0, 0, 255] // 头部正面 (8,8,8,8)
            } else if in_area(20, 20, 8, 12) {
                [20, 0, 0, 255] // 身体 (20,20,8,12)
            } else if in_area(44, 20, 4, 12) {
                [30, 0, 0, 255] // 右手 (44,20,4,12)
            } else if in_area(36, 52, 4, 12) {
                [40, 0, 0, 255] // 左手 (36,52,4,12)
            } else if in_area(4, 20, 4, 12) {
                [50, 0, 0, 255] // 右腿 (4,20,4,12)
            } else if in_area(20, 52, 4, 12) {
                [60, 0, 0, 255] // 左腿 (20,52,4,12)
            } else {
                [0, 0, 0, 0] // 其余（含所有 overlay 区域）透明
            }
        });
        let mut image = image;

        let result = skin_2d_draw_typea(&mut image, Some(SkinType::New));
        let mut out = result.expect("typea 2D 展开应成功");
        assert_eq!(out.width(), 128, "typea 输出宽度应为 16 * 8");
        assert_eq!(out.height(), 256, "typea 输出高度应为 32 * 8");

        // 16x32 中间画布上的目标区域 -> 缩放 8 倍后的输出区域取中心点校验
        // 头部 (4,0,8,8) -> 输出中心 (64, 32)
        assert_eq!(get_bytes(&mut out, 64, 32), [10, 0, 0, 255], "头部");
        // 身体 (4,8,8,12) -> 输出中心 (64, 128)
        assert_eq!(get_bytes(&mut out, 64, 128), [20, 0, 0, 255], "身体");
        // 右手 (0,8,4,12) -> 输出中心 (16, 128)
        assert_eq!(get_bytes(&mut out, 16, 128), [30, 0, 0, 255], "右手");
        // 左手 (12,8,4,12) -> 输出中心 (112, 128)
        assert_eq!(get_bytes(&mut out, 112, 128), [40, 0, 0, 255], "左手");
        // 右腿 (4,20,4,12) -> 输出中心 (48, 208)
        assert_eq!(get_bytes(&mut out, 48, 208), [50, 0, 0, 255], "右腿");
        // 左腿 (8,20,4,12) -> 输出中心 (80, 208)
        assert_eq!(get_bytes(&mut out, 80, 208), [60, 0, 0, 255], "左腿");

        // 部件之间的空隙应保持透明（例如 16x32 画布的 (0,0) -> 输出 (0,0)）
        assert_eq!(get_bytes(&mut out, 0, 0), [0, 0, 0, 0], "空隙应透明");
    }

    /// 纤细 (slim) 皮肤的右手只有 3 像素宽，左手位置与普通皮肤一致
    #[test]
    fn test_skin_2d_draw_typea_slim_layout() {
        let image = make_bitmap(64, 64, |x, y| {
            let in_area = |ax: i32, ay: i32, aw: i32, ah: i32| {
                x >= ax && x < ax + aw && y >= ay && y < ay + ah
            };
            if in_area(8, 8, 8, 8) {
                [10, 0, 0, 255] // 头部正面
            } else if in_area(44, 20, 3, 12) {
                [30, 0, 0, 255] // 纤细右手 (44,20,3,12)
            } else if in_area(36, 52, 3, 12) {
                [40, 0, 0, 255] // 纤细左手 (36,52,3,12)
            } else {
                [0, 0, 0, 0]
            }
        });
        let mut image = image;

        let mut out = skin_2d_draw_typea(&mut image, Some(SkinType::NewSlim))
            .expect("纤细皮肤 typea 展开应成功");
        assert_eq!(out.width(), 128);
        assert_eq!(out.height(), 256);

        assert_eq!(get_bytes(&mut out, 64, 32), [10, 0, 0, 255], "头部");
        // 纤细右手放在 (1,8,3,12) -> 输出中心 (20, 128)
        assert_eq!(get_bytes(&mut out, 20, 128), [30, 0, 0, 255], "纤细右手");
        // 纤细左手放在 (12,8,3,12) -> 输出中心 (112, 128)
        assert_eq!(get_bytes(&mut out, 112, 128), [40, 0, 0, 255], "纤细左手");
    }
}
