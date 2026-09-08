use std::{
    collections::HashMap,
    f32::consts::PI,
    io::Cursor,
    slice,
    sync::LazyLock,
};

use glam::{Mat4, Vec3, Vec4};
use mcml_base::{archives::BaseArchive, serialize_tools};
use mcml_names::i18_items::error_type::{CoreResult, ErrorType};
use mcml_sys::path_helper;
use serde::{Deserialize, Serialize};
use skia_safe::{
    AlphaType, Bitmap, BlendMode, Canvas, Color, ColorType, Data, EncodedImageFormat, IRect, Image,
    ImageInfo, Paint, Point, Point3, SamplingOptions, TileMode, surfaces,
    vertices::{self, VertexMode},
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

static INDICES: [usize; 24] = [
    0, 4, 7, 3, // Back face
    0, 4, 5, 1, // Bottom face
    0, 1, 2, 3, // Right face
    3, 7, 6, 2, // Top face
    4, 5, 6, 7, // Left face
    1, 5, 6, 2, // Front face
];

/// 三个可见面（顶、北、东）的基准uv角，角序与INDICES的展开顺序一致
/// （顶面u沿世界z，已与wiki木板顶面纹理走向比对确认）
static BASE_UV: [[Point; 4]; 3] = [
    // 顶面
    [
        Point { x: 0.0, y: 1.0 },
        Point { x: 1.0, y: 1.0 },
        Point { x: 1.0, y: 0.0 },
        Point { x: 0.0, y: 0.0 },
    ],
    // 北面
    [
        Point { x: 0.0, y: 1.0 },
        Point { x: 1.0, y: 1.0 },
        Point { x: 1.0, y: 0.0 },
        Point { x: 0.0, y: 0.0 },
    ],
    // 东面
    [
        Point { x: 1.0, y: 1.0 },
        Point { x: 0.0, y: 1.0 },
        Point { x: 0.0, y: 0.0 },
        Point { x: 1.0, y: 0.0 },
    ],
];

/// 等轴测旋转矩阵（先翻转y，再绕y轴45°、x轴30°）
static MAT: LazyLock<Mat4> = LazyLock::new(|| {
    Mat4::IDENTITY
        * Mat4::from_rotation_x(30.0 * PI / 180.0)
        * Mat4::from_rotation_y(45.0 * PI / 180.0)
        * Mat4::from_scale(Vec3::new(1.0, -1.0, 1.0))
});

/// 投影一组模型空间点：先旋转，再按包围盒等比缩放居中（fit到画布，四周留边距）
/// 返回屏幕坐标与深度（旋转后z，用于面排序）
fn project_fit(points: &[Point3]) -> (Vec<Point>, Vec<f32>) {
    // 画布四周留边距
    const MARGIN: f32 = 2.0;
    let mat = *MAT;

    let mut depths: Vec<f32> = Vec::with_capacity(points.len());
    let mut ver: Vec<Point> = Vec::with_capacity(points.len());
    for p in points {
        let res = project(&mat, p);
        depths.push(res.z);
        ver.push(Point { x: res.x, y: res.y });
    }

    // 投影包围盒
    let mut min_x = f32::INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut max_y = f32::NEG_INFINITY;
    for p in &ver {
        min_x = min_x.min(p.x);
        min_y = min_y.min(p.y);
        max_x = max_x.max(p.x);
        max_y = max_y.max(p.y);
    }

    // 等比缩放到画布内（长边贴合），居中
    let inner = BLOCK_SIZE as f32 - MARGIN * 2.0;
    let scale = inner / (max_x - min_x).max(max_y - min_y);
    let dx = (BLOCK_SIZE as f32 - (max_x - min_x) * scale) / 2.0;
    let dy = (BLOCK_SIZE as f32 - (max_y - min_y) * scale) / 2.0;
    for p in ver.iter_mut() {
        *p = Point::new((p.x - min_x) * scale + dx, (p.y - min_y) * scale + dy);
    }

    (ver, depths)
}

/// 完整方块三个可见面的展开顶点（行序：0-2未用，3顶面、4北面、5东面）
static MATRIX: LazyLock<[Point; 24]> = LazyLock::new(|| {
    let pts: Vec<Point3> = (0..24).map(|i| VERTICES[INDICES[i]]).collect();
    let (ver, _) = project_fit(&pts);
    ver.try_into().unwrap()
});

/// 每个面的四边形按 (0,1,2) (0,2,3) 展开为两个三角形
const TRI_ORDER: [usize; 6] = [0, 1, 2, 0, 2, 3];

/// 输出图片尺寸
const BLOCK_SIZE: i32 = 256;

/// 游戏帧率（用于APNG延迟：frametime / FRAME_RATE 秒）
const FRAME_RATE: u16 = 20;

/// 平原群系颜色（与wiki物品图标一致）：草/树叶等灰度贴图按此染色（tintindex）
const GRASS_TINT: [u8; 3] = [145, 189, 89]; // #91BD59
const FOLIAGE_TINT: [u8; 3] = [119, 171, 47]; // #77AB2F

fn project(mat: &Mat4, point: &Point3) -> Vec4 {
    let vec = Vec4::new(point.x, point.y, point.z, 1.0);
    let mut res = mat * vec;

    if res.w != 0.0 {
        res.x = res.x / res.w;
        res.y = res.y / res.w;
    }

    res
}

/// 一个面的绘制参数：贴图 + 4个顶点uv（角序同MATRIX展开）
pub type FaceDraw<'a> = (&'a Bitmap, [Point; 4]);

