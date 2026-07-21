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
