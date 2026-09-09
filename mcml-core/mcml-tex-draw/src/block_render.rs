use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    f32::consts::PI,
    io::Cursor,
    slice,
    sync::{LazyLock, atomic::{AtomicU64, AtomicUsize, Ordering}},
};

use glam::{Mat4, Vec3, Vec4};
use mcml_base::{archives::BaseArchive, serialize_tools};
use mcml_game::gui_hook::ProgressGui;
use rayon::prelude::*;
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

/// 三个可见面（顶、北、东）的基准uv角，角序与INDICES的展开顺序一致（取自FACE_DEFS）
static BASE_UV: LazyLock<[[Point; 4]; 3]> =
    LazyLock::new(|| [FACE_DEFS[3].2, FACE_DEFS[4].2, FACE_DEFS[5].2]);

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

/// 完整方块投影的fit参数（min_x, min_y, scale, dx, dy）。
/// 半砖等矮模型按完整方块的比例绘制（不放大铺满画布），与游戏图标一致
static FULL_FIT: LazyLock<(f32, f32, f32, f32, f32)> = LazyLock::new(|| {
    // 画布四周留边距
    const MARGIN: f32 = 2.0;
    let mut min_x = f32::INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut max_y = f32::NEG_INFINITY;
    for p in &VERTICES {
        let res = project(&MAT, p);
        min_x = min_x.min(res.x);
        min_y = min_y.min(res.y);
        max_x = max_x.max(res.x);
        max_y = max_y.max(res.y);
    }
    let inner = BLOCK_SIZE as f32 - MARGIN * 2.0;
    let scale = inner / (max_x - min_x).max(max_y - min_y);
    let dx = (BLOCK_SIZE as f32 - (max_x - min_x) * scale) / 2.0;
    let dy = (BLOCK_SIZE as f32 - (max_y - min_y) * scale) / 2.0;
    (min_x, min_y, scale, dx, dy)
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
/// 固定色树叶（游戏内不随群系变化）
const BIRCH_TINT: [u8; 3] = [128, 167, 85]; // #80A755
const SPRUCE_TINT: [u8; 3] = [97, 153, 97]; // #619961

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
///
/// 面未显式写uv时，按游戏规则从元素盒子的from/to投影到该面自动生成
/// （楼梯等元素模型依赖此规则，缺省不能当全幅处理）
fn face_uv(
    base: &[Point; 4],
    face: &BlockFaceObj,
    face_name: &str,
    from: &[f32],
    to: &[f32],
) -> [Point; 4] {
    let steps = (face.rotation.unwrap_or(0) / 90) % 4;
    let default_uv = match face_name {
        // 与游戏 FaceBakery 的自动uv规则一致
        "down" => [from[0], 16.0 - to[2], to[0], 16.0 - from[2]],
        "up" => [from[0], from[2], to[0], to[2]],
        "north" => [16.0 - to[0], 16.0 - to[1], 16.0 - from[0], 16.0 - from[1]],
        "south" => [from[0], 16.0 - to[1], to[0], 16.0 - from[1]],
        "west" => [from[2], 16.0 - to[1], to[2], 16.0 - from[1]],
        "east" => [16.0 - to[2], 16.0 - to[1], 16.0 - from[2], 16.0 - from[1]],
        _ => [0.0, 0.0, 16.0, 16.0],
    };
    let [x1, y1, x2, y2] = face
        .uv
        .as_deref()
        .and_then(|uv| <[f32; 4]>::try_from(uv).ok())
        .unwrap_or(default_uv);
    // 顶/底面的基准u沿世界z、v沿x：相对矩形分量是(u取y范围, v取16-x)的90°旋转，
    // 局部uv（楼梯上半块顶面[8,0,16,16]等）才能与面的两边尺寸1:1对应，
    // 否则半张会被拉伸到长边上（表现为整张贴图铺满）；
    // 整块矩形[0,16,0,16]经过该变换不变，已验证的整块顶/底面外观不受影响
    let (u1, v1, u2, v2) = if face_name == "up" || face_name == "down" {
        (y1, 16.0 - x2, y2, 16.0 - x1)
    } else {
        (x1, y1, x2, y2)
    };
    base.map(|p| {
        let (mut u, mut v) = (p.x, p.y);
        for _ in 0..steps {
            // 游戏rotation为贴图顺时针旋转，对应uv角变换 (u,v)->(v,1-u)
            (u, v) = (v, 1.0 - u);
        }
        Point::new((u1 + u * (u2 - u1)) / 16.0, (v1 + v * (v2 - v1)) / 16.0)
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

/// 可见面slot（0顶/1北/2东）在屏幕上的4角（来自MATRIX展开）
fn slot_pos(slot: usize) -> [Point; 4] {
    let base = (slot + 3) * 4;
    [MATRIX[base], MATRIX[base + 1], MATRIX[base + 2], MATRIX[base + 3]]
}

/// 渲染成单张图片（完整方块三面快捷方式，屏幕坐标来自MATRIX）
fn render_block(faces: &[FaceDraw; 3]) -> Option<Image> {
    let geoms: Vec<FaceGeom> = faces
        .iter()
        .enumerate()
        .map(|(slot, &(tex, uv))| FaceGeom {
            tex,
            pos: slot_pos(slot),
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
/// 静态面每帧复用同一贴图；有动画的面各自按mcmeta的frametime/interpolate展开。
/// 逐刻全渲染可达上千帧（如prismarine frametime=300），按最多[MAX_APNG_FRAMES]帧
/// 对逐刻时间线等距采样，采样步长代表的持续刻数并入APNG帧延迟；
/// 渲染结果相同的相邻帧合并延迟（无插值动画采样后多为同帧）
pub fn make_faces_apng(faces: &[FaceTexture; 3]) -> Option<Vec<u8>> {
    // 每个面展开成动画时间线（按刻惰性取帧，不预先物化全部帧位图）
    let timelines: Vec<AnimTimeline> = faces
        .iter()
        .map(|f| AnimTimeline::build(&f.tex, &f.anim))
        .collect::<Option<Vec<_>>>()?;
    let totals: Vec<u32> = timelines.iter().map(|t| t.total_ticks()).collect();
    let total = totals.iter().copied().max()?;

    // 最多渲染MAX_APNG_FRAMES帧，等距采样逐刻时间线
    const MAX_APNG_FRAMES: usize = 64;
    let stride = total.div_ceil(MAX_APNG_FRAMES as u32);

    let mut frames: Vec<(Vec<u8>, u16)> = Vec::with_capacity((total / stride + 1) as usize);
    let mut k: u32 = 0;
    while k < total {
        let owned: Vec<(Bitmap, [Point; 4])> = timelines
            .iter()
            .zip(faces.iter())
            .enumerate()
            .map(|(j, (tl, f))| tl.frame_at(k % totals[j]).map(|bm| (bm, f.uv)))
            .collect::<Option<Vec<_>>>()?;
        let draw: [FaceDraw; 3] = std::array::from_fn(|j| (&owned[j].0, owned[j].1));
        let img = render_frame(&draw)?;
        // 该帧在逐刻时间线上代表的刻数，即APNG显示时长
        let ticks = stride.min(total - k) as u16;
        match frames.last_mut() {
            // 渲染结果与上一帧相同：并入其显示时长，不重复存帧
            Some((last_img, last_ticks)) if *last_img == img => *last_ticks += ticks,
            _ => frames.push((img, ticks)),
        }
        k += stride;
    }
    encode_apng(frames)
}

/// 动画时间线：源帧 + 展开后的帧序列，按刻惰性取帧。
/// 取帧时才做克隆/插值混合，避免物化整条逐刻帧序列（frametime大时可达上千帧位图）
struct AnimTimeline {
    src: Vec<Bitmap>,
    entries: Vec<(u32, u32)>,
    interpolate: bool,
}

impl AnimTimeline {
    fn build(tex: &Bitmap, meta: &AnimMeta) -> Option<Self> {
        let count = (tex.height() / tex.width()).max(1) as usize;
        let src: Vec<Bitmap> = (0..count)
            .map(|i| extract_frame(tex, i))
            .collect::<Option<Vec<_>>>()?;
        let frametime = meta.frametime.max(1);
        // frames列表优先（帧号越界取模兼容）；缺省按0..n顺序、每帧frametime刻
        let entries = meta.frames.as_ref().map(|list| {
            list.iter()
                .map(|f| (f.index % src.len() as u32, f.time.max(1)))
                .collect::<Vec<_>>()
        })
        .unwrap_or_else(|| (0..src.len() as u32).map(|i| (i, frametime)).collect());
        Some(Self {
            src,
            entries,
            interpolate: meta.interpolate,
        })
    }

    fn total_ticks(&self) -> u32 {
        self.entries.iter().map(|&(_, time)| time).sum()
    }

    /// 取逐刻时间线上第tick刻的帧；interpolate时向序列下一帧线性过渡
    fn frame_at(&self, tick: u32) -> Option<Bitmap> {
        let mut acc = 0u32;
        for (i, &(idx, time)) in self.entries.iter().enumerate() {
            if tick < acc + time {
                let frame = &self.src[idx as usize];
                if !self.interpolate {
                    return Some(frame.clone());
                }
                // 向序列下一帧线性过渡（与游戏内平滑动画一致）
                let next = self.entries[(i + 1) % self.entries.len()].0 as usize;
                let f = (tick - acc) as f32 / time as f32;
                return blend_bitmap(frame, &self.src[next], f);
            }
            acc += time;
        }
        None
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

/// 合并成APNG（每帧附带显示时长，单位：刻，1刻 = FRAME_RATE分之一秒）
fn encode_apng(frames: Vec<(Vec<u8>, u16)>) -> Option<Vec<u8>> {
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
        let mut writer = match encoder.write_header() {
            Ok(writer) => writer,
            Err(e) => {
                eprintln!("write_header失败: {e}");
                return None;
            }
        };
        for (frame, ticks) in &frames {
            // 帧显示时长 = 持续刻数 / FRAME_RATE 秒（采样跨过的刻数一并计入）
            if let Err(e) = writer.set_frame_delay(*ticks, FRAME_RATE) {
                eprintln!("set_frame_delay失败: {e}");
                return None;
            }
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
/// （公开供手动测试调试单个方块用）
#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum TextureRefObj {
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
/// （公开供手动测试调试单个方块用）
#[derive(Clone, Serialize, Deserialize, Default)]
pub struct BlockElementObj {
    /// 缺省即完整立方体边界（与游戏规则一致）
    #[serde(default = "default_origin")]
    from: Vec<f32>,
    #[serde(default = "default_size")]
    to: Vec<f32>,
    faces: Option<HashMap<String, BlockFaceObj>>,
    /// 元素旋转（十字植物为绕y轴45°薄平面，营火原木/吊灯/紫水晶等也用到）
    rotation: Option<ElementRotationObj>,
    /// 面朝向着色开关（十字等设为false，游戏内全亮）
    shade: Option<bool>,
}

/// 元素旋转参数（绕origin绕轴旋转angle度，rescale时垂直轴放大1/cos(angle)）
#[derive(Clone, Serialize, Deserialize, Default)]
struct ElementRotationObj {
    #[serde(default = "default_rot_origin")]
    origin: Vec<f32>,
    #[serde(default)]
    axis: Option<String>,
    #[serde(default)]
    angle: Option<f32>,
    #[serde(default)]
    rescale: Option<bool>,
}

fn default_rot_origin() -> Vec<f32> {
    vec![8.0, 8.0, 8.0]
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
    /// 未写uv时按元素盒子from/to自动生成（全幅元素即[0,0,16,16]），格式[u1,v1,u2,v2]
    uv: Option<Vec<f32>>,
    /// 贴图顺时针旋转（0/90/180/270，down面为逆时针）
    rotation: Option<u32>,
    /// 群系染色索引（草/树叶等灰度贴图需要乘群系颜色）
    tintindex: Option<u32>,
}

/// 几何判定完整方块：所有元素都占满0,0,0→16,16,16
/// （第一个元素为基准六面，其余元素由render_full_cube作为overlay叠绘，如grass_block）
fn is_full_cube(elements: &[BlockElementObj]) -> bool {
    elements.iter().all(|e| e.from == [0.0, 0.0, 0.0] && e.to == [16.0, 16.0, 16.0])
}

/// 分阶段耗时统计（纳秒累计），设置 MCML_RENDER_PROFILE=1 时在渲染结束后打印
/// （用于定位并发渲染的性能瓶颈，正常路径零开销仅两次fetch_add）
static PROFILE: LazyLock<RenderProfile> = LazyLock::new(|| RenderProfile {
    enabled: std::env::var("MCML_RENDER_PROFILE").is_ok(),
    ..Default::default()
});

#[derive(Default)]
struct RenderProfile {
    enabled: bool,
    resolve: AtomicU64,
    draw: AtomicU64,
    write: AtomicU64,
}

impl RenderProfile {
    fn print(&self, total: std::time::Duration) {
        if !self.enabled {
            return;
        }
        let f = |nanos: u64| nanos as f64 / 1e9;
        println!(
            "[性能] 总计{:.1}s | resolve {:.1}s | draw {:.1}s | write {:.1}s | 其他 {:.1}s",
            total.as_secs_f64(),
            f(self.resolve.load(Ordering::Relaxed)),
            f(self.draw.load(Ordering::Relaxed)),
            f(self.write.load(Ordering::Relaxed)),
            total.as_secs_f64()
                - f(self.resolve.load(Ordering::Relaxed))
                - f(self.draw.load(Ordering::Relaxed))
                - f(self.write.load(Ordering::Relaxed)),
        );
    }
}

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
    // 顶面：u沿世界z，v沿x（已与wiki木板顶面纹理走向比对确认）
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

/// 元素rotation的变换矩阵（模型0-16空间）：绕origin旋转angle度，
/// rescale时垂直轴先放大1/cos(angle)（与游戏FaceBakery一致，十字平面借此铺满对角线）
fn element_rotation_matrix(rot: &ElementRotationObj) -> Mat4 {
    let origin = Vec3::from_slice(&rot.origin);
    let axis = match rot.axis.as_deref() {
        Some("x") => Vec3::X,
        Some("z") => Vec3::Z,
        _ => Vec3::Y,
    };
    let angle = rot.angle.unwrap_or(0.0).to_radians();
    let s = if rot.rescale.unwrap_or(false) && angle.cos() != 0.0 {
        1.0 / angle.cos()
    } else {
        1.0
    };
    let scale = match axis {
        v if v == Vec3::X => Vec3::new(1.0, s, s),
        v if v == Vec3::Y => Vec3::new(s, 1.0, s),
        _ => Vec3::new(s, s, 1.0),
    };
    Mat4::from_translation(origin)
        * Mat4::from_axis_angle(axis, angle)
        * Mat4::from_scale(scale)
        * Mat4::from_translation(-origin)
}

/// 绕y轴旋转一个模型空间点（gui旋转与完整方块[30,225,0]的yaw差）
fn rot_point_y(p: Point3, sin: f32, cos: f32) -> Point3 {
    Point3 {
        x: p.x * cos + p.z * sin,
        y: p.y,
        z: -p.x * sin + p.z * cos,
    }
}

/// 渲染缩放基准：Full按完整方块尺度（半砖等矮模型不放大铺满），
/// Own按模型自身包围盒铺满画布（两格高的整门/整床等多格合并模型）
#[derive(Clone, Copy)]
enum FitMode {
    Full,
    Own,
}

/// 渲染楼梯类多元素盒子模型：按gui旋转角刚体旋转所有元素面，
/// 投影后剔除背向观察者的面，按深度排序绘制（painter's algorithm）
/// 渲染单个模型（公开供手动测试调试单个方块用）
pub fn render_model_png(
    archive: &BaseArchive,
    tex_cache: &mut HashMap<String, Bitmap>,
    textures: &HashMap<String, TextureRefObj>,
    elements: &[BlockElementObj],
    rot_y: f32,
) -> Option<Vec<u8>> {
    let (texs, uvs, shades, corner_sets) =
        load_model_faces(archive, tex_cache, textures, elements, rot_y)?;
    render_faces_png(&texs, &uvs, &shades, &corner_sets, FitMode::Full)
}

/// 载入模型朝向观察者的面：按gui旋转角刚体旋转面法线与角点，
/// 剔除背向观察者的面，返回（贴图、uv、亮度、旋转后角点集）
fn load_model_faces(
    archive: &BaseArchive,
    tex_cache: &mut HashMap<String, Bitmap>,
    textures: &HashMap<String, TextureRefObj>,
    elements: &[BlockElementObj],
    rot_y: f32,
) -> Option<(Vec<Bitmap>, Vec<[Point; 4]>, Vec<u8>, Vec<[Point3; 4]>)> {
    // 模型gui旋转与完整方块[30,225,0]的yaw差，绕y轴刚体旋转
    let mat = *MAT;
    let angle = (rot_y - 225.0) * PI / 180.0;
    let (sin, cos) = angle.sin_cos();

    let mut texs: Vec<Bitmap> = Vec::new();
    let mut uvs: Vec<[Point; 4]> = Vec::new();
    let mut shades: Vec<u8> = Vec::new();
    let mut corner_sets: Vec<[Point3; 4]> = Vec::new();
    for element in elements {
        let faces = element.faces.as_ref()?;
        // 元素rotation（十字植物/营火原木/吊灯笼等）：角点与法线一并旋转
        let rot_m = element.rotation.as_ref().map(element_rotation_matrix);
        for (name, row, base_uv, normal) in FACE_DEFS {
            let Some(obj) = faces.get(name) else {
                continue;
            };
            // 元素rotation只取线性部分作用到法线，再叠加gui旋转；投影z<0说明面朝向观察者
            let normal = match rot_m {
                Some(m) => {
                    let r = m.transform_vector3(Vec3::new(normal.x, normal.y, normal.z));
                    Point3::new(r.x, r.y, r.z)
                }
                None => normal,
            };
            let n = rot_point_y(normal, sin, cos);
            if project(&mat, &n).z >= 0.0 {
                continue;
            }
            let ft = load_face(
                archive,
                tex_cache,
                textures,
                obj,
                &base_uv,
                name,
                &element.from,
                &element.to,
            )?;
            #[cfg(debug_assertions)]
            if std::env::var("MCML_UV_DEBUG").is_ok() {
                eprintln!("[uv] element {:?} face {name}: obj.uv={:?} -> ft.uv={:?}", element.from, obj.uv, ft.uv);
            }
            // 楼梯贴图均为静态，动画贴图兜底取第一帧
            texs.push(static_frame(&ft.tex));
            uvs.push(ft.uv);
            // 面亮度按旋转后的朝向：顶面全亮、南北向0.8、东西向0.6（与游戏一致）；
            // 元素shade:false（十字等）不着一向着色
            shades.push(if element.shade == Some(false) {
                255
            } else if n.y > 0.5 {
                255
            } else if n.y < -0.5 {
                128
            } else if n.z.abs() > n.x.abs() {
                204
            } else {
                153
            });
            // 角点经元素rotation（0-16模型空间）后再叠加gui旋转
            let corners = element_face_points(&element.from, &element.to, row);
            let corners = match rot_m {
                Some(m) => corners.map(|c| {
                    let p = m.transform_point3(Vec3::new((c.x + 1.0) * 8.0, (c.y + 1.0) * 8.0, (c.z + 1.0) * 8.0));
                    Point3::new(p.x / 8.0 - 1.0, p.y / 8.0 - 1.0, p.z / 8.0 - 1.0)
                }),
                None => corners,
            };
            corner_sets.push(corners.map(|c| rot_point_y(c, sin, cos)));
        }
    }
    if texs.is_empty() {
        return None;
    }
    Some((texs, uvs, shades, corner_sets))
}

/// 绘制已载入的面集：等轴测投影后按fit模式缩放（Full=完整方块尺度，
/// Own=模型自身包围盒铺满画布），按深度排序绘制（painter's algorithm）
fn render_faces_png(
    texs: &[Bitmap],
    uvs: &[[Point; 4]],
    shades: &[u8],
    corner_sets: &[[Point3; 4]],
    mode: FitMode,
) -> Option<Vec<u8>> {
    let mat = *MAT;

    // 先投影全部角点（屏幕坐标 + 深度），再按fit模式确定缩放
    let mut raw_sets: Vec<[Point; 4]> = Vec::with_capacity(corner_sets.len());
    let mut depth_sets: Vec<[f32; 4]> = Vec::with_capacity(corner_sets.len());
    for cs in corner_sets {
        let mut ps = [Point::default(); 4];
        let mut ds = [0.0f32; 4];
        for (i, c) in cs.iter().enumerate() {
            let res = project(&mat, c);
            ps[i] = Point { x: res.x, y: res.y };
            ds[i] = res.z;
        }
        raw_sets.push(ps);
        depth_sets.push(ds);
    }

    // 画布四周留边距
    const MARGIN: f32 = 2.0;
    let inner = BLOCK_SIZE as f32 - MARGIN * 2.0;
    let (fmin_x, fmin_y, fscale, fdx, fdy) = match mode {
        // 与完整方块同尺度，矮模型不放大铺满
        FitMode::Full => *FULL_FIT,
        // 多格合并模型按自身投影包围盒等比缩放居中
        FitMode::Own => {
            let mut min_x = f32::INFINITY;
            let mut min_y = f32::INFINITY;
            let mut max_x = f32::NEG_INFINITY;
            let mut max_y = f32::NEG_INFINITY;
            for ps in &raw_sets {
                for p in ps {
                    min_x = min_x.min(p.x);
                    min_y = min_y.min(p.y);
                    max_x = max_x.max(p.x);
                    max_y = max_y.max(p.y);
                }
            }
            let scale = inner / (max_x - min_x).max(max_y - min_y);
            let dx = (BLOCK_SIZE as f32 - (max_x - min_x) * scale) / 2.0;
            let dy = (BLOCK_SIZE as f32 - (max_y - min_y) * scale) / 2.0;
            (min_x, min_y, scale, dx, dy)
        }
    };
    let fit =
        |p: Point| Point::new((p.x - fmin_x) * fscale + fdx, (p.y - fmin_y) * fscale + fdy);
    let pos_sets: Vec<[Point; 4]> = raw_sets
        .iter()
        .map(|ps| ps.map(|p| fit(p)))
        .collect();

    // 组装面并按深度排序（远的先画）
    let mut geoms: Vec<(f32, FaceGeom)> = texs
        .iter()
        .enumerate()
        .map(|(i, tex)| {
            let depth = depth_sets[i].iter().sum::<f32>();
            (
                depth,
                FaceGeom {
                    tex,
                    pos: pos_sets[i],
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

/// 沿parent链解析模型（公开供手动测试调试单个方块用）：合并textures（子覆盖父），返回
/// （合并后的textures，第一个带elements祖先的全部元素，该祖先的模型相对名，
/// 是否完整方块，gui旋转y分量）
pub fn resolve_model(
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

        // 子模型的textures/display覆盖父模型（同名键子值优先，
        // 父模板的"#up"等占位引用不能挡住子模型的实际贴图）
        let mut merged = model.textures.unwrap_or_default();
        for (k, v) in textures {
            merged.insert(k, v);
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
///
/// `from`/`to`为该面所属元素盒子的边界，面未写uv时按其自动生成uv
fn load_face(
    archive: &BaseArchive,
    tex_cache: &mut HashMap<String, Bitmap>,
    textures: &HashMap<String, TextureRefObj>,
    face_obj: &BlockFaceObj,
    base_uv: &[Point; 4],
    face_name: &str,
    from: &[f32],
    to: &[f32],
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

    // 灰度贴图按群系色染色：白桦/云杉树叶固定色，其余树叶用植被色，
    // 草类（草顶/草侧overlay）用草色，取平原群系（与wiki物品图标一致）
    let tex = if face_obj.tintindex.is_some() {
        let tint = if path.contains("birch_leaves") {
            BIRCH_TINT
        } else if path.contains("spruce_leaves") {
            SPRUCE_TINT
        } else if path.contains("leaves") {
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
        uv: face_uv(base_uv, face_obj, face_name, from, to),
        anim: read_anim_meta(archive, &path),
    })
}

/// 完整方块：取第一个元素的六面定义，载入三个可见面渲染（动画贴图存APNG）。
/// 其余元素（is_full_cube已保证同为完整立方体）作为overlay面叠绘在基准面之后，
/// 如grass_block的侧面草皮overlay（与游戏分层绘制一致）
pub fn render_full_cube(
    archive: &BaseArchive,
    tex_cache: &mut HashMap<String, Bitmap>,
    textures: &HashMap<String, TextureRefObj>,
    elements: &[BlockElementObj],
) -> Option<Vec<u8>> {
    let face_objs = elements.first()?.faces.as_ref()?;
    // 完整方块元素边界恒为0-16，自动uv即全幅
    let (from, to) = (&elements[0].from, &elements[0].to);

    // 载入三个可见面（顶、北、东），任一面解析失败则跳过该方块
    let mut slots: [Option<FaceTexture>; 3] = [None, None, None];
    for (slot, face_name) in ["up", "north", "east"].iter().enumerate() {
        match face_objs.get(*face_name).and_then(|obj| {
            load_face(
                archive,
                tex_cache,
                textures,
                obj,
                &BASE_UV[slot],
                face_name,
                from,
                to,
            )
        }) {
            Some(face) => slots[slot] = Some(face),
            None => return None,
        }
    }
    let [Some(top), Some(north), Some(east)] = slots else {
        return None;
    };
    let faces = [top, north, east];

    // overlay元素只取可见三面，记录其slot用于叠绘
    let mut overlays: Vec<(usize, FaceTexture)> = Vec::new();
    for element in &elements[1..] {
        let Some(objs) = element.faces.as_ref() else {
            continue;
        };
        for (slot, face_name) in ["up", "north", "east"].iter().enumerate() {
            if let Some(obj) = objs.get(*face_name)
                && let Some(ft) = load_face(
                    archive,
                    tex_cache,
                    textures,
                    obj,
                    &BASE_UV[slot],
                    face_name,
                    &element.from,
                    &element.to,
                )
            {
                overlays.push((slot, ft));
            }
        }
    }

    // 动画贴图存成APNG（带overlay时统一走静态路径，取动画首帧）
    if overlays.is_empty() && faces.iter().any(|f| f.tex.height() > f.tex.width()) {
        return make_faces_apng(&faces);
    }
    // 先持有帧位图，再组装引用
    let mut draw: Vec<(Bitmap, usize, [Point; 4], u8)> = faces
        .iter()
        .enumerate()
        .map(|(slot, f)| (static_frame(&f.tex), slot, f.uv, FACE_SHADE[slot]))
        .collect();
    draw.extend(overlays.iter().map(|(slot, ft)| {
        (
            static_frame(&ft.tex),
            *slot,
            ft.uv,
            FACE_SHADE[*slot],
        )
    }));
    let geoms: Vec<FaceGeom> = draw
        .iter()
        .map(|(tex, slot, uv, shade)| FaceGeom {
            tex,
            pos: slot_pos(*slot),
            uv: *uv,
            shade: *shade,
        })
        .collect();
    render_geoms(&geoms)
        .and_then(|img| {
            #[allow(deprecated)]
            img.encode_to_data(EncodedImageFormat::PNG)
        })
        .map(|d| d.as_bytes().to_vec())
}

/// 多图方块种类：门（2格高）、床（2格宽）、活板门（单格多状态）
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MergedKind {
    Door,
    Bed,
    Trapdoor,
}

/// 多图方块合并组：变体模型共享一张基础ID图标，reps为渲染用的代表模型
/// （门/活板门只用reps[0]，床用reps[0]=foot、reps[1]=head）
struct MergedGroup<'a> {
    base: String,
    kind: MergedKind,
    reps: [Option<&'a str>; 2],
}

/// 按模型名识别多图方块的变体，返回（方块基础ID, 种类）
///
/// 门覆盖老版命名（oak_door_bottom/top）与新版8变体（oak_door_bottom_left等），
/// 合并后只出一张 minecraft_<wood>_door.png，不再按变体出图
fn classify_merged(id_rel: &str) -> Option<(String, MergedKind)> {
    // 模板模型（template_trapdoor_bottom等）不是方块状态，不参与合并
    if id_rel.starts_with("template_") {
        return None;
    }
    for suffix in [
        "_bottom_left_open",
        "_bottom_right_open",
        "_top_left_open",
        "_top_right_open",
        "_bottom_left",
        "_bottom_right",
        "_top_left",
        "_top_right",
        "_bottom",
        "_top",
    ] {
        if let Some(base) = id_rel.strip_suffix(suffix)
            && base.ends_with("_door")
        {
            return Some((base.to_string(), MergedKind::Door));
        }
    }
    for (suffix, kind) in [("_foot", MergedKind::Bed), ("_head", MergedKind::Bed)] {
        if let Some(base) = id_rel.strip_suffix(suffix)
            && base.ends_with("_bed")
        {
            return Some((base.to_string(), kind));
        }
    }
    for suffix in ["_bottom", "_top", "_open"] {
        if let Some(base) = id_rel.strip_suffix(suffix)
            && base.ends_with("_trapdoor")
        {
            return Some((base.to_string(), MergedKind::Trapdoor));
        }
    }
    None
}

/// 状态后缀：模型名带这些后缀且对应基础模型也存在时，视为同一方块的其他状态
/// （铁轨的_raised_ne、栅栏门的_open、熔炉的_on、吊灯的_hanging等），不单独出图。
/// 后缀可叠加（repeater_1tick_on），故逐层剥离；"fence_gate_wall"为墙上栅栏门的整体后缀
const STATE_SUFFIXES: &[&str] = &[
    "open", "closed", "on", "off", "lit", "unlit", "powered", "unpowered",
    "locked", "extended", "triggered", "hanging", "snow", "moist", "corner",
    "empty", "left", "right", "active", "inactive", "short", "tall", "sticky",
    "attached", "raised_ne", "raised_nw", "raised_se", "raised_sw",
    "fence_gate_wall", "1tick", "2tick", "3tick", "4tick",
];

/// 状态变体判定：模型名能逐层剥出状态后缀、且剥出的基础模型确实存在时返回true
///
/// 后缀必须在下划线边界后匹配（"button"不会被"on"误剥），且每层剥出的基础模型
/// 都要存在，避免误杀oak_stairs这类名字本身就是完整方块的模型
fn is_state_variant(id_rel: &str, all: &HashSet<&str>) -> bool {
    let mut cur = id_rel;
    loop {
        let mut stripped = false;
        for suffix in STATE_SUFFIXES {
            if let Some(rest) = cur.strip_suffix(suffix)
                && let Some(base) = rest.strip_suffix('_')
                && !base.is_empty()
                && all.contains(base)
            {
                cur = base;
                stripped = true;
                break;
            }
        }
        if !stripped {
            return cur != id_rel;
        }
    }
}

/// 部件后缀：blockstate拼装用的零件模型，不是独立方块
/// （栅栏的post/side、墙的post/side、玻璃板/栏杆的cap/noside、火焰/红石粉的贴片面等）
const PART_SUFFIXES: &[&str] = &["post", "side", "cap", "noside", "dot", "up", "floor"];

/// 通用模板模型的物品外观（fence/wall/button栅栏等没有vanilla方块对应，
/// 且基础模型不存在，其余规则剔除不到，这里显式删掉）
const GENERIC_TEMPLATES: &[&str] = &["fence_inventory", "wall_inventory", "custom_fence_inventory"];

/// 剥掉数字/alt/方向等修饰（fire_up_alt0 -> fire_up，bamboo_fence_side_east ->
/// bamboo_fence_side），便于识别零件本体
fn part_stem(id_rel: &str) -> &str {
    let mut cur = id_rel;
    while cur.chars().next_back().is_some_and(|c| c.is_ascii_digit()) {
        cur = &cur[..cur.len() - 1];
    }
    for deco in ["_alt", "_east", "_north", "_south", "_west", "_small", "_tall"] {
        if let Some(stem) = cur.strip_suffix(deco) {
            cur = stem;
            break;
        }
    }
    cur
}

/// 零件/模板模型判定：模板模型、blockstate拼装零件、木牌/挂牌的旋转状态都不是独立方块
///
/// 零件后缀须在下划线边界后匹配（part_stem剥修饰），避免误杀button这类同尾名模型
fn is_part_model(id_rel: &str) -> bool {
    // 模板模型与通用物品外观，根本没有对应方块
    if id_rel.starts_with("template_") || GENERIC_TEMPLATES.contains(&id_rel) {
        return true;
    }
    // 挂牌的挂墙状态（attached_rot_0..3）；rot_1..3只是rot_0的旋转
    if id_rel.contains("_attached_rot_") {
        return true;
    }
    if !id_rel.ends_with("_rot_0")
        && id_rel.rsplit_once("_rot_").is_some_and(|(_, tail)| {
            !tail.is_empty() && tail.bytes().all(|c| c.is_ascii_digit())
        })
    {
        return true;
    }
    let stem = part_stem(id_rel);
    PART_SUFFIXES.iter().any(|suffix| {
        stem.len() > suffix.len() + 1
            && stem.ends_with(suffix)
            && stem.as_bytes()[stem.len() - suffix.len() - 1] == b'_'
    })
}

/// 冗余物品外观判定：基础模型存在时，_inventory只是同一方块的物品展示复制品
/// （oak_button_inventory、piston_inventory、各shelf_inventory等）
fn is_redundant_inventory(id_rel: &str, all: &HashSet<&str>) -> bool {
    id_rel
        .strip_suffix("_inventory")
        .is_some_and(|base| all.contains(base))
}

/// 锚模型到方块本体ID的映射：fence/wall没有完整模型（inventory即物品外观），
/// 木牌/挂牌以rot_0为默认状态，图标注册到真实方块ID下
///
/// 返回None表示模型名即方块ID（含已是真实ID的墙牌X_wall_sign等）
fn anchor_base(id_rel: &str) -> Option<&str> {
    if let Some(base) = id_rel.strip_suffix("_rot_0") {
        return Some(base);
    }
    id_rel.strip_suffix("_inventory")
}

/// 合并组代表模型的优先级（越小越优先）：取关闭状态的下半块
fn merged_rep_priority(kind: MergedKind, id_rel: &str) -> u8 {
    match kind {
        MergedKind::Door if id_rel.ends_with("_bottom_left") => 0,
        MergedKind::Door if id_rel.ends_with("_bottom") => 1,
        MergedKind::Door => 2,
        MergedKind::Trapdoor if id_rel.ends_with("_bottom") => 0,
        MergedKind::Trapdoor if id_rel.ends_with("_top") => 1,
        MergedKind::Trapdoor => 2,
        // 床按foot/head各取一个，无优先级
        MergedKind::Bed => 0,
    }
}

/// 平移元素盒子（多格合并：上半门y+16、床头z+16）；
/// 面的uv均显式给出，不随from/to平移变化
fn shift_element(e: &BlockElementObj, dx: f32, dy: f32, dz: f32) -> BlockElementObj {
    let mut e = e.clone();
    e.from = e.from.iter().zip([dx, dy, dz]).map(|(v, d)| v + d).collect();
    e.to = e.to.iter().zip([dx, dy, dz]).map(|(v, d)| v + d).collect();
    e
}

/// 渲染多图方块的合并图标（门/床/活板门：变体模型拼为一张基础ID等距图）
fn render_merged_block(
    archive: &BaseArchive,
    tex_cache: &mut HashMap<String, Bitmap>,
    group: &MergedGroup,
) -> Option<Vec<u8>> {
    match group.kind {
        // 活板门：三个状态同贴图同几何（只是贴在方块不同位置），按下一半块等距渲染
        MergedKind::Trapdoor => {
            let (textures, elements, _, _, rot_y) = resolve_model(archive, group.reps[0]?)?;
            render_model_png(archive, tex_cache, &textures, &elements, rot_y)
        }
        // 门：bottom/top两半模型拼成2格高的整门，按自身包围盒等距渲染
        MergedKind::Door => {
            let rep = group.reps[0]?;
            let (textures, bottom, _, _, rot_y) = resolve_model(archive, rep)?;
            // 上半模型与下半同族："..._bottom_left" -> "..._top_left"
            let (_, top, ..) = resolve_model(archive, rep.replacen("_bottom", "_top", 1).as_str())?;
            let mut elements = bottom;
            elements.extend(top.iter().map(|e| shift_element(e, 0.0, 16.0, 0.0)));
            let (texs, uvs, shades, corner_sets) =
                load_model_faces(archive, tex_cache, &textures, &elements, rot_y)?;
            render_faces_png(&texs, &uvs, &shades, &corner_sets, FitMode::Own)
        }
        // 床：foot/head两半模型沿床头方向拼成2格长的整床（foot贴图与head贴图不同，
        // 两半各自带textures解析），床头在后，按自身包围盒等距渲染
        MergedKind::Bed => {
            let (foot_tex, foot_el, _, _, rot_y) = resolve_model(archive, group.reps[0]?)?;
            let (head_tex, head_el, ..) = resolve_model(archive, group.reps[1]?)?;
            let head_el: Vec<BlockElementObj> = head_el
                .iter()
                .map(|e| shift_element(e, 0.0, 0.0, 16.0))
                .collect();
            let (mut texs, mut uvs, mut shades, mut corner_sets) =
                load_model_faces(archive, tex_cache, &foot_tex, &foot_el, rot_y)?;
            let (t2, u2, s2, c2) =
                load_model_faces(archive, tex_cache, &head_tex, &head_el, rot_y)?;
            texs.extend(t2);
            uvs.extend(u2);
            shades.extend(s2);
            corner_sets.extend(c2);
            render_faces_png(&texs, &uvs, &shades, &corner_sets, FitMode::Own)
        }
    }
}

/// 从客户端jar提取所有完整方块贴图并渲染保存
///
/// 下载流程（版本清单 → 客户端jar）见lib的load_blocks，这里只做解包渲染。
/// 支持多贴图方块（cube_bottom_top/cube_column/orientable等）：
/// 沿parent链合并textures，取第一个带elements祖先的面定义，
/// 只画当前视角可见的三个面（顶、北、东）
///
/// 渲染按CPU核数并发（rayon）：各工作线程经thread_local持有一份只读jar句柄
/// 与跨方块复用的贴图缓存，完成后单线程汇总写入方块状态
///
/// `gui`可选：按已处理的候选模型数上报进度（约每1%一次）
pub fn render_blocks(archive: &BaseArchive, gui: ProgressGui) -> CoreResult<()> {
    let render_start = std::time::Instant::now();
    let dir = crate::get_block_dir().ok_or(ErrorType::DownloadFileFail)?;

    // 提取语言文件并构建 ID->语言键 映射
    let names = extract_langs(archive);

    // 预收集候选模型条目（entries是不可变切片，可跨线程共享）
    let entries: Vec<&str> = archive
        .entries()
        .iter()
        .map(|entry| entry.name.as_str())
        .filter(|name| {
            name.starts_with("assets/minecraft/models/block/") && name.ends_with(".json")
        })
        .collect();

    // 多图方块分组：门/床/活板门的变体模型合并为一张基础ID图标，其余走常规渲染；
    // 模板/blockstate零件不渲染；状态变体（铁轨_raised_ne、栅栏门_open、熔炉_on等）
    // 已有基础模型时不单独出图；fence/wall的inventory与木牌rot_0重命名到真实方块ID
    let model_rels: HashSet<&str> = entries
        .iter()
        .map(|name| {
            name.trim_start_matches("assets/minecraft/models/block/")
                .trim_end_matches(".json")
        })
        .collect();
    let mut normal: Vec<&str> = Vec::new();
    let mut merged: Vec<MergedGroup> = Vec::new();
    let mut merged_rels: Vec<Vec<&str>> = Vec::new();
    let mut group_index: HashMap<String, usize> = HashMap::new();
    for &name in &entries {
        let rel = name.trim_start_matches("assets/minecraft/models/").trim_end_matches(".json");
        let id_rel = rel.trim_start_matches("block/");
        if is_part_model(id_rel) {
            continue;
        }
        let Some((base, kind)) = classify_merged(id_rel) else {
            if is_state_variant(id_rel, &model_rels)
                || is_redundant_inventory(id_rel, &model_rels)
            {
                continue;
            }
            normal.push(name);
            continue;
        };
        let idx = *group_index.entry(base.clone()).or_insert_with(|| {
            merged.push(MergedGroup {
                base,
                kind,
                reps: [None, None],
            });
            merged_rels.push(Vec::new());
            merged.len() - 1
        });
        merged_rels[idx].push(rel);
    }
    // 选代表模型：床取foot与head各一，门/活板门取最接近关闭状态的变体
    for (group, rels) in merged.iter_mut().zip(&merged_rels) {
        match group.kind {
            MergedKind::Bed => {
                group.reps[0] = rels.iter().find(|rel| rel.ends_with("_foot")).copied();
                group.reps[1] = rels.iter().find(|rel| rel.ends_with("_head")).copied();
            }
            _ => {
                group.reps[0] = rels
                    .iter()
                    .min_by_key(|rel| merged_rep_priority(group.kind, rel))
                    .copied();
            }
        }
    }

    enum Job<'a> {
        Normal(&'a str),
        Merged(MergedGroup<'a>),
    }
    let mut jobs: Vec<Job> = normal.into_iter().map(Job::Normal).collect();
    jobs.extend(merged.into_iter().map(Job::Merged));

    let total = jobs.len();
    let done = AtomicUsize::new(0);
    // 每1%左右上报一次，避免高频回调刷爆GUI
    let step = (total / 100).max(1);

    // 每个工作线程的独立资源，随线程存活跨job复用：
    // - jar只读句柄：共享archive的read在锁内只有一个句柄，会把JSON/贴图读取全部串行化；
    //   但map_init的init在工作被偷取时会按job重新执行（实测2000+次），不能在那里打开，
    //   故用thread_local保证每线程只开一次。只读打开避开写模式打开的杀软扫描开销，
    //   打开失败时置Some(None)记为已尝试，后续回退共享实例
    // - 贴图解码缓存：同一贴图常被多个方块/多个面共用，按线程缓存避免反复解码
    thread_local! {
        static LOCAL_ARCHIVE: RefCell<Option<Option<BaseArchive>>> = const { RefCell::new(None) };
        static TEX_CACHE: RefCell<HashMap<String, Bitmap>> = RefCell::new(HashMap::new());
    }

    // 并发渲染
    let rendered: Vec<Option<(String, String, Option<String>)>> = jobs
        .par_iter()
        .map(|job| {
            LOCAL_ARCHIVE.with(|local_archive| {
                TEX_CACHE.with(|tex_cache| {
                    let mut tex_cache = tex_cache.borrow_mut();
                    let mut local_archive = local_archive.borrow_mut();
                    let archive = local_archive
                        .get_or_insert_with(|| BaseArchive::open_readonly(archive.path()).ok())
                        .as_ref()
                        .unwrap_or(archive);
                    // 单个候选模型的处理（进度计数对跳过的条目也要生效）
                    let item_start = std::time::Instant::now();
                    let mut render = || -> Option<(String, String, Option<String>)> {
                    match job {
                        Job::Normal(name) => {
                    // 模型相对名与方块ID："block/stone" -> "minecraft:stone"
                    // 锚模型（fence/wall的inventory、木牌rot_0）注册到真实方块ID
                    let rel = name
                        .trim_start_matches("assets/minecraft/models/")
                        .trim_end_matches(".json");
                    let id_rel = name
                        .trim_start_matches("assets/minecraft/models/block/")
                        .trim_end_matches(".json");
                    let id_rel = anchor_base(id_rel).unwrap_or(id_rel);
                    let id = format!("minecraft:{id_rel}");

                    // 完整方块/楼梯/半砖/首元素完整（信标等内嵌模型）才渲染，玻璃板等跳过
                    let t = std::time::Instant::now();
                    let resolved = resolve_model(archive, rel);
                    PROFILE.resolve.fetch_add(
                        t.elapsed().as_nanos() as u64,
                        Ordering::Relaxed,
                    );
                    let Some((textures, elements, _template, full, rot_y)) = resolved else {
                        return None;
                    };
                    // gui旋转与完整方块[30,225,0]的yaw差为0时走完整方块快捷路径
                    let rot_steps = ((225.0 - rot_y) / 90.0).rem_euclid(4.0) as usize;

                    let t = std::time::Instant::now();
                    let data = if !full || rot_steps != 0 {
                        // 多元素/带旋转的模型：按gui旋转角渲染全部可见面
                        render_model_png(archive, &mut tex_cache, &textures, &elements, rot_y)
                    } else {
                        render_full_cube(archive, &mut tex_cache, &textures, &elements)
                    };
                    PROFILE.draw.fetch_add(
                        t.elapsed().as_nanos() as u64,
                        Ordering::Relaxed,
                    );
                    let data = data?;

                    let out_name = format!("minecraft_{id_rel}.png");
                    let t = std::time::Instant::now();
                    path_helper::write_bytes(&dir.join(&out_name), &data).ok()?;
                    PROFILE.write.fetch_add(
                        t.elapsed().as_nanos() as u64,
                        Ordering::Relaxed,
                    );
                    let lang_key = names.get(&id).cloned();
                    Some((id, out_name, lang_key))
                        }
                        Job::Merged(group) => {
                        // 合并图标：按基础ID出一张图并注册到方块表（带语言键）
                        let t = std::time::Instant::now();
                        let data = render_merged_block(archive, &mut tex_cache, group)?;
                        PROFILE.draw.fetch_add(
                            t.elapsed().as_nanos() as u64,
                            Ordering::Relaxed,
                        );
                        let out_name = format!("minecraft_{}.png", group.base);
                        let t = std::time::Instant::now();
                        path_helper::write_bytes(&dir.join(&out_name), &data).ok()?;
                        PROFILE.write.fetch_add(
                            t.elapsed().as_nanos() as u64,
                            Ordering::Relaxed,
                        );
                        let id = format!("minecraft:{}", group.base);
                        let lang_key = names.get(&id).cloned();
                        Some((id, out_name, lang_key))
                        }
                    }
                };
                let out = render();

                // 定位异常慢的单方块（如超大动画贴图）
                if PROFILE.enabled {
                    let secs = item_start.elapsed().as_secs_f64();
                    let job_name = match job {
                        Job::Normal(name) => name,
                        Job::Merged(group) => group.base.as_str(),
                    };
                    if secs > 1.0 {
                        println!("[性能] 慢方块 {job_name}：{secs:.1}s");
                    }
                }

                // 候选模型处理完一个计一个（含跳过的），每跨过step阈值上报一次
                let now = done.fetch_add(1, Ordering::Relaxed) + 1;
                if now % step == 0
                    && let Some(gui) = &gui
                {
                    gui.set_progress_now(now, Some(total));
                }

                out
                    })
            })
        })
        .collect();

    // 分阶段耗时统计（MCML_RENDER_PROFILE=1 时打印）
    PROFILE.print(render_start.elapsed());

    // 完成时补一次100%，避免进度停在最后一个step前
    if let Some(gui) = &gui {
        gui.set_progress_now(total, Some(total));
    }

    // 单线程汇总写入方块状态
    let mut blocks = crate::blocks_write();
    for (id, out_name, lang_key) in rendered.into_iter().flatten() {
        blocks.tex.insert(id.clone(), out_name);
        if let Some(lang_key) = lang_key {
            blocks.name.insert(id, lang_key);
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
#[derive(Clone)]
pub struct AnimMeta {
    /// 帧间隔（单位：游戏刻，1刻 = 50ms）
    pub frametime: u32,
    /// 帧之间是否平滑插值
    pub interpolate: bool,
    /// mcmeta的frames自定义帧序列（缺省按0..n顺序展开）
    pub frames: Option<Vec<AnimFrame>>,
}

/// 帧序列的一项（帧号 + 该帧持续刻数，缺省时长为frametime）
#[derive(Clone, Copy)]
pub struct AnimFrame {
    pub index: u32,
    pub time: u32,
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
    /// 自定义帧序列：数字项为帧号，对象项为{index, time}
    frames: Option<Vec<serde_json::Value>>,
}

/// 读取动画贴图配置（帧间隔、是否插值、自定义帧序列）
pub fn read_anim_meta(archive: &BaseArchive, tex_path: &str) -> AnimMeta {
    let fallback = AnimMeta {
        frametime: 1,
        interpolate: false,
        frames: None,
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
    let frametime = animation.frametime.filter(|t| *t > 0).unwrap_or(1);
    let frames = animation.frames.map(|list| {
        list.into_iter()
            .filter_map(|f| match f {
                serde_json::Value::Number(n) => n.as_u64().map(|i| AnimFrame {
                    index: i as u32,
                    time: frametime,
                }),
                serde_json::Value::Object(o) => {
                    let index = o.get("index").and_then(|v| v.as_u64())? as u32;
                    let time = o
                        .get("time")
                        .and_then(|v| v.as_u64())
                        .filter(|t| *t > 0)
                        .unwrap_or(frametime as u64) as u32;
                    Some(AnimFrame { index, time })
                }
                _ => None,
            })
            .collect::<Vec<_>>()
    });
    AnimMeta {
        frametime,
        interpolate: animation.interpolate.unwrap_or(false),
        frames: frames.filter(|f| !f.is_empty()),
    }
}

/// 当前已渲染的游戏版本
pub(crate) fn mcml_tex_draw_id() -> String {
    crate::blocks_read().id.clone()
}

#[cfg(test)]
mod merged_tests {
    use super::*;

    /// 多图方块变体应归类到基础ID，普通方块与模板模型不归类
    #[test]
    fn classify_merged_kinds() {
        // 门：老版与新版命名
        assert_eq!(
            classify_merged("acacia_door_bottom_left"),
            Some(("acacia_door".to_string(), MergedKind::Door))
        );
        assert_eq!(
            classify_merged("oak_door_top"),
            Some(("oak_door".to_string(), MergedKind::Door))
        );
        // 床：foot/head两半
        assert_eq!(
            classify_merged("white_bed_foot"),
            Some(("white_bed".to_string(), MergedKind::Bed))
        );
        // 活板门：三种状态
        assert_eq!(
            classify_merged("acacia_trapdoor_open"),
            Some(("acacia_trapdoor".to_string(), MergedKind::Trapdoor))
        );
        // 普通方块与模板模型不归类
        assert_eq!(classify_merged("stone"), None);
        assert_eq!(classify_merged("sandstone_bottom"), None);
        assert_eq!(classify_merged("template_bed_foot"), None);
        assert_eq!(classify_merged("door_bottom_left"), None);
    }

    /// 模型集合：基础模型与其状态变体（对齐26.2 jar中的真实模型名）
    fn variant_names() -> HashSet<&'static str> {
        HashSet::from([
            "rail",
            "rail_corner",
            "rail_raised_ne",
            "rail_raised_sw",
            "activator_rail",
            "activator_rail_on",
            "activator_rail_on_raised_ne",
            "oak_fence_gate",
            "oak_fence_gate_wall",
            "oak_fence_gate_wall_open",
            "furnace",
            "furnace_on",
            "lantern",
            "lantern_hanging",
            "repeater",
            "repeater_1tick",
            "repeater_1tick_on",
            "repeater_1tick_on_locked",
            "grass_block",
            "grass_block_snow",
            "farmland",
            "farmland_moist",
            "sculk_sensor",
            "sculk_sensor_active",
            // 与状态后缀同名结尾但非下划线边界/基础不存在的对照项
            "button",
            "sandstone_bottom",
            "oak_stairs",
            "red_sandstone",
            "red_sandstone_wall",
        ])
    }

    /// 状态变体应识别为true：铁轨的抬升/转角、栅栏门开启、熔炉点燃等，可逐层叠加
    #[test]
    fn state_variants_detected() {
        let all = variant_names();
        assert!(is_state_variant("rail_raised_ne", &all));
        assert!(is_state_variant("rail_corner", &all));
        assert!(is_state_variant("activator_rail_on_raised_ne", &all));
        assert!(is_state_variant("oak_fence_gate_open", &all));
        assert!(is_state_variant("oak_fence_gate_wall_open", &all));
        assert!(is_state_variant("furnace_on", &all));
        assert!(is_state_variant("lantern_hanging", &all));
        assert!(is_state_variant("repeater_1tick_on", &all));
        assert!(is_state_variant("repeater_1tick_on_locked", &all));
        assert!(is_state_variant("grass_block_snow", &all));
        assert!(is_state_variant("farmland_moist", &all));
        assert!(is_state_variant("sculk_sensor_active", &all));
    }

    /// 基础模型、无下划线边界的同尾名、基础不存在的变体都不是状态变体
    #[test]
    fn state_variants_reject_normal() {
        let all = variant_names();
        assert!(!is_state_variant("rail", &all));
        assert!(!is_state_variant("furnace", &all));
        // "button"不能被"on"误剥（非下划线边界）
        assert!(!is_state_variant("button", &all));
        // 基础模型不存在时保留：oak_stairs无oak模型，red_sandstone_wall无red_sandstone后缀边界
        assert!(!is_state_variant("oak_stairs", &all));
        assert!(!is_state_variant("red_sandstone_wall", &all));
        assert!(!is_state_variant("sandstone_bottom", &all));
    }

    /// 模板与blockstate零件应判定为true：栅栏/墙的post/side、玻璃板/栏杆零件、
    /// 火焰/红石粉贴片面（带数字/alt/方向修饰）、挂牌挂墙状态、rot_1..3旋转状态
    #[test]
    fn part_models_detected() {
        assert!(is_part_model("template_fence_gate"));
        assert!(is_part_model("template_four_turtle_eggs"));
        assert!(is_part_model("fence_inventory"));
        assert!(is_part_model("wall_inventory"));
        assert!(is_part_model("custom_fence_inventory"));
        assert!(is_part_model("acacia_fence_post"));
        assert!(is_part_model("acacia_fence_side"));
        assert!(is_part_model("bamboo_fence_side_east"));
        assert!(is_part_model("andesite_wall_post"));
        assert!(is_part_model("andesite_wall_side"));
        assert!(is_part_model("andesite_wall_side_tall"));
        assert!(is_part_model("pale_moss_carpet_side_small"));
        assert!(is_part_model("glass_pane_post"));
        assert!(is_part_model("glass_pane_noside"));
        assert!(is_part_model("iron_bars_cap"));
        assert!(is_part_model("hopper_side"));
        assert!(is_part_model("chorus_plant_side"));
        assert!(is_part_model("mossy_carpet_side"));
        assert!(is_part_model("fire_side0"));
        assert!(is_part_model("fire_up_alt1"));
        assert!(is_part_model("soul_fire_floor1"));
        assert!(is_part_model("redstone_dust_dot"));
        assert!(is_part_model("redstone_dust_side_alt0"));
        assert!(is_part_model("redstone_dust_up"));
        assert!(is_part_model("oak_hanging_sign_attached_rot_0"));
        assert!(is_part_model("oak_sign_rot_1"));
    }

    /// 锚模型与正常方块不应被零件规则误杀
    #[test]
    fn part_models_reject_normal() {
        // 木牌rot_0是保留的默认状态，inventory是fence/wall的锚
        assert!(!is_part_model("oak_sign_rot_0"));
        assert!(!is_part_model("oak_hanging_sign_rot_0"));
        assert!(!is_part_model("oak_fence_inventory"));
        assert!(!is_part_model("andesite_wall_inventory"));
        // 非下划线边界/正常方块
        assert!(!is_part_model("button"));
        assert!(!is_part_model("grass_block"));
        assert!(!is_part_model("furnace"));
        assert!(!is_part_model("sea_pickle"));
        assert!(!is_part_model("oak_wall_sign"));
    }

    /// 冗余物品外观：基础模型存在时_inventory不单独出图
    #[test]
    fn redundant_inventory_detected() {
        let all: HashSet<&str> = HashSet::from(["oak_button", "piston", "acacia_shelf"]);
        assert!(is_redundant_inventory("oak_button_inventory", &all));
        assert!(is_redundant_inventory("piston_inventory", &all));
        assert!(is_redundant_inventory("acacia_shelf_inventory", &all));
        // 基础模型不存在的是锚，保留
        assert!(!is_redundant_inventory("oak_fence_inventory", &all));
        assert!(!is_redundant_inventory("andesite_wall_inventory", &all));
    }

    /// 锚模型应映射到真实方块ID，普通模型与墙牌（已是真实ID）不变
    #[test]
    fn anchor_bases() {
        assert_eq!(anchor_base("oak_fence_inventory"), Some("oak_fence"));
        assert_eq!(anchor_base("andesite_wall_inventory"), Some("andesite_wall"));
        assert_eq!(anchor_base("oak_sign_rot_0"), Some("oak_sign"));
        assert_eq!(
            anchor_base("oak_hanging_sign_rot_0"),
            Some("oak_hanging_sign")
        );
        assert_eq!(anchor_base("oak_wall_sign"), None);
        assert_eq!(anchor_base("stone"), None);
        assert_eq!(anchor_base("oak_sign_rot_1"), None);
    }
}