/// 面方向光照：顶面100%、北面80%、东面60%（与游戏内一致）
const FACE_SHADE: [u8; 3] = [255, 204, 153];

/// 一个面的绘制参数：贴图 + 屏幕4角 + 贴图uv4角
struct FaceGeom<'a> {
    tex: &'a Bitmap,
    /// 屏幕4角
    pos: [Point; 4],
    /// 贴图uv4角（角序同pos）
    uv: [Point; 4],
    /// 面亮度预乘（255为全亮）
    shade: u8,
}

fn draw_faces(faces: &[FaceGeom], canvas: &Canvas) {
    for face in faces {
        let mut builder = vertices::Builder::new(
            VertexMode::Triangles,
            6,
            0,
            vertices::BuilderFlags::HAS_TEX_COORDS,
        );

        {
            let positions = builder.positions();
            for (i, pos) in positions.iter_mut().enumerate() {
                *pos = face.pos[TRI_ORDER[i]];
            }
        }

        {
            let tex_coords = builder.tex_coords().unwrap();
            for (i, tex_coord) in tex_coords.iter_mut().enumerate() {
                let uv = face.uv[TRI_ORDER[i]];
                *tex_coord = Point::new(
                    uv.x * face.tex.width() as f32,
                    uv.y * face.tex.height() as f32,
                );
            }
        }

        // 顶点颜色在带贴图shader时不参与混合，改为按面亮度预乘贴图副本
        let shade = face.shade;
        let shaded;
        let tex = if shade == 255 {
            face.tex
        } else {
            shaded = shade_texture(face.tex, [shade; 3]).expect("贴图预乘应成功");
            &shaded
        };

        let mut paint = Paint::default();
        paint.set_anti_alias(true);
        paint.set_shader(tex.to_shader(
            Some((TileMode::Clamp, TileMode::Clamp)),
            SamplingOptions::default(),
            None,
        ));

        canvas.draw_vertices(&builder.detach(), BlendMode::SrcOver, &paint);
    }
}

/// 生成按颜色预乘的贴图副本（只乘RGB，保持alpha不变）
fn shade_texture(tex: &Bitmap, shade: [u8; 3]) -> Option<Bitmap> {
    let info = tex.info().clone();

    // 读出像素
    let mut pixels = vec![0u8; info.min_row_bytes() as usize * tex.height() as usize];
    let image = tex.as_image();
    if !image.read_pixels(
        &info,
        &mut pixels,
        info.min_row_bytes(),
        (0, 0),
        skia_safe::image::CachingHint::Disallow,
    ) {
        return None;
    }

    // 乘颜色
    for px in pixels.chunks_exact_mut(4) {
        px[0] = ((u16::from(px[0]) * u16::from(shade[0])) / 255) as u8;
        px[1] = ((u16::from(px[1]) * u16::from(shade[1])) / 255) as u8;
        px[2] = ((u16::from(px[2]) * u16::from(shade[2])) / 255) as u8;
    }

    // 写入新bitmap
    let mut out = Bitmap::new();
    if !out.set_info(&info, None) {
        return None;
    }
    out.alloc_pixels();
    let size = out.compute_byte_size();
    let dst = unsafe { slice::from_raw_parts_mut(out.pixels() as *mut u8, size) };
    dst.copy_from_slice(&pixels);
    Some(out)
}

/// 解码贴图数据
pub fn decode_png(data: &[u8]) -> Option<Bitmap> {
    let image = Image::from_encoded(Data::new_copy(data))?;

    let dims = image.dimensions();
    let info = ImageInfo::new(
        (dims.width, dims.height),
        ColorType::RGBA8888,
        AlphaType::Premul,
        None,
    );
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
        skia_safe::image::CachingHint::Disallow,
    ) {
        return None;
    }
    Some(bitmap)
}

/// 按游戏规则换算面的4个uv角：基准角 → rotation顺时针旋转 → uv矩形映射（0-16 → 0-1）
fn face_uv(base: &[Point; 4], face: &BlockFaceObj) -> [Point; 4] {
    let steps = (face.rotation.unwrap_or(0) / 90) % 4;
    let uv = face.uv.as_deref().unwrap_or(&[0.0, 0.0, 16.0, 16.0]);
    let (x1, y1, x2, y2) = (uv[0], uv[1], uv[2], uv[3]);
    base.map(|p| {
        let (mut u, mut v) = (p.x, p.y);
        for _ in 0..steps {
            // 游戏rotation为贴图顺时针旋转，对应uv角变换 (u,v)->(v,1-u)
            (u, v) = (v, 1.0 - u);
        }
        Point::new((x1 + u * (x2 - x1)) / 16.0, (y1 + v * (y2 - y1)) / 16.0)
    })
}

/// 渲染单帧并输出 RGBA 数据（直接还原为直通alpha）
fn render_frame(faces: &[FaceDraw; 3]) -> Option<Vec<u8>> {
    let image = render_block(faces)?;

    let out_info = ImageInfo::new(
        (BLOCK_SIZE, BLOCK_SIZE),
        ColorType::RGBA8888,
        AlphaType::Unpremul,
        None,
    );
    let mut out = vec![0u8; out_info.min_row_bytes() as usize * BLOCK_SIZE as usize];
    if !image.read_pixels(
        &out_info,
        &mut out,
        out_info.min_row_bytes(),
        (0, 0),
        skia_safe::image::CachingHint::Disallow,
    ) {
        return None;
    }
    Some(out)
}

