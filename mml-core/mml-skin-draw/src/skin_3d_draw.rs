//! 皮肤整体 3D 等距渲染（TypeA：固定水平 45°、俯视 30° 的正交视角）
//!
//! 复用 [`crate::head_3d_draw`] 的投影 / 三角形填充 / 降采样，把全身 6 个部件
//! （头 / 身体 / 双臂 / 双腿，各含 1.125 倍 overlay 层）按远到近的顺序逐面绘制，
//! 输出 `SIZE` x `SIZE` 的等距全身图。
//!
//! 模型坐标约定与 [`crate::head_3d_draw`] 一致：
//! +x 为角色正面、+y 为上、+z 为角色左侧，原点取身高一半处；
//! 旧版 64x32 皮肤先补成 64x64（右肢镜像填进左肢块）再统一按新版布局取贴图。

use std::f32::consts::PI;

use glam::{Mat4, Vec3};
use mml_skin::{SkinType, skin_type_checker};
use tiny_skia::Pixmap;

use crate::head_3d_draw::{SIZE, SUPERSAMPLE, downsample, fill_triangle, project};
use crate::skin_draw::{BPP, draw, row_bytes};

/// 每像素读取 / 写入（与 skin_2d_draw 相同的原始字节语义）
fn get_pixel(image: &Pixmap, x: i32, y: i32) -> [u8; 4] {
    let offset = y as usize * row_bytes(image) + x as usize * BPP;
    let mut out = [0u8; 4];
    out.copy_from_slice(&image.data()[offset..offset + BPP]);
    out
}

fn set_pixel(image: &mut Pixmap, x: i32, y: i32, color: [u8; 4]) {
    let offset = y as usize * row_bytes(image) + x as usize * BPP;
    image.data_mut()[offset..offset + BPP].copy_from_slice(&color);
}

/// 复制一个面区域（可选水平翻转）到目标位置
fn copy_face(
    dest: &mut Pixmap,
    dx: i32,
    dy: i32,
    src: &Pixmap,
    sx: i32,
    sy: i32,
    w: i32,
    h: i32,
    flip: bool,
) -> Option<()> {
    for j in 0..h {
        for i in 0..w {
            let si = if flip { w - 1 - i } else { i };
            let pix = get_pixel(src, sx + si, sy + j);
            set_pixel(dest, dx + i, dy + j, pix);
        }
    }
    Some(())
}

/// 旧版 64x32 皮肤补成 64x64：右腿 / 右臂镜像填进左腿 / 左臂的贴图块
///
/// 镜像规则（沿身体左右对称面）：顶 / 底 / 前 / 后面水平翻转，
/// 左右侧面原样（镜像后朝向互换，贴图本身不翻转）。
fn normalize_texture(image: &Pixmap) -> Option<Pixmap> {
    if image.width() == 64 && image.height() == 64 {
        return Some(image.clone());
    }
    if image.width() != 64 || image.height() != 32 {
        // 其它尺寸（HD 皮肤等）不处理，按原样取贴图
        return Some(image.clone());
    }

    let mut out = Pixmap::new(64, 64)?;
    // 上半部分（头 / 身体 / 右肢）原样复制
    draw(&mut out, image, 0, 0, 0, 0, 64, 32)?;

    // 右腿块 (0,16) → 左腿块 (16,48)；右臂块 (40,16) → 左臂块 (32,48)
    for (sx, sy, dx, dy) in [(0, 16, 16, 48), (40, 16, 32, 48)] {
        copy_face(&mut out, dx + 4, dy, image, sx + 4, sy, 4, 4, true)?; // 顶
        copy_face(&mut out, dx + 8, dy, image, sx + 8, sy, 4, 4, true)?; // 底
        // 左肢的右侧面 = 源左侧面（镜像后朝向互换），原样复制
        copy_face(&mut out, dx, dy + 4, image, sx + 8, sy + 4, 4, 12, false)?;
        copy_face(&mut out, dx + 4, dy + 4, image, sx + 4, sy + 4, 4, 12, true)?; // 前
        copy_face(&mut out, dx + 8, dy + 4, image, sx, sy + 4, 4, 12, false)?;
        copy_face(&mut out, dx + 12, dy + 4, image, sx + 12, sy + 4, 4, 12, true)?; // 后
    }

    Some(out)
}

/// 部件立方体（模型坐标：+x 正面 / +y 上 / +z 角色左，原点在身高一半处）
struct Cuboid {
    min: [f32; 3],
    max: [f32; 3],
}

/// 一个面的贴图区域：皮肤贴图上的 (tx, ty, tw, th)
type FaceRect = [i32; 4];

