use std::{f32::consts::PI, slice};

use glam::{Mat4, Vec3, Vec4};
use skia_safe::{
    AlphaType, Bitmap, BlendMode, Canvas, Color, ColorType, IRect, ImageInfo, Paint, Point, Point3,
    Rect, SamplingOptions, TileMode,
    canvas::SrcRectConstraint,
    image::CachingHint,
    surfaces,
    vertices::{BuilderFlags, VertexMode},
};

static CUBE_VERTICES: [Point3; 16] = [
    // Front face
    Point3::new(-1.0, -1.0, 1.0),
    Point3::new(1.0, -1.0, 1.0),
    Point3::new(1.0, 1.0, 1.0),
    Point3::new(-1.0, 1.0, 1.0),
    // Back face
    Point3::new(-1.0, -1.0, -1.0),
    Point3::new(1.0, -1.0, -1.0),
    Point3::new(1.0, 1.0, -1.0),
    Point3::new(-1.0, 1.0, -1.0),
    // Front face (Top layer, 1.125x scale)
    Point3::new(-1.125, -1.125, 1.125),
    Point3::new(1.125, -1.125, 1.125),
    Point3::new(1.125, 1.125, 1.125),
    Point3::new(-1.125, 1.125, 1.125),
    // Back face (Top layer)
    Point3::new(-1.125, -1.125, -1.125),
    Point3::new(1.125, -1.125, -1.125),
    Point3::new(1.125, 1.125, -1.125),
    Point3::new(-1.125, 1.125, -1.125),
];

static CUBE_INDICES: [usize; 48] = [
    8, 12, 15, 11, // Back face (Top)
    8, 12, 13, 9, // Bottom face (Top)
    8, 9, 10, 11, // Right face (Top)
    0, 4, 7, 3, // Back face
    0, 4, 5, 1, // Bottom face
    0, 1, 2, 3, // Right face
    3, 7, 6, 2, // Top face
    4, 5, 6, 7, // Left face
    1, 5, 6, 2, // Front face
    11, 15, 14, 10, // Top face (Top)
    12, 13, 14, 15, // Left face (Top)
    9, 13, 14, 10, // Front face (Top)
];

static FACE_POS: [IRect; 12] = [
    IRect::new(56, 8, 64, 16), // Back face (Top)
    IRect::new(48, 0, 56, 8),  // Bottom face (Top)
    IRect::new(48, 8, 56, 16), // Right face (Top)
    IRect::new(24, 8, 32, 16), // Back face
    IRect::new(16, 0, 24, 8),  // Bottom face
    IRect::new(16, 8, 24, 16), // Right face
    IRect::new(8, 0, 16, 8),   // Top face
    IRect::new(0, 8, 8, 16),   // Left face
    IRect::new(8, 8, 16, 16),  // Front face
    IRect::new(40, 0, 48, 8),  // Top face (Top)
    IRect::new(32, 8, 40, 16), // Left face (Top)
    IRect::new(40, 8, 48, 16), // Front face (Top)
];