/// 渲染一组面到新画布
fn render_geoms(faces: &[FaceGeom]) -> Option<Image> {
    let info = ImageInfo::new(
        (BLOCK_SIZE, BLOCK_SIZE),
        ColorType::RGBA8888,
        AlphaType::Premul,
        None,
    );
    let mut surface = surfaces::raster(&info, None, None)?;
    let canvas = surface.canvas();
    canvas.clear(Color::TRANSPARENT);

    draw_faces(faces, canvas);

    Some(surface.image_snapshot())
}

/// 渲染成单张图片（完整方块三面快捷方式，屏幕坐标来自MATRIX）
fn render_block(faces: &[FaceDraw; 3]) -> Option<Image> {
    let geoms: Vec<FaceGeom> = faces
        .iter()
        .enumerate()
        .map(|(slot, &(tex, uv))| FaceGeom {
            tex,
            pos: [
                MATRIX[(slot + 3) * 4],
                MATRIX[(slot + 3) * 4 + 1],
                MATRIX[(slot + 3) * 4 + 2],
                MATRIX[(slot + 3) * 4 + 3],
            ],
            uv,
            shade: FACE_SHADE[slot],
        })
        .collect();
    render_geoms(&geoms)
}

/// 从竖排动画贴图中取出一帧
fn extract_frame(tex: &Bitmap, index: usize) -> Option<Bitmap> {
    let height = tex.width();
    let top = index as i32 * height;
    let mut frame = Bitmap::new();
    if !tex.extract_subset(&mut frame, &IRect::new(0, top, height, top + height)) {
        return None;
    }
    Some(frame)
}

/// 单贴图三个面的绘制参数（cube_all类，全幅uv）
fn same_faces(tex: &Bitmap) -> [FaceDraw<'_>; 3] {
    [(tex, BASE_UV[0]), (tex, BASE_UV[1]), (tex, BASE_UV[2])]
}

/// 动画条带取第一帧用于静态渲染
fn static_frame(tex: &Bitmap) -> Bitmap {
    if tex.height() > tex.width() {
        extract_frame(tex, 0).unwrap_or_else(|| tex.clone())
    } else {
        tex.clone()
    }
}

/// 贴图只有一张图片（三个面同贴图，全幅uv）
pub fn make_block_png(tex: &Bitmap) -> Option<Image> {
    let frame = static_frame(tex);
    render_block(&same_faces(&frame))
}

/// 将贴图渲染成方块图片（动画贴图随机取一帧）
pub fn make_block_image(tex: &Bitmap) -> Option<Image> {
    let frame = if tex.height() > tex.width() {
        let count = (tex.height() / tex.width()) as usize;
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()?
            .as_nanos();
        extract_frame(tex, nanos as usize % count)?
    } else {
        tex.clone()
    };
    render_block(&same_faces(&frame))
}

/// 贴图是一个动画，拆帧渲染合成APNG（单贴图快捷方式）
pub fn make_block_apng(tex: &Bitmap, meta: AnimMeta) -> Option<Vec<u8>> {
    let face = FaceTexture {
        tex: tex.clone(),
        uv: BASE_UV[0],
        anim: meta,
    };
    make_faces_apng(&[face.clone(), face.clone(), face])
}

/// 一个可见面的渲染输入
#[derive(Clone)]
pub struct FaceTexture {
    /// 面贴图（静态图或动画竖排条带）
    pub tex: Bitmap,
    /// 面4个顶点uv（角序同MATRIX展开，已按模型rotation/uv换算）
    pub uv: [Point; 4],
    /// 动画配置（静态贴图忽略）
    pub anim: AnimMeta,
}

/// 多贴图方块：按各面动画配置展开成逐刻帧序列，逐帧渲染合成APNG
///
/// 静态面每帧复用同一贴图；有动画的面各自按mcmeta的frametime/interpolate展开
pub fn make_faces_apng(faces: &[FaceTexture; 3]) -> Option<Vec<u8>> {
    // 每个面展开成逐刻帧序列
    let face_frames: Vec<Vec<Bitmap>> = faces
        .iter()
        .map(|f| build_face_frames(&f.tex, f.anim))
        .collect::<Option<Vec<_>>>()?;
    let total = face_frames.iter().map(|f| f.len()).max()?;

    let mut frames = Vec::with_capacity(total);
    for k in 0..total {
        let draw = [
            (
                &face_frames[0][k % face_frames[0].len()],
                faces[0].uv,
            ),
            (
                &face_frames[1][k % face_frames[1].len()],
                faces[1].uv,
            ),
            (
                &face_frames[2][k % face_frames[2].len()],
                faces[2].uv,
            ),
        ];
        frames.push(render_frame(&draw)?);
    }
    encode_apng(frames)
}