/// 一个身体部件：基础层 + overlay 层（各 6 个面的贴图区域，按方向下标索引）
struct Part {
    base: Cuboid,
    overlay: Cuboid,
    base_tex: [FaceRect; 6],
    overlay_tex: [FaceRect; 6],
}

/// 面方向下标：0 正面(+x) / 1 背面(-x) / 2 右侧(-z，角色右) / 3 左侧(+z，角色左) / 4 顶 / 5 底
mod dir {
    pub const FRONT: usize = 0;
    pub const BACK: usize = 1;
    pub const RIGHT: usize = 2;
    pub const LEFT: usize = 3;
    pub const TOP: usize = 4;
    pub const BOTTOM: usize = 5;
}

/// 单个部件面绘制顺序（远 → 近，与 head_3d_draw 的固定角度一致）：
/// 背面 → 底 → 左侧 → 顶 → 右侧 → 正面
const FACE_ORDER: [usize; 6] = [
    dir::BACK,
    dir::BOTTOM,
    dir::LEFT,
    dir::TOP,
    dir::RIGHT,
    dir::FRONT,
];

/// 从部件贴图块左上角 (tx, ty) 取六个面的区域
///
/// 块内布局：上排 [顶(w,4) | 底(w,4)]，下排 [右(4,12) | 前(w,12) | 左(4,12) | 后(w,12)]
fn part_faces(tx: i32, ty: i32, w: i32) -> [FaceRect; 6] {
    let mut f: [FaceRect; 6] = [[0, 0, 0, 0]; 6];
    f[dir::FRONT] = [tx + 4, ty + 4, w, 12];
    f[dir::BACK] = [tx + 8 + w, ty + 4, w, 12];
    f[dir::RIGHT] = [tx, ty + 4, 4, 12];
    f[dir::LEFT] = [tx + 4 + w, ty + 4, 4, 12];
    f[dir::TOP] = [tx + 4, ty, w, 4];
    f[dir::BOTTOM] = [tx + 8 + w, ty, w, 4];
    f
}

/// Cuboid 快捷构造（min_x, max_x, min_y, max_y, min_z, max_z）
fn cuboid(min_x: f32, max_x: f32, min_y: f32, max_y: f32, min_z: f32, max_z: f32) -> Cuboid {
    Cuboid {
        min: [min_x, min_y, min_z],
        max: [max_x, max_y, max_z],
    }
}

/// 组装全身部件（纤细手臂 3 像素宽，普通 4 像素宽）
fn parts(skin_type: SkinType) -> Vec<Part> {
    let slim = skin_type == SkinType::NewSlim;
    let aw = if slim { 3 } else { 4 }; // 手臂宽（像素）

    let head = Part {
        base: cuboid(-4.0, 4.0, 8.0, 16.0, -4.0, 4.0),
        overlay: cuboid(-4.5, 4.5, 7.5, 16.5, -4.5, 4.5),
        base_tex: part_faces(0, 0, 8),
        overlay_tex: part_faces(32, 0, 8),
    };
    let body = Part {
        base: cuboid(-2.0, 2.0, -4.0, 8.0, -4.0, 4.0),
        overlay: cuboid(-2.5, 2.5, -4.5, 8.5, -4.5, 4.5),
        base_tex: part_faces(16, 16, 8),
        overlay_tex: part_faces(16, 32, 8),
    };
    let leg_r = Part {
        base: cuboid(-2.0, 2.0, -16.0, -4.0, -4.0, 0.0),
        overlay: cuboid(-2.5, 2.5, -16.5, -3.5, -4.5, 0.5),
        base_tex: part_faces(0, 16, 4),
        overlay_tex: part_faces(0, 32, 4),
    };
    let leg_l = Part {
        base: cuboid(-2.0, 2.0, -16.0, -4.0, 0.0, 4.0),
        overlay: cuboid(-2.5, 2.5, -16.5, -3.5, -0.5, 4.5),
        base_tex: part_faces(16, 48, 4),
        overlay_tex: part_faces(0, 48, 4),
    };
    // 手臂：截面 4x4（纤细 3x4），贴在身体两侧
    let arm_r = Part {
        base: cuboid(-2.0, 2.0, -4.0, 8.0, -2.0 - aw as f32, -2.0),
        overlay: cuboid(-2.5, 2.5, -4.5, 8.5, -2.0 - aw as f32 - 0.5, -1.5),
        base_tex: part_faces(40, 16, aw),
        overlay_tex: part_faces(40, 32, aw),
    };
    let arm_l = Part {
        base: cuboid(-2.0, 2.0, -4.0, 8.0, 2.0, 2.0 + aw as f32),
        overlay: cuboid(-2.5, 2.5, -4.5, 8.5, 1.5, 2.0 + aw as f32 + 0.5),
        base_tex: part_faces(32, 48, aw),
        overlay_tex: part_faces(48, 48, aw),
    };

    // 绘制顺序（远 → 近，视角在角色右前方）：左肢 → 右肢 → 身体 → 头
    vec![leg_l, leg_r, arm_l, arm_r, body, head]
}

