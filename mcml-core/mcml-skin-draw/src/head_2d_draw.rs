use tiny_skia::Pixmap;

use crate::skin_draw::{
    SCALE_TYPEA, SCALE_TYPEB, draw, draw_mix, draw_with_fill_image, draw_with_fill_image_mix, scale,
};

pub fn head_2d_draw_typea(image: &Pixmap) -> Option<Pixmap> {
    let mut dest = Pixmap::new(8, 8)?;

    draw(&mut dest, image, 0, 0, 8, 8, 8, 8)?;
    draw_mix(&mut dest, image, 0, 0, 40, 8, 8, 8)?;
    scale(&dest, SCALE_TYPEA)
}

pub fn head_2d_draw_typeb(image: &Pixmap) -> Option<Pixmap> {
    let mut dest = Pixmap::new(72, 72)?;

    draw_with_fill_image(&mut dest, image, 4, 4, 8, 8, 8, 8, 8, 8)?;
    draw_with_fill_image_mix(&mut dest, image, 0, 0, 40, 8, 8, 8, 9, 9)?;

    scale(&dest, SCALE_TYPEB)
}