/// 把一个面的动画展开成逐刻帧序列（interpolate时在相邻帧之间生成过渡帧）
fn build_face_frames(tex: &Bitmap, meta: AnimMeta) -> Option<Vec<Bitmap>> {
    let count = (tex.height() / tex.width()).max(1) as usize;
    if count == 1 {
        return Some(vec![tex.clone()]);
    }

    let src: Vec<Bitmap> = (0..count)
        .map(|i| extract_frame(tex, i))
        .collect::<Option<Vec<_>>>()?;
    let steps = meta.frametime.max(1) as usize;

    if !meta.interpolate {
        // 无插值：每帧停留frametime刻
        let mut out = Vec::with_capacity(count * steps);
        for frame in &src {
            for _ in 0..steps {
                out.push(frame.clone());
            }
        }
        Some(out)
    } else {
        // 有插值：相邻帧间按1刻步长线性过渡（与游戏内平滑动画一致）
        let mut out = Vec::with_capacity(count * steps);
        for (i, a) in src.iter().enumerate() {
            let b = &src[(i + 1) % count];
            for k in 0..steps {
                out.push(blend_bitmap(a, b, k as f32 / steps as f32)?);
            }
        }
        Some(out)
    }
}

/// 按比例混合两张贴图（RGBA线性插值）
fn blend_bitmap(a: &Bitmap, b: &Bitmap, f: f32) -> Option<Bitmap> {
    let info = a.info().clone();
    let size = info.min_row_bytes() as usize * a.height() as usize;

    let mut pa = vec![0u8; size];
    if !a.as_image().read_pixels(
        &info,
        &mut pa,
        info.min_row_bytes(),
        (0, 0),
        skia_safe::image::CachingHint::Disallow,
    ) {
        return None;
    }
    let mut pb = vec![0u8; size];
    if !b.as_image().read_pixels(
        &info,
        &mut pb,
        info.min_row_bytes(),
        (0, 0),
        skia_safe::image::CachingHint::Disallow,
    ) {
        return None;
    }

    let mut pixels = vec![0u8; size];
    for (o, (x, y)) in pixels.chunks_exact_mut(4).zip(pa.chunks_exact(4).zip(pb.chunks_exact(4))) {
        for c in 0..4 {
            o[c] = (f32::from(x[c]) * (1.0 - f) + f32::from(y[c]) * f).round() as u8;
        }
    }

    let mut out = Bitmap::new();
    if !out.set_info(&info, None) {
        return None;
    }
    out.alloc_pixels();
    let size = out.compute_byte_size();
    let dst = unsafe { slice::from_raw_parts_mut(out.pixels() as *mut u8, size) };
    dst.copy_from_slice(&pixels);
    Some(out)
}

/// 合并成APNG（逐刻帧：1刻 = FRAME_RATE分之一秒）
fn encode_apng(frames: Vec<Vec<u8>>) -> Option<Vec<u8>> {
    let size = BLOCK_SIZE as u32;

    let mut cursor = Cursor::new(Vec::new());
    {
        let mut encoder = png::Encoder::new(&mut cursor, size, size);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        if let Err(e) = encoder.set_animated(frames.len() as u32, 0) {
            eprintln!("set_animated失败: {e}");
            return None;
        }
        if let Err(e) = encoder.set_frame_delay(1, FRAME_RATE) {
            eprintln!("set_frame_delay失败: {e}");
            return None;
        }
        let mut writer = match encoder.write_header() {
            Ok(writer) => writer,
            Err(e) => {
                eprintln!("write_header失败: {e}");
                return None;
            }
        };
        for frame in &frames {
            if let Err(e) = writer.write_image_data(frame) {
                eprintln!("write_image_data失败: {e}");
                return None;
            }
        }
        if let Err(e) = writer.finish() {
            eprintln!("finish失败: {e}");
            return None;
        }
    }
    Some(cursor.into_inner())
}


/// 方块模型（只取需要的字段）
#[derive(Serialize, Deserialize, Default)]
struct BlockModelObj {
    parent: Option<String>,
    textures: Option<HashMap<String, TextureRefObj>>,
    display: Option<BlockDisplayObj>,
    elements: Option<Vec<BlockElementObj>>,
}

/// gui显示变换（只取旋转，决定模型在图标视角下的朝向）
#[derive(Serialize, Deserialize, Default)]
struct BlockDisplayObj {
    gui: Option<GuiRotateObj>,
}

#[derive(Serialize, Deserialize, Default)]
struct GuiRotateObj {
    rotation: Option<Vec<f32>>,
}

/// textures表的值：老版为字符串，新版可为对象（如glass的force_translucent）
#[derive(Serialize, Deserialize)]
#[serde(untagged)]
enum TextureRefObj {
    Plain(String),
    Object { sprite: String },
}

impl TextureRefObj {
    fn value(&self) -> Option<&str> {
        match self {
            Self::Plain(value) => Some(value),
            Self::Object { sprite } => Some(sprite),
        }
    }
}

/// 模型元素（轴对齐盒子，from/to为0-16坐标）
#[derive(Clone, Serialize, Deserialize, Default)]
struct BlockElementObj {
    /// 缺省即完整立方体边界（与游戏规则一致）
    #[serde(default = "default_origin")]
    from: Vec<f32>,
    #[serde(default = "default_size")]
    to: Vec<f32>,
    faces: Option<HashMap<String, BlockFaceObj>>,
}

fn default_origin() -> Vec<f32> {
    vec![0.0, 0.0, 0.0]
}

fn default_size() -> Vec<f32> {
    vec![16.0, 16.0, 16.0]
}

