use tiny_skia::Pixmap;

use crate::skin_draw::{SCALE_TYPEA, draw, scale};

pub fn draw_cape_2d(image: &Pixmap) -> Option<Pixmap> {
    let mut dest = Pixmap::new(10, 16)?;

    draw(&mut dest, image, 0, 0, 1, 1, 10, 16)?;
    scale(&dest, SCALE_TYPEA)
}

pub fn draw_cape_back_2d(image: &Pixmap) -> Option<Pixmap> {
    let mut dest = Pixmap::new(10, 16)?;

    draw(&mut dest, image, 0, 0, 12, 1, 10, 16)?;
    scale(&dest, SCALE_TYPEA)
}
