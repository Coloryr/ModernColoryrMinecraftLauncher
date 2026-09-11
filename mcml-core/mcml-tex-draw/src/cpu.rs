//! CPU 渲染后端（skia 2D软件光栅化）：GPU全链不可用时的回退
//!
//! 与GPU同一变换链和光照；quad先做背面剔除（与GPU cull Back一致），
//! 再按深度排序 painter's algorithm，逐quad预乘亮度贴图。

use std::{collections::HashMap, slice};

use glam::{Mat3, Mat4, Vec3, Vec4};
use skia_safe::{AlphaType, Bitmap, BlendMode, Color, ColorType, ImageInfo, Paint, Point, SamplingOptions, TileMode, surfaces, vertices};

use crate::model::BakedModel;

/// 生成按颜色预乘的贴图副本（只乘RGB，保持alpha不变）
pub(crate) fn shade_texture(tex: &Bitmap, shade: [u8; 3]) -> Option<Bitmap> {
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

/// 渲染一个烘焙模型到 size×size RGBA，返回 straight-alpha 像素（未成功返回 None）
pub fn render_cpu(
    model: &BakedModel,
    textures: &HashMap<String, Bitmap>,
    size: u32,
) -> Option<Vec<u8>> {
    let slot = size as f32;
    // 直接用模型→像素矩阵（y向下的屏幕空间），无需再过正交投影
    let model_m = Mat4::from_translation(Vec3::new(slot / 2.0, slot / 2.0, 0.0))
        * Mat4::from_scale(Vec3::new(slot, -slot, slot))
        * crate::gpu::item_transform_matrix(&model.transform);
    let n3 = Mat3::from_mat4(model_m).inverse().transpose();
    let (light0, light1) = crate::gpu::light_dirs(model.gui_light_3d);

    // 投影全部quad到屏幕，按quad计算光照（面内法线一致）
    struct CpuQuad {
        pos: [Point; 4],
        depth: f32,
        uv: [[f32; 2]; 4],
        shade: [u8; 3],
        tex: String,
    }
    let mut quads: Vec<CpuQuad> = Vec::with_capacity(model.quads.len());
    for quad in &model.quads {
        let mut pos = [Point::new(0.0, 0.0); 4];
        let mut depth = 0.0;
        for (i, p) in quad.pos.iter().enumerate() {
            let v = model_m * Vec4::new(p[0], p[1], p[2], 1.0);
            // 像素坐标（y向下），z越大越近（离观察者越近像素z越大）
            pos[i] = Point::new(v.x, v.y);
            depth += v.z;
        }
        let n = (n3 * Vec3::from(quad.normal)).normalize();
        // 背面剔除（同GPU cull Back：面朝向与视线反向才可见）
        if n.z <= 0.0 {
            continue;
        }
        let d = Vec3::from_slice(&light0[..3]).dot(n).max(0.0)
            + Vec3::from_slice(&light1[..3]).dot(n).max(0.0);
        // 自发光（火焰）不做方向光衰减
        let accum = if quad.fullbright {
            1.0
        } else {
            (d * 0.6 + 0.4).min(1.0)
        };
        // 按quad亮度+染色预乘RGB（alpha通道由贴图原始值决定）
        let t = quad.color[0];
        quads.push(CpuQuad {
            pos,
            depth: depth / 4.0,
            uv: quad.uv,
            shade: [
                (accum * t[0] * 255.0) as u8,
                (accum * t[1] * 255.0) as u8,
                (accum * t[2] * 255.0) as u8,
            ],
            tex: quad.tex.clone(),
        });
    }

    // 远→近排序（剔除背面后 n.z>0，z越小越远）
    quads.sort_by(|a, b| a.depth.partial_cmp(&b.depth).unwrap_or(std::cmp::Ordering::Equal));

    let info = ImageInfo::new(
        (size as i32, size as i32),
        ColorType::RGBA8888,
        AlphaType::Unpremul,
        None,
    );
    let mut surface = surfaces::raster(&info, None, None)?;
    let canvas = surface.canvas();
    canvas.clear(Color::TRANSPARENT);

    for q in &quads {
        let tex = textures.get(&q.tex)?;

        // 按亮度预乘贴图副本（skia带贴图shader时顶点颜色不参与混合）
        let Some(shaded) = shade_texture(tex, q.shade) else {
            return None;
        };

        let mut builder = vertices::Builder::new(
            vertices::VertexMode::Triangles,
            6,
            0,
            vertices::BuilderFlags::HAS_TEX_COORDS,
        );
        for (i, p) in builder.positions().iter_mut().enumerate() {
            *p = q.pos[[0, 1, 2, 0, 2, 3][i]];
        }
        for (i, tc) in builder.tex_coords().unwrap().iter_mut().enumerate() {
            let uv = q.uv[[0, 1, 2, 0, 2, 3][i]];
            *tc = Point::new(uv[0] * tex.width() as f32, uv[1] * tex.height() as f32);
        }
        let mut paint = Paint::default();
        paint.set_anti_alias(true);
        paint.set_shader(shaded.to_shader(
            Some((TileMode::Clamp, TileMode::Clamp)),
            SamplingOptions::default(),
            None,
        ));
        canvas.draw_vertices(&builder.detach(), BlendMode::SrcOver, &paint);
    }

    let mut out = vec![0u8; (size * size * 4) as usize];
    let image = surface.image_snapshot();
    if !image.read_pixels(
        &info,
        &mut out,
        info.min_row_bytes(),
        (0, 0),
        skia_safe::image::CachingHint::Disallow,
    ) {
        return None;
    }
    Some(out)
}
