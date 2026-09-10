//! 单方块 / 单物品对照渲染：GPU管道冒烟与遮挡/透明诊断（直接用本地jar，不走下载链）
//!
//! 运行：cargo test -p mcml-tex-draw --test render_single -- --ignored --nocapture
//! 环境变量：
//! - MCML_TEST_JAR 指定客户端jar，缺省用反编译参考jar
//! - MCML_TEST_BLOCK 指定方块模型（如 block/big_dripleaf），缺省 block/oak_stairs
//! - MCML_TEST_ITEM 指定物品ID（如 enchanted_book），缺省 apple
//! GPU/CPU 各出一张图到 tests/out/

use std::{collections::HashMap, path::PathBuf};

use mcml_base::archives::BaseArchive;

fn ref_jar() -> PathBuf {
    std::env::var("MCML_TEST_JAR").map(PathBuf::from).unwrap_or_else(|_| {
        PathBuf::from(std::env::var("TEMP").unwrap_or_default())
            .join("mcml-262-ref")
            .join("client.jar")
    })
}

fn out_path(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("out");
    std::fs::create_dir_all(&dir).unwrap();
    dir.join(name)
}

fn write_png(path: &PathBuf, size: u32, rgba: &[u8]) {
    let file = std::fs::File::create(path).unwrap();
    let mut encoder = png::Encoder::new(file, size, size);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().unwrap();
    writer.write_image_data(rgba).unwrap();
    println!("输出：{}", path.display());
}

/// 渲染单个方块模型并出图（GPU与CPU各出一张，诊断遮挡/透明问题）
#[test]
#[ignore]
fn render_one_block() {
    let jar = ref_jar();
    assert!(jar.exists(), "参考jar不存在：{}", jar.display());
    let archive = BaseArchive::open(&jar).unwrap();

    let rel = std::env::var("MCML_TEST_BLOCK").unwrap_or_else(|_| "block/oak_stairs".into());
    let name = rel.rsplit('/').next().unwrap_or(&rel).to_string();

    let mut cache = HashMap::new();
    let Some((model, textures)) =
        mcml_tex_draw::model::bake_model(&archive, &rel, &mut cache)
    else {
        panic!("bake_model 失败");
    };
    println!(
        "bake：{} quads，{} 贴图，gui_light_3d={}",
        model.quads.len(),
        textures.len(),
        model.gui_light_3d
    );
    for q in &model.quads {
        println!(
            "  quad tex={} n=({:.0},{:.0},{:.0}) translucent={} p0={:.3?} p2={:.3?}",
            q.tex.split('/').next_back().unwrap_or(""),
            q.normal[0],
            q.normal[1],
            q.normal[2],
            q.translucent,
            q.pos[0],
            q.pos[2]
        );
    }

    // CPU 侧复算 shader 光照，诊断明暗是否符合预期
    {
        use glam::{Mat3, Mat4, Vec3};
        let slot = 256.0;
        println!(
            "  transform rot={:?} t={:?} s={:?}",
            model.transform.rotation, model.transform.translation, model.transform.scale
        );
        let model_m = Mat4::from_translation(Vec3::new(slot / 2.0, slot / 2.0, 0.0))
            * Mat4::from_scale(Vec3::new(slot, -slot, slot))
            * mcml_tex_draw::gpu::item_transform_matrix(&model.transform);
        let n3 = Mat3::from_mat4(model_m).inverse().transpose();
        println!("  n3列={:?}", n3.to_cols_array());
        let (l0, l1) = mcml_tex_draw::gpu::light_dirs(model.gui_light_3d);
        println!(
            "  L0=({:.3},{:.3},{:.3}) L1=({:.3},{:.3},{:.3})",
            l0[0], l0[1], l0[2], l1[0], l1[1], l1[2]
        );
        for q in &model.quads {
            let n = n3 * Vec3::from(q.normal);
            let n = n.normalize();
            let d = Vec3::from_slice(&l0[..3]).dot(n).max(0.0)
                + Vec3::from_slice(&l1[..3]).dot(n).max(0.0);
            let accum = (d * 0.6 + 0.4).min(1.0);
            println!(
                "  光照 tex={} n=({:.2},{:.2},{:.2}) accum={:.3}",
                q.tex.split('/').next_back().unwrap_or(""),
                n[0],
                n[1],
                n[2],
                accum
            );
        }
    }

    let ctx = mcml_tex_draw::gpu::GpuCtx::try_new();
    if let Some(ctx) = ctx.as_ref() {
        let Some(pixels) = ctx.render(&model, &textures, None, 256) else {
            panic!("GPU渲染失败");
        };
        write_png(&out_path(&format!("{name}_gpu.png")), 256, &pixels);
    } else {
        println!("无可用GPU后端，跳过GPU出图");
    }

    // CPU对照：painter's algorithm（无深度缓冲），用于区分GPU深度问题与数据问题
    if let Some(pixels) = mcml_tex_draw::cpu::render_cpu(&model, &textures, 256) {
        write_png(&out_path(&format!("{name}_cpu.png")), 256, &pixels);
    }
}

/// 渲染单个物品图标（items/*.json → extrude/元素烘焙 → GPU/CPU各出一张）
/// 样例：diamond_sword（挤出）、enchanted_book（glint）、compass（condition→range_dispatch）、
/// spyglass（select）、leather_chestplate（tint）、white_bed（composite拼合）
#[test]
#[ignore]
fn render_one_item() {
    let jar = ref_jar();
    assert!(jar.exists(), "参考jar不存在：{}", jar.display());
    let archive = BaseArchive::open(&jar).unwrap();

    let id = std::env::var("MCML_TEST_ITEM").unwrap_or_else(|_| "apple".into());
    let name = id.rsplit('/').next().unwrap_or(&id).to_string();

    // 物品输出目录初始化到独立子目录（渲染结果再拷到tests/out查看）
    let dir = out_path("item-render");
    mcml_tex_draw::init(&dir).unwrap();

    let mut cache = HashMap::new();
    let gpu = mcml_tex_draw::gpu::GpuCtx::try_new();
    let out = mcml_tex_draw::item::render_item(gpu.as_ref(), &archive, &mut cache, &id)
        .unwrap_or_else(|| panic!("render_item GPU失败：{id}"));
    std::fs::copy(dir.join("items").join(&out.1), out_path(&format!("{name}_gpu.png"))).unwrap();

    let out = mcml_tex_draw::item::render_item(None, &archive, &mut cache, &id)
        .unwrap_or_else(|| panic!("render_item CPU失败：{id}"));
    std::fs::copy(dir.join("items").join(&out.1), out_path(&format!("{name}_cpu.png"))).unwrap();
}
