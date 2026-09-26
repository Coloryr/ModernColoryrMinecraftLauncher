//! 皮肤整体 3D 等距渲染（wgpu 离屏：固定视角正交投影 + 深度测试）
//!
//! 模型坐标约定与 `mml-skin-render` 一致（cube.rs 顶点顺序 + texture.rs UV 布局）：
//! 角色面朝 +z、x− 为角色右手侧、+y 为上，原点取身高一半处；贴图 v 向下。
//! 旧版 64x32 皮肤先补成 64x64（右肢镜像填进左肢块）再统一按新版布局取贴图。
//! 底层部件先画（写深度），顶层 1.125 倍 overlay 后画（半透明混合）。

use std::f32::consts::PI;

use glam::{Mat4, Vec3};
use mml_skin::{SkinType, skin_type_checker};
use tiny_skia::Pixmap;

use crate::gpu_3d::{face_verts, render_3d};
use crate::skin_draw::{BPP, draw, row_bytes};

/// 输出尺寸（与皮肤 2D 叠加一致）
const SIZE_W: u32 = 272;
const SIZE_H: u32 = 532;

/// 模型缩放（全身 32 像素 + overlay 边缘 33 像素 → 478px，画布 532 高）
const SCALE: f32 = 14.5;

/// 超采样倍数（2x 渲染后降采样得到抗锯齿）
const SUPERSAMPLE: u32 = 2;

/// 每像素读取 / 写入的原始 4 字节（预乘 BGRA，仅镜像归一化用）
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

/// 面方向下标（cube.rs 顶点表的面顺序）
mod face {
    /// 背面（z−，角色背后）
    pub const BACK: usize = 0;
    /// 正面（z+，角色面朝方向）
    pub const FRONT: usize = 1;
    /// 角色右手侧（x−）
    pub const RIGHT: usize = 2;
    /// 角色左手侧（x+）
    pub const LEFT: usize = 3;
    /// 顶面（y+）
    pub const TOP: usize = 4;
    /// 底面（y−）
    pub const BOTTOM: usize = 5;
    /// 全部方向（深度测试自动处理遮挡，顺序不影响结果）
    pub const ALL: [usize; 6] = [BACK, FRONT, RIGHT, LEFT, TOP, BOTTOM];
}

/// 取立方体某个面的四个角（相对部件中心的偏移 + 块内归一化 uv）
///
/// 顶点顺序与 uv 布局来自 `mml-skin-render`（cube.rs 的 CUBE / texture.rs 的
/// UV 表）：正面 / 侧面 / 顶面 u 沿面向右、v 沿面向下；背面 / 底面 u 镜像。
/// uv 为 0..1 的块内比例（v=0 在贴图块顶部）。
fn face_corners(h: [f32; 3], d: usize) -> [([f32; 3], [f32; 2]); 4] {
    let (hx, hy, hz) = (h[0], h[1], h[2]);
    match d {
        face::BACK => [
            ([hx, hy, -hz], [1.0, 0.0]),
            ([hx, -hy, -hz], [1.0, 1.0]),
            ([-hx, -hy, -hz], [0.0, 1.0]),
            ([-hx, hy, -hz], [0.0, 0.0]),
        ],
        face::FRONT => [
            ([-hx, hy, hz], [0.0, 0.0]),
            ([-hx, -hy, hz], [0.0, 1.0]),
            ([hx, -hy, hz], [1.0, 1.0]),
            ([hx, hy, hz], [1.0, 0.0]),
        ],
        face::RIGHT => [
            ([-hx, hy, -hz], [0.0, 0.0]),
            ([-hx, -hy, -hz], [0.0, 1.0]),
            ([-hx, -hy, hz], [1.0, 1.0]),
            ([-hx, hy, hz], [1.0, 0.0]),
        ],
        face::LEFT => [
            ([hx, hy, hz], [0.0, 0.0]),
            ([hx, -hy, hz], [0.0, 1.0]),
            ([hx, -hy, -hz], [1.0, 1.0]),
            ([hx, hy, -hz], [1.0, 0.0]),
        ],
        face::TOP => [
            ([-hx, hy, -hz], [0.0, 0.0]),
            ([-hx, hy, hz], [0.0, 1.0]),
            ([hx, hy, hz], [1.0, 1.0]),
            ([hx, hy, -hz], [1.0, 0.0]),
        ],
        _ => [
            ([hx, -hy, -hz], [1.0, 0.0]),
            ([hx, -hy, hz], [1.0, 1.0]),
            ([-hx, -hy, hz], [0.0, 1.0]),
            ([-hx, -hy, -hz], [0.0, 0.0]),
        ],
    }
}

/// 一个面的贴图区域：皮肤贴图上的 (tx, ty, tw, th)
type FaceRect = [i32; 4];