/// 元素面的贴图引用（uv与rotation按游戏规则换算）
#[derive(Clone, Serialize, Deserialize, Default)]
struct BlockFaceObj {
    texture: Option<String>,
    /// 默认[0,0,16,16]，格式[u1,v1,u2,v2]
    uv: Option<Vec<f32>>,
    /// 贴图顺时针旋转（0/90/180/270，down面为逆时针）
    rotation: Option<u32>,
    /// 群系染色索引（草/树叶等灰度贴图需要乘群系颜色）
    tintindex: Option<u32>,
}

/// 完整方块模板：模型沿parent链向上，第一个带elements的祖先必须是这些。
/// 半砖/楼梯/十字/栏杆等非完整方块模板不在此列，直接跳过。
/// 模板外的自带elements模型（observer/釉陶/grass_block等）由几何判定兜底
const FULL_CUBE_TEMPLATES: &[&str] = &[
    "block/cube",
    "block/cube_all_inner_faces",
    "block/cube_bottom_top_inner_faces",
    "block/cube_column_horizontal",
    "block/cube_column_uv_locked_x",
    "block/cube_column_uv_locked_y",
    "block/cube_column_uv_locked_z",
    "block/cube_directional",
    "block/cube_mirrored",
    "block/cube_north_west_mirrored",
    "block/leaves",
];

/// 几何判定完整方块：所有元素都占满0,0,0→16,16,16
/// （如grass_block的侧面overlay元素，取第一个元素的六面渲染，overlay忽略）
fn is_full_cube(elements: &[BlockElementObj]) -> bool {
    elements.iter().all(|e| e.from == [0.0, 0.0, 0.0] && e.to == [16.0, 16.0, 16.0])
}

/// 楼梯模板（多元素盒子模型）
const STAIRS_TEMPLATES: &[&str] = &["block/stairs", "block/inner_stairs", "block/outer_stairs"];

/// 全部6个面的定义：面名、INDICES行号（角点取自VERTICES，角序即uv展开序）、基准uv角、面法线
/// （顶/北/东的uv角与BASE_UV一致，其余三面按游戏规则镜像推导）
static FACE_DEFS: [(&str, usize, [Point; 4], Point3); 6] = [
    // 西面：u与东面镜像
    (
        "west",
        0,
        [
            Point::new(0.0, 1.0),
            Point::new(1.0, 1.0),
            Point::new(1.0, 0.0),
            Point::new(0.0, 0.0),
        ],
        Point3::new(-1.0, 0.0, 0.0),
    ),
    // 底面（gui视角不可见，与顶面同规则）
    (
        "down",
        1,
        [
            Point::new(0.0, 1.0),
            Point::new(1.0, 1.0),
            Point::new(1.0, 0.0),
            Point::new(0.0, 0.0),
        ],
        Point3::new(0.0, -1.0, 0.0),
    ),
    // 南面：u与北面镜像
    (
        "south",
        2,
        [
            Point::new(1.0, 1.0),
            Point::new(0.0, 1.0),
            Point::new(0.0, 0.0),
            Point::new(1.0, 0.0),
        ],
        Point3::new(0.0, 0.0, 1.0),
    ),
    // 顶面：u沿世界z（已与wiki木板顶面纹理走向比对确认）
    (
        "up",
        3,
        [
            Point::new(0.0, 1.0),
            Point::new(1.0, 1.0),
            Point::new(1.0, 0.0),
            Point::new(0.0, 0.0),
        ],
        Point3::new(0.0, 1.0, 0.0),
    ),
    // 北面
    (
        "north",
        4,
        [
            Point::new(0.0, 1.0),
            Point::new(1.0, 1.0),
            Point::new(1.0, 0.0),
            Point::new(0.0, 0.0),
        ],
        Point3::new(0.0, 0.0, -1.0),
    ),
    // 东面
    (
        "east",
        5,
        [
            Point::new(1.0, 1.0),
            Point::new(0.0, 1.0),
            Point::new(0.0, 0.0),
            Point::new(1.0, 0.0),
        ],
        Point3::new(1.0, 0.0, 0.0),
    ),
];

/// 元素盒子某面的4个模型空间角点（0-16→±1，角点取自VERTICES，与完整方块几何一致）
fn element_face_points(from: &[f32], to: &[f32], row: usize) -> [Point3; 4] {
    (0..4)
        .map(|i| VERTICES[INDICES[row * 4 + i]])
        .map(|v| Point3 {
            x: (if v.x > 0.0 { to[0] } else { from[0] }) / 8.0 - 1.0,
            y: (if v.y > 0.0 { to[1] } else { from[1] }) / 8.0 - 1.0,
            z: (if v.z > 0.0 { to[2] } else { from[2] }) / 8.0 - 1.0,
        })
        .collect::<Vec<_>>()
        .try_into()
        .unwrap()
}

/// 绕y轴旋转一个模型空间点（gui旋转与完整方块[30,225,0]的yaw差）
fn rot_point_y(p: Point3, sin: f32, cos: f32) -> Point3 {
    Point3 {
        x: p.x * cos + p.z * sin,
        y: p.y,
        z: -p.x * sin + p.z * cos,
    }
}