static SOURCE_VERTICES: [Point; 48] = [
    Point::new(0.0, 1.0),
    Point::new(1.0, 1.0),
    Point::new(1.0, 0.0),
    Point::new(0.0, 0.0), // Back
    Point::new(1.0, 0.0),
    Point::new(0.0, 0.0),
    Point::new(0.0, 1.0),
    Point::new(1.0, 1.0), // Bottom
    Point::new(1.0, 1.0),
    Point::new(0.0, 1.0),
    Point::new(0.0, 0.0),
    Point::new(1.0, 0.0), // Right
    Point::new(0.0, 1.0),
    Point::new(1.0, 1.0),
    Point::new(1.0, 0.0),
    Point::new(0.0, 0.0), // Back
    Point::new(1.0, 0.0),
    Point::new(0.0, 0.0),
    Point::new(0.0, 1.0),
    Point::new(1.0, 1.0), // Bottom
    Point::new(1.0, 1.0),
    Point::new(0.0, 1.0),
    Point::new(0.0, 0.0),
    Point::new(1.0, 0.0), // Right
    Point::new(1.0, 0.0),
    Point::new(0.0, 0.0),
    Point::new(0.0, 1.0),
    Point::new(1.0, 1.0), // Top
    Point::new(0.0, 1.0),
    Point::new(1.0, 1.0),
    Point::new(1.0, 0.0),
    Point::new(0.0, 0.0), // Left
    Point::new(1.0, 1.0),
    Point::new(0.0, 1.0),
    Point::new(0.0, 0.0),
    Point::new(1.0, 0.0), // Front
    Point::new(1.0, 0.0),
    Point::new(0.0, 0.0),
    Point::new(0.0, 1.0),
    Point::new(1.0, 1.0), // Top
    Point::new(0.0, 1.0),
    Point::new(1.0, 1.0),
    Point::new(1.0, 0.0),
    Point::new(0.0, 0.0), // Left
    Point::new(1.0, 1.0),
    Point::new(0.0, 1.0),
    Point::new(0.0, 0.0),
    Point::new(1.0, 0.0), // Front
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

fn draw_texture_face(
    canvas: &Canvas,
    texture: &mut Bitmap,
    tran: &Mat4,
    index: usize,
    enable_z: bool,
) {
    let face = FACE_POS[index];

    let info = ImageInfo::new((8, 8), ColorType::RGBA8888, AlphaType::Premul, None);
    let mut source_image = Bitmap::new();

    source_image.alloc_pixels_info(&info, None);

    let img_canvas = Canvas::from_bitmap(&source_image, None).unwrap();
    img_canvas.draw_image_rect(
        texture.as_image(),
        Some((
            &Rect::new(
                face.left as f32,
                face.top as f32,
                face.right as f32,
                face.bottom as f32,
            ),
            SrcRectConstraint::Strict,
        )),
        Rect::new(0.0, 0.0, 8.0, 8.0),
        &Paint::default(),
    );

    let base_index = index * 4;

    let mut builder = skia_safe::vertices::Builder::new(
        VertexMode::TriangleFan,
        4,
        0,
        BuilderFlags::HAS_TEX_COORDS,
    );
    let pos = builder.positions();
    pos[0] = project(&tran, CUBE_VERTICES[CUBE_INDICES[base_index]], enable_z);
    pos[1] = project(&tran, CUBE_VERTICES[CUBE_INDICES[base_index + 1]], enable_z);
    pos[2] = project(&tran, CUBE_VERTICES[CUBE_INDICES[base_index + 2]], enable_z);
    pos[3] = project(&tran, CUBE_VERTICES[CUBE_INDICES[base_index + 3]], enable_z);

    let tex = builder.tex_coords().unwrap();
    tex[0] = Point::new(
        SOURCE_VERTICES[base_index].x * 8.0,
        SOURCE_VERTICES[base_index].y * 8.0,
    );
    tex[1] = Point::new(
        SOURCE_VERTICES[base_index + 1].x * 8.0,
        SOURCE_VERTICES[base_index + 1].y * 8.0,
    );
    tex[2] = Point::new(
        SOURCE_VERTICES[base_index + 2].x * 8.0,
        SOURCE_VERTICES[base_index + 2].y * 8.0,
    );
    tex[3] = Point::new(
        SOURCE_VERTICES[base_index + 3].x * 8.0,
        SOURCE_VERTICES[base_index + 3].y * 8.0,
    );

    let vertices = builder.detach();

    let shader = source_image.to_shader(
        Some((TileMode::Clamp, TileMode::Clamp)),
        SamplingOptions::default(),
        None,
    );

    let mut paint = Paint::default();
    let paint = paint.set_anti_alias(true);
    let paint = paint.set_shader(shader);

    canvas.draw_vertices(&vertices, BlendMode::SrcOver, &paint);

    // mcml_skin::skin::save_image(
    //     &canvas.surface().unwrap().image_snapshot(),
    //     std::path::Path::new("tests").join("temp.png").as_path(),
    // );
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

    let face_count = CUBE_INDICES.len() / 4;
    for index in 0..face_count {
        draw_texture_face(canvas, image, &tran, index, false);
    }

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

    let face_count = CUBE_INDICES.len() / 4;
    for index in 0..face_count {
        draw_texture_face(canvas, image, &tran, index, true);
    }

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