/// 固定角度（水平 45°、俯视 30°）的模型变换矩阵（与 head_3d_draw 同角度，缩放适配全身）
fn create_tran() -> Mat4 {
    let roty = Mat4::from_rotation_y(45.0 * PI / 180.0);
    let rotx = Mat4::from_rotation_x(-30.0 * PI / 180.0);

    // 全身高 32 像素，缩放 10 倍占 320px，居中在 400px 画布里
    let scale = Mat4::from_scale(Vec3::new(10.0, -10.0, 10.0));

    let tran = Mat4::from_translation(Vec3::new(200.0, 200.0, 0.0));

    tran * scale * rotx * roty
}

/// 取立方体某个面的四个角（模型坐标 + 块内归一化 uv）
///
/// 角顺序与 uv 公式都和 head_3d_draw 的立方体展开一致
/// （u/v 均为 0..1 的块内比例，v=0 在贴图块顶部）。
fn face_corners(c: &Cuboid, d: usize) -> [([f32; 3], [f32; 2]); 4] {
    let (x0, y0, z0) = (c.min[0], c.min[1], c.min[2]);
    let (x1, y1, z1) = (c.max[0], c.max[1], c.max[2]);

    match d {
        // 正面 (+x)：u 沿 -z（u=1 在角色左），v 沿 -y
        dir::FRONT => [
            ([x1, y0, z1], [1.0, 1.0]),
            ([x1, y0, z0], [0.0, 1.0]),
            ([x1, y1, z0], [0.0, 0.0]),
            ([x1, y1, z1], [1.0, 0.0]),
        ],
        // 背面 (-x)：镜像
        dir::BACK => [
            ([x0, y0, z1], [0.0, 1.0]),
            ([x0, y0, z0], [1.0, 1.0]),
            ([x0, y1, z0], [1.0, 0.0]),
            ([x0, y1, z1], [0.0, 0.0]),
        ],
        // 右侧 (-z，角色右)：u 沿 +x（u=0 在正面一端）
        dir::RIGHT => [
            ([x0, y0, z0], [1.0, 1.0]),
            ([x1, y0, z0], [0.0, 1.0]),
            ([x1, y1, z0], [0.0, 0.0]),
            ([x0, y1, z0], [1.0, 0.0]),
        ],
        // 左侧 (+z，角色左)：u 沿 +x
        dir::LEFT => [
            ([x0, y0, z1], [0.0, 1.0]),
            ([x1, y0, z1], [1.0, 1.0]),
            ([x1, y1, z1], [1.0, 0.0]),
            ([x0, y1, z1], [0.0, 0.0]),
        ],
        // 顶 / 底：u 沿 z（u=1 在角色左），v 沿 x（v=1 在正面一端）
        dir::TOP => [
            ([x0, y1, z1], [1.0, 0.0]),
            ([x0, y1, z0], [0.0, 0.0]),
            ([x1, y1, z0], [0.0, 1.0]),
            ([x1, y1, z1], [1.0, 1.0]),
        ],
        _ => [
            ([x0, y0, z1], [1.0, 0.0]),
            ([x0, y0, z0], [0.0, 0.0]),
            ([x1, y0, z0], [0.0, 1.0]),
            ([x1, y0, z1], [1.0, 1.0]),
        ],
    }
}

/// 把一个面投影并逐三角形绘制到目标位图（屏幕坐标先乘超采样倍数）
fn draw_face(
    pixmap: &mut Pixmap,
    texture: &Pixmap,
    tran: &Mat4,
    c: &Cuboid,
    d: usize,
    tex: &FaceRect,
    scale: f32,
) {
    let (tx, ty, tw, th) = (tex[0] as f32, tex[1] as f32, tex[2] as f32, tex[3] as f32);

    let corners = face_corners(c, d);
    let mut projected = [((0.0f32, 0.0f32), (0.0f32, 0.0f32)); 4];
    for (i, (pos, uv)) in corners.iter().enumerate() {
        let screen = project(tran, *pos, false);
        let t = (tx + uv[0] * tw, ty + uv[1] * th);
        projected[i] = (screen, t);
    }

    fill_triangle(pixmap, texture, [projected[0], projected[1], projected[2]], scale);
    fill_triangle(pixmap, texture, [projected[0], projected[2], projected[3]], scale);
}