/// 渲染楼梯类多元素盒子模型：按gui旋转角刚体旋转所有元素面，
/// 投影后剔除背向观察者的面，按深度排序绘制（painter's algorithm）
fn render_model_png(
    archive: &BaseArchive,
    tex_cache: &mut HashMap<String, Bitmap>,
    textures: &HashMap<String, TextureRefObj>,
    elements: &[BlockElementObj],
    rot_y: f32,
) -> Option<Vec<u8>> {
    // 画布四周留边距
    const MARGIN: f32 = 2.0;
    let mat = *MAT;
    // 模型gui旋转与完整方块[30,225,0]的yaw差，绕y轴刚体旋转
    let angle = (rot_y - 225.0) * PI / 180.0;
    let (sin, cos) = angle.sin_cos();

    // 载入朝向观察者的面并旋转角点
    let mut texs: Vec<Bitmap> = Vec::new();
    let mut uvs: Vec<[Point; 4]> = Vec::new();
    let mut shades: Vec<u8> = Vec::new();
    let mut corner_sets: Vec<[Point3; 4]> = Vec::new();
    for element in elements {
        let faces = element.faces.as_ref()?;
        for (name, row, base_uv, normal) in FACE_DEFS {
            let Some(obj) = faces.get(name) else {
                continue;
            };
            // 旋转后的面法线：投影z<0说明面朝向观察者
            let n = rot_point_y(normal, sin, cos);
            if project(&mat, &n).z >= 0.0 {
                continue;
            }
            let ft = load_face(archive, tex_cache, textures, obj, &base_uv)?;
            #[cfg(debug_assertions)]
            if std::env::var("MCML_UV_DEBUG").is_ok() {
                eprintln!("[uv] element {:?} face {name}: obj.uv={:?} -> ft.uv={:?}", element.from, obj.uv, ft.uv);
            }
            // 楼梯贴图均为静态，动画贴图兜底取第一帧
            texs.push(static_frame(&ft.tex));
            uvs.push(ft.uv);
            // 面亮度按旋转后的朝向：顶面全亮、南北向0.8、东西向0.6（与游戏一致）
            shades.push(if n.y > 0.5 {
                255
            } else if n.y < -0.5 {
                128
            } else if n.z.abs() > n.x.abs() {
                204
            } else {
                153
            });
            let corners = element_face_points(&element.from, &element.to, row);
            corner_sets.push(corners.map(|c| rot_point_y(c, sin, cos)));
        }
    }
    if texs.is_empty() {
        return None;
    }

    // 投影全部角点，计算全模型包围盒fit（与完整方块同尺度）
    let mut pos_sets: Vec<[Point; 4]> = Vec::with_capacity(corner_sets.len());
    let mut depth_sets: Vec<[f32; 4]> = Vec::with_capacity(corner_sets.len());
    let (mut min_x, mut min_y) = (f32::INFINITY, f32::INFINITY);
    let (mut max_x, mut max_y) = (f32::NEG_INFINITY, f32::NEG_INFINITY);
    for cs in &corner_sets {
        let mut ps = [Point::default(); 4];
        let mut ds = [0.0f32; 4];
        for (i, c) in cs.iter().enumerate() {
            let res = project(&mat, c);
            ps[i] = Point { x: res.x, y: res.y };
            ds[i] = res.z;
            min_x = min_x.min(ps[i].x);
            min_y = min_y.min(ps[i].y);
            max_x = max_x.max(ps[i].x);
            max_y = max_y.max(ps[i].y);
        }
        pos_sets.push(ps);
        depth_sets.push(ds);
    }
    let inner = BLOCK_SIZE as f32 - MARGIN * 2.0;
    let scale = inner / (max_x - min_x).max(max_y - min_y);
    let dx = (BLOCK_SIZE as f32 - (max_x - min_x) * scale) / 2.0;
    let dy = (BLOCK_SIZE as f32 - (max_y - min_y) * scale) / 2.0;

    // 组装面并按深度排序（远的先画）
    let mut geoms: Vec<(f32, FaceGeom)> = texs
        .iter()
        .enumerate()
        .map(|(i, tex)| {
            let fit =
                |p: Point| Point::new((p.x - min_x) * scale + dx, (p.y - min_y) * scale + dy);
            let depth = depth_sets[i].iter().sum::<f32>();
            (
                depth,
                FaceGeom {
                    tex,
                    pos: pos_sets[i].map(fit),
                    uv: uvs[i],
                    shade: shades[i],
                },
            )
        })
        .collect();
    geoms.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    let faces: Vec<FaceGeom> = geoms.into_iter().map(|(_, g)| g).collect();

    let img = render_geoms(&faces)?;
    #[allow(deprecated)]
    let data = img.encode_to_data(EncodedImageFormat::PNG)?;
    Some(data.as_bytes().to_vec())
}