/// 从部件尺寸 (W 宽, H 高, D 深) 与贴图块左上角 (bx, by) 取六个面的区域
///
/// 块内布局（texture.rs，W/H/D 为部件三向尺寸）：
/// 上排 [顶(W,D) | 底(W,D)]，下排 [右(D,H) | 前(W,H) | 左(D,H) | 后(W,H)]；
/// 「右」= 角色右手侧面（x−），即贴图块最左列。
fn face_rect(w: i32, h: i32, d: i32, bx: i32, by: i32, di: usize) -> FaceRect {
    match di {
        face::BACK => [bx + 2 * d + w, by + d, w, h],
        face::FRONT => [bx + d, by + d, w, h],
        face::RIGHT => [bx, by + d, d, h],
        face::LEFT => [bx + d + w, by + d, d, h],
        face::TOP => [bx + d, by, w, d],
        _ => [bx + d + w, by, w, d],
    }
}

/// 一个身体部件：尺寸 + 基础层 / overlay 层的中心与半尺寸 + 贴图块左上角
struct Part {
    /// 部件尺寸 (W 宽, H 高, D 深)，皮肤像素
    dims: [i32; 3],
    /// 基础层中心（模型坐标）与半尺寸
    base_center: [f32; 3],
    base_half: [f32; 3],
    /// overlay 层半尺寸（基础层 × 1.125）与贴图块
    overlay_half: [f32; 3],
    base_block: [i32; 2],
    overlay_block: [i32; 2],
}

/// 组装全身部件（纤细手臂 3 像素宽，普通 4 像素宽）
///
/// 手臂整段贴在身体侧面之外（身体 x 半宽 4，手臂从 ±4 往外延伸）；
/// 角色右手侧在 x−。
fn parts(skin_type: SkinType) -> Vec<Part> {
    let slim = skin_type == SkinType::NewSlim;
    let aw = if slim { 3 } else { 4 }; // 手臂宽（像素）
    let enlarge = 1.125; // overlay 层放大系数

    let mk = |dims: [i32; 3], center: [f32; 3], base_block: [i32; 2], overlay_block: [i32; 2]| {
        let half = [dims[0] as f32 / 2.0, dims[1] as f32 / 2.0, dims[2] as f32 / 2.0];
        Part {
            dims,
            base_center: center,
            base_half: half,
            overlay_half: [half[0] * enlarge, half[1] * enlarge, half[2] * enlarge],
            base_block,
            overlay_block,
        }
    };

    vec![
        // 头：中心在颈部上方 4px（y = 12）
        mk([8, 8, 8], [0.0, 12.0, 0.0], [0, 0], [32, 0]),
        // 身体
        mk([8, 12, 4], [0.0, 2.0, 0.0], [16, 16], [16, 32]),
        // 右腿 / 左腿（右手侧在 x−）
        mk([4, 12, 4], [-2.0, -10.0, 0.0], [0, 16], [0, 32]),
        mk([4, 12, 4], [2.0, -10.0, 0.0], [16, 48], [0, 48]),
        // 右臂 / 左臂：整段贴在身体侧面之外
        mk([aw, 12, 4], [-4.0 - aw as f32 / 2.0, 2.0, 0.0], [40, 16], [40, 32]),
        mk([aw, 12, 4], [4.0 + aw as f32 / 2.0, 2.0, 0.0], [32, 48], [48, 48]),
    ]
}

/// 模型变换矩阵（俯仰 + 偏航 → 缩放 → 平移到画布中心）
///
/// - `pitch`: 俯仰角（度，负 = 面朝上 / 抬头，正 = 面朝下 / 低头）
/// - `yaw`: 偏航角（度，角色面朝 +z，正值 = 正面朝向屏幕右侧）
fn create_tran(pitch: f32, yaw: f32) -> Mat4 {
    let roty = Mat4::from_rotation_y(yaw * PI / 180.0);
    let rotx = Mat4::from_rotation_x(pitch * PI / 180.0);

    let scale = Mat4::from_scale(Vec3::new(SCALE, -SCALE, SCALE));

    let tran = Mat4::from_translation(Vec3::new(
        SIZE_W as f32 / 2.0,
        SIZE_H as f32 / 2.0,
        0.0,
    ));

    tran * scale * rotx * roty
}

