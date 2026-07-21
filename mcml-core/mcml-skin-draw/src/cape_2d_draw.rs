use skia_safe::{Bitmap, ImageInfo};

use crate::skin_draw::{SCALE_TYPEA, draw, scale};

pub fn draw_cape_2d(image: &mut Bitmap) -> Option<Bitmap> {
    let mut dest = Bitmap::new();
    let image_info = ImageInfo::new(
        (10, 16),
        image.color_type(),
        image.alpha_type(),
        image.color_space().map(|cs| cs.clone()),
    );
    if !dest.set_info(&image_info, None) {
        return None;
    }
    dest.alloc_pixels();

    draw(&mut dest, image, 0, 0, 1, 1, 10, 16)?;
    scale(&mut dest, SCALE_TYPEA)
}

pub fn draw_cape_back_2d(image: &mut Bitmap) -> Option<Bitmap> {
    let mut dest = Bitmap::new();
    let image_info = ImageInfo::new(
        (10, 16),
        image.color_type(),
        image.alpha_type(),
        image.color_space().map(|cs| cs.clone()),
    );
    if !dest.set_info(&image_info, None) {
        return None;
    }
    dest.alloc_pixels();

    draw(&mut dest, image, 0, 0, 12, 1, 10, 16)?;
    scale(&mut dest, SCALE_TYPEA)
}