/// 沿parent链解析模型：合并textures（子覆盖父），返回
/// （合并后的textures，第一个带elements祖先的全部元素，该祖先的模型相对名，
/// 是否完整方块，gui旋转y分量）
fn resolve_model(
    archive: &BaseArchive,
    rel: &str,
) -> Option<(
    HashMap<String, TextureRefObj>,
    Vec<BlockElementObj>,
    String,
    bool,
    f32,
)> {
    let mut textures: HashMap<String, TextureRefObj> = HashMap::new();
    let mut gui_rot: Option<Vec<f32>> = None;
    let mut current = rel.to_string();
    for _ in 0..16 {
        let data = archive
            .read(&format!("assets/minecraft/models/{current}.json"))
            .ok()?;
        let model = serialize_tools::json_from_bytes::<BlockModelObj>(&data).ok()?;

        // 子模型的textures/display覆盖父模型
        let mut merged = model.textures.unwrap_or_default();
        for (k, v) in textures {
            merged.entry(k).or_insert(v);
        }
        textures = merged;
        if let Some(rot) = model
            .display
            .and_then(|d| d.gui)
            .and_then(|g| g.rotation)
        {
            gui_rot = Some(rot);
        }

        // 第一个带elements的祖先提供全部元素定义
        if let Some(elements) = &model.elements {
            if elements.is_empty() || elements[0].faces.is_none() {
                return None;
            }
            let full = is_full_cube(elements);
            // gui旋转y分量（block/block默认[30,225,0]）
            let rot_y = gui_rot.and_then(|r| r.get(1).copied()).unwrap_or(225.0);
            return Some((textures, elements.clone(), current, full, rot_y));
        }

        let parent = model.parent?;
        current = parent.strip_prefix("minecraft:").unwrap_or(&parent).to_string();
    }
    None
}

/// 解析贴图引用链（"#side" -> textures里的值，值可能还是引用）
fn resolve_ref(textures: &HashMap<String, TextureRefObj>, r: &str) -> Option<String> {
    let mut r = r;
    for _ in 0..8 {
        // 不是引用即为最终贴图值
        let Some(rest) = r.strip_prefix('#') else {
            return Some(r.to_string());
        };
        r = textures.get(rest)?.value()?;
    }
    None
}

/// 贴图引用值 -> jar内路径："minecraft:block/stone" -> assets/minecraft/textures/block/stone.png
/// 只支持minecraft命名空间（客户端jar里没有其他命名空间的贴图）
fn texture_path(value: &str) -> Option<String> {
    let (ns, path) = value.split_once(':').unwrap_or(("minecraft", value));
    if ns != "minecraft" {
        return None;
    }
    Some(format!("assets/minecraft/textures/{path}.png"))
}

/// 载入一个面的渲染输入（贴图缓存复用，uv按模型换算）
fn load_face(
    archive: &BaseArchive,
    tex_cache: &mut HashMap<String, Bitmap>,
    textures: &HashMap<String, TextureRefObj>,
    face_obj: &BlockFaceObj,
    base_uv: &[Point; 4],
) -> Option<FaceTexture> {
    let value = resolve_ref(textures, face_obj.texture.as_deref()?)?;
    let path = texture_path(&value)?;
    let tex = if let Some(tex) = tex_cache.get(&path) {
        tex.clone()
    } else {
        let bytes = archive.read(&path).ok()?;
        let tex = decode_png(&bytes)?;
        tex_cache.insert(path.clone(), tex.clone());
        tex
    };

    // 灰度贴图按群系色染色：树叶用植被色，其余（草顶/草侧overlay）用草色，
    // 取平原群系（与wiki物品图标一致）
    let tex = if face_obj.tintindex.is_some() {
        let tint = if path.contains("leaves") {
            FOLIAGE_TINT
        } else {
            GRASS_TINT
        };
        shade_texture(&tex, tint)?
    } else {
        tex
    };
    Some(FaceTexture {
        tex,
        uv: face_uv(base_uv, face_obj),
        anim: read_anim_meta(archive, &path),
    })
}

/// 完整方块：取第一个元素的六面定义，载入三个可见面渲染（动画贴图存APNG）
fn render_full_cube(
    archive: &BaseArchive,
    tex_cache: &mut HashMap<String, Bitmap>,
    textures: &HashMap<String, TextureRefObj>,
    elements: &[BlockElementObj],
) -> Option<Vec<u8>> {
    let face_objs = elements.first()?.faces.as_ref()?;

    // 载入三个可见面（顶、北、东），任一面解析失败则跳过该方块
    let mut slots: [Option<FaceTexture>; 3] = [None, None, None];
    for (slot, face_name) in ["up", "north", "east"].iter().enumerate() {
        match face_objs
            .get(*face_name)
            .and_then(|obj| load_face(archive, tex_cache, textures, obj, &BASE_UV[slot]))
        {
            Some(face) => slots[slot] = Some(face),
            None => return None,
        }
    }
    let [Some(top), Some(north), Some(east)] = slots else {
        return None;
    };
    let faces = [top, north, east];

    // 动画贴图存成APNG，静态贴图存成PNG
    if faces.iter().any(|f| f.tex.height() > f.tex.width()) {
        make_faces_apng(&faces)
    } else {
        #[allow(deprecated)]
        render_block(&[
            (&faces[0].tex, faces[0].uv),
            (&faces[1].tex, faces[1].uv),
            (&faces[2].tex, faces[2].uv),
        ])
        .and_then(|img| img.encode_to_data(EncodedImageFormat::PNG))
        .map(|d| d.as_bytes().to_vec())
    }
}