/// 渲染 3D 等距全身图（指定角度）
///
/// - `image`: 皮肤贴图（64x64 / 64x32，其它尺寸按原样取贴图）
/// - `skin_type`: 皮肤类型，`None` 为自动检测
/// - `pitch`: 俯仰角（度，负 = 面朝上，正 = 面朝下）
/// - `yaw`: 偏航角（度）
///
/// # 返回值
///
/// 返回 `SIZE_W` x `SIZE_H` 的渲染结果，分配失败或无可用 GPU 时返回 `None`
pub fn draw_skin_3d_typeb(
    image: &Pixmap,
    skin_type: Option<SkinType>,
    pitch: f32,
    yaw: f32,
) -> Option<Pixmap> {
    let st = skin_type.unwrap_or_else(|| skin_type_checker::get_skin_type(image));
    let texture = normalize_texture(image)?;
    let part_list = parts(st);
    let (tw, th) = (texture.width() as f32, texture.height() as f32);

    let mut base = Vec::new();
    let mut overlay = Vec::new();
    for part in &part_list {
        for &d in &face::ALL {
            base.extend_from_slice(&face_verts(
                face_corners(part.base_half, d)
                    .map(|(pos, uv)| {
                        (
                            [pos[0] + part.base_center[0], pos[1] + part.base_center[1], pos[2] + part.base_center[2]],
                            uv,
                        )
                    }),
                face_rect(part.dims[0], part.dims[1], part.dims[2], part.base_block[0], part.base_block[1], d),
                tw,
                th,
            ));
            overlay.extend_from_slice(&face_verts(
                face_corners(part.overlay_half, d)
                    .map(|(pos, uv)| {
                        (
                            [pos[0] + part.base_center[0], pos[1] + part.base_center[1], pos[2] + part.base_center[2]],
                            uv,
                        )
                    }),
                face_rect(part.dims[0], part.dims[1], part.dims[2], part.overlay_block[0], part.overlay_block[1], d),
                tw,
                th,
            ));
        }
    }

    render_3d(&texture, &base, &overlay, create_tran(pitch, yaw), SIZE_W, SIZE_H, SUPERSAMPLE)
}

/// 渲染 3D 等距全身图（固定角度：水平 45°、面朝上 30°）
///
/// - `image`: 皮肤贴图
/// - `skin_type`: 皮肤类型，`None` 为自动检测
///
/// # 返回值
///
/// 返回 `SIZE_W` x `SIZE_H` 的渲染结果，分配失败或无可用 GPU 时返回 `None`
pub fn draw_skin_3d_typea(image: &Pixmap, skin_type: Option<SkinType>) -> Option<Pixmap> {
    draw_skin_3d_typeb(image, skin_type, -30.0, 45.0)
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

    /// 渲染输出 272x532；角色面向屏幕右：头正面中心 ≈ (177, 95)、
    /// 身体正面中心 ≈ (156, 231)
    #[test]
    fn test_draw_skin_3d_layout() {
        let skin = make_skin();
        let out = draw_skin_3d_typea(&skin, Some(SkinType::New)).expect("渲染应成功");
        assert_eq!(out.width(), SIZE_W);
        assert_eq!(out.height(), SIZE_H);

        let head = get_pixel(&out, 177, 95);
        assert_eq!(head[3], 255, "头正面应不透明");
        assert!(head[0] > 128, "头正面应为红色系 {:?}", head);

        let body = get_pixel(&out, 156, 231);
        assert_eq!(body[3], 255, "身体正面应不透明");
        assert!(body[2] > 128, "身体正面应为蓝色系 {:?}", body);

        // 画布四角应保持透明（模型居中）
        for (x, y) in [(5, 5), (267, 5), (5, 527), (267, 527)] {
            assert_eq!(get_pixel(&out, x, y)[3], 0, "四角应透明 ({x},{y})");
        }
    }

    /// 手臂应整段贴在身体侧面之外：把右臂块的四个侧面都涂绿，
    /// 45° 视角下绿色区域最左端 ≈ 75；若手臂半埋进身体（x 从 -6 起），
    /// 只能到 ≈ 95
    #[test]
    fn test_draw_skin_3d_arm_outside() {
        let mut data = vec![0u8; 64 * 64 * BPP];
        let mut bm = Pixmap::from_vec(data, IntSize::from_wh(64, 64).unwrap()).unwrap();
        // 右臂块下排四个侧面 (40,20)-(56,32) 绿色
        for j in 0..12 {
            for i in 0..16 {
                set_pixel(&mut bm, 40 + i, 20 + j, [0, 255, 0, 255]);
            }
        }
        let out = draw_skin_3d_typea(&bm, Some(SkinType::New)).expect("渲染应成功");

        let mut leftmost = SIZE_W as i32;
        for y in 0..SIZE_H as i32 {
            for x in 0..SIZE_W as i32 {
                if get_pixel(&out, x, y)[3] > 0 {
                    leftmost = x;
                    break;
                }
            }
            if leftmost < SIZE_W as i32 {
                break;
            }
        }
        assert!(
            leftmost < 80,
            "手臂应伸出身体轮廓外（左缘 {leftmost} 应 < 80）"
        );
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
        assert_eq!(out.width(), SIZE_W);
        assert_eq!(get_pixel(&out, 177, 95)[0], 255, "头正面应为红色");

        // 镜像归一化后输出 64x64
        let norm = normalize_texture(&bm).expect("归一化应成功");
        assert_eq!(norm.width(), 64);
        assert_eq!(norm.height(), 64);
    }
}
