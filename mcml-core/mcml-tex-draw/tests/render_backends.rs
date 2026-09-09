//! 四后端对比：同一模型分别用 DX12 / VK / GL / CPU(skia) 各渲染一张
//!
//! 运行：cargo test -p mcml-tex-draw --test render_backends -- --ignored --nocapture

use std::{collections::HashMap, path::PathBuf};

use mcml_base::archives::BaseArchive;

fn ref_jar() -> PathBuf {
    std::env::var("MCML_TEST_JAR").map(PathBuf::from).unwrap_or_else(|_| {
        PathBuf::from(std::env::var("TEMP").unwrap_or_default())
            .join("mcml-262-ref")
            .join("client.jar")
    })
}

fn out_dir() -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("out");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn write_png(dir: &PathBuf, name: &str, size: u32, rgba: &[u8]) {
    let path = dir.join(name);
    let file = std::fs::File::create(&path).unwrap();
    let mut encoder = png::Encoder::new(file, size, size);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().unwrap();
    writer.write_image_data(rgba).unwrap();
    println!("输出：{}", path.display());
}

/// 渲染单个方块模型并出图（VK/GL在部分机器上可能无适配器，失败只打印跳过）
#[test]
#[ignore]
fn render_all_backends() {
    let jar = ref_jar();
    assert!(jar.exists(), "参考jar不存在：{}", jar.display());
    let archive = BaseArchive::open(&jar).unwrap();

    let mut cache = HashMap::new();
    let Some((model, textures)) =
        mcml_tex_draw::model::bake_model(&archive, "block/furnace", &mut cache)
    else {
        panic!("bake_model 失败");
    };
    println!("bake：{} quads，{} 贴图", model.quads.len(), textures.len());

    let dir = out_dir();

    // 诊断：每个quad的屏幕包围盒与深度（与 render_cpu 同一矩阵）
    {
        use glam::{Mat3, Mat4, Vec3, Vec4};
        let size = 256.0f32;
        let model_m = Mat4::from_translation(Vec3::new(size / 2.0, size / 2.0, 0.0))
            * Mat4::from_scale(Vec3::new(size, -size, size))
            * mcml_tex_draw::gpu::item_transform_matrix(&model.transform);
        let (l0, l1) = mcml_tex_draw::gpu::light_dirs(model.gui_light_3d);
        for q in &model.quads {
            let mut min_x = f32::MAX;
            let mut max_x = f32::MIN;
            let mut min_y = f32::MAX;
            let mut max_y = f32::MIN;
            let mut z = 0.0;
            for p in &q.pos {
                let v = model_m * Vec4::new(p[0], p[1], p[2], 1.0);
                min_x = min_x.min(v.x);
                max_x = max_x.max(v.x);
                min_y = min_y.min(v.y);
                max_y = max_y.max(v.y);
                z += v.z;
            }
            let n = Mat3::from_mat4(model_m).inverse().transpose()
                * Vec3::from(q.normal);
            let n = n.normalize();
            let d = Vec3::from_slice(&l0[..3]).dot(n).max(0.0)
                + Vec3::from_slice(&l1[..3]).dot(n).max(0.0);
            println!(
                "  {:8} 屏幕x[{:.0},{:.0}] y[{:.0},{:.0}] depth={:.2} n=({:.2},{:.2},{:.2}) accum={:.3}",
                q.tex.split('/').next_back().unwrap_or(""),
                min_x, max_x, min_y, max_y,
                z / 4.0,
                n[0], n[1], n[2],
                (d * 0.6 + 0.4).min(1.0),
            );
        }
    }

    // CPU（skia，painter's algorithm）
    match mcml_tex_draw::cpu::render_cpu(&model, &textures, 256) {
        Some(pixels) => write_png(&dir, "furnace_cpu.png", 256, &pixels),
        None => println!("CPU 渲染失败"),
    }

    // 各GPU后端
    let backends: [(&str, wgpu::Backends); 3] = [
        ("DX12", wgpu::Backends::DX12),
        ("VULKAN", wgpu::Backends::VULKAN),
        ("GL", wgpu::Backends::GL),
    ];
    for (name, be) in backends {
        match mcml_tex_draw::gpu::GpuCtx::try_new_backend(name, be) {
            Some(ctx) => match ctx.render(&model, &textures, 256) {
                Some(pixels) => write_png(&dir, &format!("furnace_{name}.png"), 256, &pixels),
                None => println!("{name} 渲染失败"),
            },
            None => println!("{name} 无可用适配器，跳过"),
        }
    }
}