/// 从客户端jar提取所有完整方块贴图并渲染保存
///
/// 下载流程（版本清单 → 客户端jar）见lib的load_blocks，这里只做解包渲染。
/// 支持多贴图方块（cube_bottom_top/cube_column/orientable等）：
/// 沿parent链合并textures，取第一个带elements祖先的面定义，
/// 只画当前视角可见的三个面（顶、北、东）
pub fn render_blocks(archive: &BaseArchive) -> CoreResult<()> {
    let dir = crate::get_block_dir().ok_or(ErrorType::DownloadFileFail)?;

    // 提取语言文件并构建 ID->语言键 映射
    let names = extract_langs(archive);

    // 贴图解码缓存（同一贴图常被多个方块/多个面共用）
    let mut tex_cache: HashMap<String, Bitmap> = HashMap::new();

    for entry in archive.entries() {
        let name = entry.name.as_str();
        if !name.starts_with("assets/minecraft/models/block/") || !name.ends_with(".json") {
            continue;
        }

        // 模型相对名与方块ID："block/stone" -> "minecraft:stone"
        let rel = name
            .trim_start_matches("assets/minecraft/models/")
            .trim_end_matches(".json");
        let id_rel = name
            .trim_start_matches("assets/minecraft/models/block/")
            .trim_end_matches(".json");
        let id = format!("minecraft:{id_rel}");

        // 完整方块或楼梯才渲染，半砖/玻璃板等跳过
        let Some((textures, elements, template, full, rot_y)) = resolve_model(archive, rel) else {
            continue;
        };
        let is_stairs = !full && STAIRS_TEMPLATES.contains(&template.as_str());
        if !full && !is_stairs && !FULL_CUBE_TEMPLATES.contains(&template.as_str()) {
            continue;
        }
        // gui旋转与完整方块[30,225,0]的yaw差为0时走完整方块快捷路径
        let rot_steps = ((225.0 - rot_y) / 90.0).rem_euclid(4.0) as usize;

        let data = if !full || rot_steps != 0 {
            // 多元素/带旋转的模型：按gui旋转角渲染全部可见面
            render_model_png(archive, &mut tex_cache, &textures, &elements, rot_y)
        } else {
            render_full_cube(archive, &mut tex_cache, &textures, &elements)
        };
        let Some(data) = data else {
            continue;
        };

        let out_name = format!("minecraft_{id_rel}.png");
        if path_helper::write_bytes(&dir.join(&out_name), &data).is_err() {
            continue;
        }

        let mut blocks = crate::blocks_write();
        blocks.tex.insert(id.clone(), out_name);
        if let Some(lang_key) = names.get(&id) {
            blocks.name.insert(id, lang_key.clone());
        }
    }

    Ok(())
}

/// 提取语言文件到blocks/langs目录，并构建方块ID→语言键映射（zh_cn优先，en_us兜底；新版jar可能只有en_us）
fn extract_langs(archive: &BaseArchive) -> std::collections::HashMap<String, String> {
    for name in ["zh_cn", "en_us"] {
        let Ok(data) = archive.read(&format!("assets/minecraft/lang/{name}.json")) else {
            continue;
        };
        // 复制到langs目录，供get_lang查询翻译
        if let Some(dir) = crate::get_lang_dir() {
            let _ = path_helper::write_bytes(dir.join(format!("{name}.json")), &data);
        }
        let Ok(buf) = String::from_utf8(data) else {
            continue;
        };
        let Ok(lang) = serialize_tools::json_from_str::<
            std::collections::HashMap<String, String>,
        >(&buf) else {
            continue;
        };
        // 方块ID → 语言键：block.minecraft.stone → minecraft:stone: block.minecraft.stone
        //（Name只存语言键，由GUI按当前语言经get_lang翻译）
        let mut map = std::collections::HashMap::new();
        for lang_key in lang.into_keys() {
            let keys: Vec<&str> = lang_key.split('.').collect();
            if keys.len() == 3 && keys[0] == "block" {
                map.insert(format!("{}:{}", keys[1], keys[2]), lang_key);
            }
        }
        return map;
    }
    std::collections::HashMap::new()
}

/// 动画贴图配置
#[derive(Clone, Copy)]
pub struct AnimMeta {
    /// 帧间隔（单位：游戏刻，1刻 = 50ms）
    pub frametime: u32,
    /// 帧之间是否平滑插值
    pub interpolate: bool,
}

/// 动画贴图配置
#[derive(Serialize, Deserialize, Default)]
struct TextureMetaObj {
    animation: Option<TextureAnimationObj>,
}

#[derive(Serialize, Deserialize, Default)]
struct TextureAnimationObj {
    frametime: Option<u32>,
    interpolate: Option<bool>,
}

/// 读取动画贴图配置（帧间隔、是否插值）
pub fn read_anim_meta(archive: &BaseArchive, tex_path: &str) -> AnimMeta {
    let fallback = AnimMeta {
        frametime: 1,
        interpolate: false,
    };
    let Ok(data) = archive.read(&format!("{tex_path}.mcmeta")) else {
        return fallback;
    };
    let Ok(buf) = String::from_utf8(data) else {
        return fallback;
    };
    let Ok(meta) = serialize_tools::json_from_str::<TextureMetaObj>(&buf) else {
        return fallback;
    };
    let Some(animation) = meta.animation else {
        return fallback;
    };
    AnimMeta {
        frametime: animation.frametime.filter(|t| *t > 0).unwrap_or(1),
        interpolate: animation.interpolate.unwrap_or(false),
    }
}

/// 当前已渲染的游戏版本
pub(crate) fn mcml_tex_draw_id() -> String {
    crate::blocks_read().id.clone()
}
