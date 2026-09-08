use std::{f32::consts::PI, slice};

use glam::{Mat4, Vec3, Vec4};
use skia_safe::{
    AlphaType, Bitmap, BlendMode, Canvas, Color, ColorType, IRect, ImageInfo, Paint, Point, Point3,
    SamplingOptions, TileMode,
    image::CachingHint,
    surfaces,
    vertices::{BuilderFlags, VertexMode},
};

static CUBE_VERTICES: [Point3; 16] = [
    // 前面
    Point3::new(-1.0, -1.0, 1.0),
    Point3::new(1.0, -1.0, 1.0),
    Point3::new(1.0, 1.0, 1.0),
    Point3::new(-1.0, 1.0, 1.0),
    // 背面
    Point3::new(-1.0, -1.0, -1.0),
    Point3::new(1.0, -1.0, -1.0),
    Point3::new(1.0, 1.0, -1.0),
    Point3::new(-1.0, 1.0, -1.0),
    // 前面（顶层，1.125 倍缩放）
    Point3::new(-1.125, -1.125, 1.125),
    Point3::new(1.125, -1.125, 1.125),
    Point3::new(1.125, 1.125, 1.125),
    Point3::new(-1.125, 1.125, 1.125),
    // 背面（顶层）
    Point3::new(-1.125, -1.125, -1.125),
    Point3::new(1.125, -1.125, -1.125),
    Point3::new(1.125, 1.125, -1.125),
    Point3::new(-1.125, 1.125, -1.125),
];

static CUBE_INDICES: [usize; 48] = [
    8, 12, 15, 11, // 背面（顶层）
    8, 12, 13, 9, // 底面（顶层）
    8, 9, 10, 11, // 右面（顶层）
    0, 4, 7, 3, // 背面
    0, 4, 5, 1, // 底面
    0, 1, 2, 3, // 右面
    3, 7, 6, 2, // 顶面
    4, 5, 6, 7, // 左面
    1, 5, 6, 2, // 前面
    11, 15, 14, 10, // 顶面（顶层）
    12, 13, 14, 15, // 左面（顶层）
    9, 13, 14, 10, // 前面（顶层）
];

static FACE_POS: [IRect; 12] = [
    IRect::new(56, 8, 64, 16), // 背面（顶层）
    IRect::new(48, 0, 56, 8),  // 底面（顶层）
    IRect::new(48, 8, 56, 16), // 右面（顶层）
    IRect::new(24, 8, 32, 16), // 背面
    IRect::new(16, 0, 24, 8),  // 底面
    IRect::new(16, 8, 24, 16), // 右面
    IRect::new(8, 0, 16, 8),   // 顶面
    IRect::new(0, 8, 8, 16),   // 左面
    IRect::new(8, 8, 16, 16),  // 前面
    IRect::new(40, 0, 48, 8),  // 顶面（顶层）
    IRect::new(32, 8, 40, 16), // 左面（顶层）
    IRect::new(40, 8, 48, 16), // 前面（顶层）
];

static SOURCE_VERTICES: [Point; 48] = [
    Point::new(0.0, 1.0),
    Point::new(1.0, 1.0),
    Point::new(1.0, 0.0),
    Point::new(0.0, 0.0), // 背面
    Point::new(1.0, 0.0),
    Point::new(0.0, 0.0),
    Point::new(0.0, 1.0),
    Point::new(1.0, 1.0), // 底面
    Point::new(1.0, 1.0),
    Point::new(0.0, 1.0),
    Point::new(0.0, 0.0),
    Point::new(1.0, 0.0), // 右面
    Point::new(0.0, 1.0),
    Point::new(1.0, 1.0),
    Point::new(1.0, 0.0),
    Point::new(0.0, 0.0), // 背面
    Point::new(1.0, 0.0),
    Point::new(0.0, 0.0),
    Point::new(0.0, 1.0),
    Point::new(1.0, 1.0), // 底面
    Point::new(1.0, 1.0),
    Point::new(0.0, 1.0),
    Point::new(0.0, 0.0),
    Point::new(1.0, 0.0), // 右面
    Point::new(1.0, 0.0),
    Point::new(0.0, 0.0),
    Point::new(0.0, 1.0),
    Point::new(1.0, 1.0), // 顶面
    Point::new(0.0, 1.0),
    Point::new(1.0, 1.0),
    Point::new(1.0, 0.0),
    Point::new(0.0, 0.0), // 左面
    Point::new(1.0, 1.0),
    Point::new(0.0, 1.0),
    Point::new(0.0, 0.0),
    Point::new(1.0, 0.0), // 前面
    Point::new(1.0, 0.0),
    Point::new(0.0, 0.0),
    Point::new(0.0, 1.0),
    Point::new(1.0, 1.0), // 顶面
    Point::new(0.0, 1.0),
    Point::new(1.0, 1.0),
    Point::new(1.0, 0.0),
    Point::new(0.0, 0.0), // 左面
    Point::new(1.0, 1.0),
    Point::new(0.0, 1.0),
    Point::new(0.0, 0.0),
    Point::new(1.0, 0.0), // 前面
];

