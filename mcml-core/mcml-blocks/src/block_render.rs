use std::{collections::HashMap, sync::LazyLock};

use glam::{Mat4, Vec4};
use skia_safe::{
    Bitmap, BlendMode, Canvas, ISize, Image, Matrix, Paint, Point, Point3, SamplingOptions, Shader,
    Size, Surface, TileMode, Vertices, canvas, surfaces, svg::Canvas,
};

static VERTICES: [Point3; 8] = [
    // Front face
    Point3 {
        x: -1.0,
        y: -1.0,
        z: 1.0,
    },
    Point3 {
        x: 1.0,
        y: -1.0,
        z: 1.0,
    },
    Point3 {
        x: 1.0,
        y: 1.0,
        z: 1.0,
    },
    Point3 {
        x: -1.0,
        y: 1.0,
        z: 1.0,
    },
    // Back face
    Point3 {
        x: -1.0,
        y: -1.0,
        z: -1.0,
    },
    Point3 {
        x: 1.0,
        y: -1.0,
        z: -1.0,
    },
    Point3 {
        x: 1.0,
        y: 1.0,
        z: -1.0,
    },
    Point3 {
        x: -1.0,
        y: 1.0,
        z: -1.0,
    },
];

static INDICES: [u8; 24] = [
    0, 4, 7, 3, // Back face
    0, 4, 5, 1, // Bottom face
    0, 1, 2, 3, // Right face
    3, 7, 6, 2, // Top face
    4, 5, 6, 7, // Left face
    1, 5, 6, 2, // Front face
];

static UV: [Point; 24] = [
    // Back face
    Point { x: 0.0, y: 1.0 },
    Point { x: 1.0, y: 1.0 },
    Point { x: 1.0, y: 0.0 },
    Point { x: 0.0, y: 0.0 },
    // Bottom face
    Point { x: 1.0, y: 0.0 },
    Point { x: 0.0, y: 0.0 },
    Point { x: 0.0, y: 1.0 },
    Point { x: 1.0, y: 1.0 },
    // Right face
    Point { x: 1.0, y: 1.0 },
    Point { x: 0.0, y: 1.0 },
    Point { x: 0.0, y: 0.0 },
    Point { x: 1.0, y: 0.0 },
    // Top face
    Point { x: 0.0, y: 0.0 },
    Point { x: 0.0, y: 1.0 },
    Point { x: 1.0, y: 1.0 },
    Point { x: 1.0, y: 0.0 },
    // Left face
    Point { x: 0.0, y: 1.0 },
    Point { x: 1.0, y: 1.0 },
    Point { x: 1.0, y: 0.0 },
    Point { x: 0.0, y: 0.0 },
    // Front face
    Point { x: 1.0, y: 1.0 },
    Point { x: 0.0, y: 1.0 },
    Point { x: 0.0, y: 0.0 },
    Point { x: 1.0, y: 0.0 },
];

static MATRIX: LazyLock<HashMap<usize, Vertices>> = LazyLock::new(|| {
    let base = Mat4::IDENTITY;
    let tran = Mat4::from_translation(translation);
    // let base = base * Matrix::translate(d)

    todo!()
});

fn project(mat: &Mat4, point: &Point3) -> Point {
    let vec = Vec4::new(point.x, point.y, point.z, 1.0);
    let mut res = mat * vec;

    if res.w != 0.0 {
        res.x = res.x / res.w;
        res.y = res.y / res.w;
    }

    Point { x: res.x, y: res.y }
}

fn draw_block_3d(tex: Bitmap, canvas: &Canvas) {
    let mut paint = Paint::default();
    paint.set_anti_alias(true);
    paint.set_shader(tex.to_shader(
        Some((TileMode::Clamp, TileMode::Clamp)),
        SamplingOptions::default(),
        None,
    ));

    for i in 0..INDICES.len() / 4 {
        let ver = MATRIX.get(&i).unwrap();

        canvas.draw_vertices(ver, BlendMode::SrcOver, &paint);
    }
}

fn make_block_3d(tex: Bitmap) -> Option<Surface> {
    let mut surface = surfaces::null(ISize {
        width: 256,
        height: 256,
    })?;

    let mut cav = surface.canvas();
    draw_block_3d(tex, cav);

    Some(surface)
}

fn make_block_gif(tex: Bitmap) -> Option<Image> {}

fn make_block_png(tex: Bitmap) -> Option<Image> {}

pub fn make_block_image(tex: Bitmap) -> Option<Image> {}
