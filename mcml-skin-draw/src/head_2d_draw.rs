pub mod head_2d {
    use skia_safe::{Bitmap, ImageInfo};

    use crate::skin_draw::{
        SCALE_TYPEA, SCALE_TYPEB, draw, draw_mix, draw_with_fill_image, draw_with_fill_image_mix,
        scale,
    };

    pub fn head_2d_draw_typea(image: &mut Bitmap) -> Option<Bitmap> {
        let mut dest = Bitmap::new();
        let image_info = ImageInfo::new(
            (8, 8),
            image.color_type(),
            image.alpha_type(),
            image.color_space().map(|cs| cs.clone()),
        );
        if !dest.set_info(&image_info, None) {
            return None;
        }
        dest.alloc_pixels();

        draw(&mut dest, image, 0,0,8,8,8,8)?;
        draw_mix(&mut dest, image, 0, 0, 40, 8, 8, 8)?;
        scale(&mut dest, SCALE_TYPEA)
    }

    pub fn head_2d_draw_typeb(image: &mut Bitmap) -> Option<Bitmap> {
        let mut dest = Bitmap::new();
        let image_info = ImageInfo::new(
            (72, 72),
            image.color_type(),
            image.alpha_type(),
            image.color_space().map(|cs| cs.clone()),
        );
        if !dest.set_info(&image_info, None) {
            return None;
        }
        dest.alloc_pixels();

        draw_with_fill_image(&mut dest, image, 4, 4, 8, 8, 8, 8, 8, 8)?;
        draw_with_fill_image_mix(&mut dest, image, 0, 0, 40, 8, 8, 8, 9, 9)?;

        scale(&mut dest, SCALE_TYPEB)
    }
}