fn create_tran() -> Mat4 {
    let roty = Mat4::from_rotation_y(45.0 * PI / 180.0);
    let rotx = Mat4::from_rotation_x(-30.0 * PI / 180.0);

    let scale = Mat4::from_scale(Vec3::new(100.0, -100.0, 100.0));

    let tran = Mat4::from_translation(Vec3::new(200.0, 200.0, 0.0));

    tran * scale * rotx * roty
}

fn create_tran_rotate(x: f32, y: f32) -> Mat4 {
    let roty = Mat4::from_rotation_y(y * PI / 180.0);
    let rotx = Mat4::from_rotation_x(-x * PI / 180.0);

    let scale = Mat4::from_scale(Vec3::new(100.0, -100.0, 100.0));

    let tran = Mat4::from_translation(Vec3::new(200.0, 200.0, 0.0));

    tran * scale * rotx * roty
}

fn project(tran: &Mat4, point: Point3, enable_z: bool) -> Point {
    let mut res = tran * Vec4::new(point.x, point.y, point.z, 1.0);

    if res.w != 0.0 {
        res.x /= res.w;
        res.y /= res.w;
        res.z /= res.w;
    }

    if enable_z {
        let z = res.z * 0.0001 + 1.0;
        res.x /= z;
        res.y /= z;
    }

    Point::new(res.x, res.y)
}

fn draw_texture_faces(canvas: &Canvas, texture: &Bitmap, tran: &Mat4, enable_z: bool) {
    let face_count = CUBE_INDICES.len() / 4;

    let mut builder = skia_safe::vertices::Builder::new(
        VertexMode::Triangles,
        face_count * 6,
        0,
        BuilderFlags::HAS_TEX_COORDS,
    );

    // 每个面的四边形按 (0,1,2) (0,2,3) 展开为两个三角形
    const TRI_ORDER: [usize; 6] = [0, 1, 2, 0, 2, 3];

    {
        let positions = builder.positions();
        for (face_index, chunk) in positions.chunks_mut(6).enumerate() {
            let base_index = face_index * 4;
            for (i, pos) in chunk.iter_mut().enumerate() {
                *pos = project(
                    tran,
                    CUBE_VERTICES[CUBE_INDICES[base_index + TRI_ORDER[i]]],
                    enable_z,
                );
            }
        }
    }

    {
        let tex_coords = builder.tex_coords().unwrap();
        for (face_index, chunk) in tex_coords.chunks_mut(6).enumerate() {
            let face = FACE_POS[face_index];
            let base_index = face_index * 4;
            for (i, tex) in chunk.iter_mut().enumerate() {
                let src = SOURCE_VERTICES[base_index + TRI_ORDER[i]];
                *tex = Point::new(
                    face.left as f32 + src.x * 8.0,
                    face.top as f32 + src.y * 8.0,
                );
            }
        }
    }

    let vertices = builder.detach();

    let shader = texture.to_shader(
        Some((TileMode::Clamp, TileMode::Clamp)),
        SamplingOptions::default(),
        None,
    );

    let mut paint = Paint::default();
    let paint = paint.set_anti_alias(true);
    let paint = paint.set_shader(shader);

    canvas.draw_vertices(&vertices, BlendMode::SrcOver, &paint);
}

pub fn draw_head_3d_typea(image: &mut Bitmap) -> Option<Bitmap> {
    let width = 400;
    let height = 400;

    let info = ImageInfo::new(
        (width, height),
        ColorType::RGBA8888,
        AlphaType::Premul,
        None,
    );

    let mut draw = surfaces::raster(&info, None, None)?;
    let canvas = draw.canvas();
    canvas.clear(Color::new(0x00000000));

    let tran = create_tran();

    draw_texture_faces(canvas, image, &tran, false);

    let image = draw.image_snapshot();

    let mut bitmap = Bitmap::new();
    if !bitmap.set_info(&info, None) {
        return None;
    }
    bitmap.alloc_pixels();
    let size = bitmap.compute_byte_size();
    let pixels = unsafe { slice::from_raw_parts_mut(bitmap.pixels() as *mut u8, size) };
    if !image.read_pixels(
        &info,
        pixels,
        bitmap.row_bytes(),
        (0, 0),
        CachingHint::Disallow,
    ) {
        return None;
    }
    Some(bitmap)
}

pub fn draw_head_3d_typeb(image: &mut Bitmap, x: f32, y: f32) -> Option<Bitmap> {
    let width = 400;
    let height = 400;

    let info = ImageInfo::new(
        (width, height),
        ColorType::RGBA8888,
        AlphaType::Premul,
        None,
    );

    let mut draw = surfaces::raster(&info, None, None)?;
    let canvas = draw.canvas();
    canvas.clear(Color::new(0x00000000));

    let tran = create_tran_rotate(x, y);

    draw_texture_faces(canvas, image, &tran, true);

    let image = draw.image_snapshot();

    let mut bitmap = Bitmap::new();
    if !bitmap.set_info(&info, None) {
        return None;
    }
    bitmap.alloc_pixels();
    let size = bitmap.compute_byte_size();
    let pixels = unsafe { slice::from_raw_parts_mut(bitmap.pixels() as *mut u8, size) };
    if !image.read_pixels(
        &info,
        pixels,
        bitmap.row_bytes(),
        (0, 0),
        CachingHint::Disallow,
    ) {
        return None;
    }
    Some(bitmap)
}