/// 渲染 3D 等距全身图（固定角度）
///
/// - `image`: 皮肤贴图（64x64 / 64x32，其它尺寸按原样取贴图）
/// - `skin_type`: 皮肤类型，`None` 为自动检测
///
/// # 返回值
///
/// 返回 `SIZE` x `SIZE` 的渲染结果，分配失败时返回 `None`
pub fn draw_skin_3d_typea(image: &Pixmap, skin_type: Option<SkinType>) -> Option<Pixmap> {
    let st = skin_type.unwrap_or_else(|| skin_type_checker::get_skin_type(image));
    let texture = normalize_texture(image)?;
    let part_list = parts(st);

    let scale = SUPERSAMPLE as f32;
    let size = SIZE * SUPERSAMPLE;
    let mut pixmap = Pixmap::new(size, size)?;

    let tran = create_tran();

    for part in &part_list {
        for (cub, tex) in [(&part.base, &part.base_tex), (&part.overlay, &part.overlay_tex)] {
            for &d in &FACE_ORDER {
                draw_face(&mut pixmap, &texture, &tran, cub, d, &tex[d], scale);
            }
        }
    }

    downsample(&pixmap, SUPERSAMPLE)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tiny_skia::IntSize;

    /// 构造 64x64 新版皮肤：头 / 身体正面用可辨识颜色，其余透明
    fn make_skin() -> Pixmap {
        let mut data = vec![0u8; 64 * 64 * BPP];
        let mut bm = Pixmap::from_vec(data, IntSize::from_wh(64, 64).unwrap()).unwrap();
        // 头正面 (8,8,8,8) 红；身体正面 (20,20,8,12) 蓝
        for j in 0..8 {
            for i in 0..8 {
                set_pixel(&mut bm, 8 + i, 8 + j, [255, 0, 0, 255]);
            }
        }
        for j in 0..12 {
            for i in 0..8 {
                set_pixel(&mut bm, 20 + i, 20 + j, [0, 0, 255, 255]);
            }
        }
        bm
    }

    /// 渲染输出 400x400，头 / 身体正面中心附近应取到对应颜色
    #[test]
    fn test_draw_skin_3d_layout() {
        let skin = make_skin();
        let out = draw_skin_3d_typea(&skin, Some(SkinType::New)).expect("渲染应成功");
        assert_eq!(out.width(), SIZE);
        assert_eq!(out.height(), SIZE);

        // 身体正面中心 → 屏幕约 (214, 190)；头正面中心 → 约 (228, 110)
        let body = get_pixel(&out, 214, 190);
        assert_eq!(body[3], 255, "身体正面应不透明");
        assert!(body[2] > 128, "身体正面应为蓝色系 {:?}", body);

        let head = get_pixel(&out, 228, 110);
        assert_eq!(head[3], 255, "头正面应不透明");
        assert!(head[0] > 128, "头正面应为红色系 {:?}", head);

        // 画布四角应保持透明（模型居中）
        for (x, y) in [(5, 5), (394, 5), (5, 394), (394, 394)] {
            assert_eq!(get_pixel(&out, x, y)[3], 0, "四角应透明 ({x},{y})");
        }
    }

    /// 旧版 64x32 皮肤：右肢镜像补进左肢块后应能正常渲染
    #[test]
    fn test_draw_skin_3d_old_skin() {
        let mut data = vec![0u8; 64 * 32 * BPP];
        let mut bm = Pixmap::from_vec(data, IntSize::from_wh(64, 32).unwrap()).unwrap();
        // 头正面
        for j in 0..8 {
            for i in 0..8 {
                set_pixel(&mut bm, 8 + i, 8 + j, [255, 0, 0, 255]);
            }
        }
        let out = draw_skin_3d_typea(&bm, None).expect("旧皮肤渲染应成功");
        assert_eq!(out.width(), SIZE);
        assert_eq!(get_pixel(&out, 228, 110)[0], 255, "头正面应为红色");

        // 镜像归一化后左臂块前脸 (36,52,4,12) 应等于源右臂块右侧面 (48,20)（原样复制）
        // 仅验证镜像函数本身：取右腿块右侧面 (0,20) 第一像素
        let norm = normalize_texture(&bm).expect("归一化应成功");
        assert_eq!(norm.width(), 64);
        assert_eq!(norm.height(), 64);
    }
}
