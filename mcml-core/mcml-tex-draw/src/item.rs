//! 物品渲染：items/*.json 图标定义 → 模型解析 → extrude 挤出/元素烘焙 → GPU/CPU 出图
//!
//! 与方块共用烘焙与渲染管线（model.rs/block::render_baked），差异在：
//! - items/*.json 的 model 节点为类型分派（model/composite/condition/select/range_dispatch），
//!   special（实体渲染：箱子/头颅/旗帜等）与 empty 跳过
//! - builtin/generated 链（无 elements）按反编译 ItemModelGenerator 挤出：
//!   front/back 全幅面 + 不透明像素边界裙边，gui_light=front
//! - tintindex 索引 items/*.json 的 tint 表（extrude 按层号索引）
//! - glint（附魔光效）：vanilla 默认注册带 ENCHANTMENT_GLINT_OVERRIDE 的物品

use std::{
    cell::RefCell,
    collections::HashMap,
    sync::atomic::{AtomicUsize, Ordering},
};

use glam::{Mat3, Mat4, Quat, Vec3};
use mcml_base::{archives::BaseArchive, serialize_tools};
use mcml_game::gui_hook::ProgressGui;
use mcml_names::i18_items::error_type::{CoreResult, ErrorType};
use rayon::prelude::*;
use serde::Deserialize;
use skia_safe::Bitmap;

use crate::{
    block::{fit_composite, read_anim_meta, render_baked, GRASS_TINT},
    gpu::GpuCtx,
    model::{
        bake_element, calculate_facing, face_index_of, face_normal, load_texture,
        rect_translucent, recalculate_winding, resolve, texture_path, BakedModel, GuiTransform,
        Quad, FACE_INFO, Resolved, select_extent,
    },
};
use mcml_sys::path_helper;

/// glint 贴图（REPEAT 采样，光纹平铺）
const GLINT_TEX: &str = "assets/minecraft/textures/misc/enchanted_glint_item.png";

/// vanilla 默认注册带附魔光效的物品（Items 类 ENCHANTMENT_GLINT_OVERRIDE）
const GLINT_IDS: [&str; 2] = ["enchanted_book", "enchanted_golden_apple"];


/// ===== items/*.json 定义解析 =====

#[derive(Deserialize)]
struct ItemDefJson {
    #[serde(default)]
    model: Option<ItemModelJson>,
}

#[derive(Deserialize)]
struct ItemModelJson {
    /// 类型：minecraft:model / composite / condition / select / range_dispatch /
    /// special / empty（缺省视为 model）
    #[serde(rename = "type", default)]
    kind: Option<String>,
    /// minecraft:model：models/ 下的模型名（如 minecraft:item/apple）
    #[serde(default)]
    model: Option<String>,
    /// minecraft:model：tint 表（模型面 tintindex 索引）
    #[serde(default)]
    tints: Option<Vec<TintJson>>,
    /// minecraft:composite：子模型（各子节点可带 transformation）
    #[serde(default)]
    models: Option<Vec<ItemModelJson>>,
    /// minecraft:condition：图标取条件不满足的默认态
    #[serde(default)]
    on_false: Option<Box<ItemModelJson>>,
    /// 条件满足的分支不渲染图标，仅为完整反序列化保留
    #[serde(default)]
    #[allow(dead_code)]
    on_true: Option<Box<ItemModelJson>>,
    /// minecraft:select：取 when 含 "gui" 的 case，否则 fallback
    #[serde(default)]
    cases: Option<Vec<CaseJson>>,
    /// select / range_dispatch：兜底（无 entry 阈值命中时才用）
    #[serde(default)]
    fallback: Option<Box<ItemModelJson>>,
    /// range_dispatch：按阈值分派的条目
    #[serde(default)]
    entries: Option<Vec<EntryJson>>,
    /// 节点自带变换（composite 子模型拼合用，ItemTransform 语义）
    #[serde(default)]
    transformation: Option<TransformDefJson>,
}

#[derive(Deserialize)]
struct CaseJson {
    #[serde(default)]
    when: Option<SelectWhen>,
    #[serde(default)]
    model: Option<ItemModelJson>,
}

/// range_dispatch 的一个分派条目（阈值 + 模型）
#[derive(Deserialize)]
struct EntryJson {
    #[serde(default)]
    model: Option<ItemModelJson>,
    #[serde(default)]
    threshold: f64,
}

/// select 的 when：字符串或字符串列表（如 "gui" / ["gui", "ground"]）
#[derive(Deserialize)]
#[serde(untagged)]
enum SelectWhen {
    One(String),
    Many(Vec<String>),
}

#[derive(Deserialize)]
struct TintJson {
    #[serde(rename = "type", default)]
    kind: Option<String>,
    /// minecraft:constant 的颜色
    #[serde(default)]
    value: Option<i64>,
    /// dye/firework/map_color/potion 等的默认色
    #[serde(default)]
    default: Option<i64>,
}

/// 节点 transformation（ItemTransform 语义，quaternion 为 [x,y,z,w]）
#[derive(Deserialize)]
struct TransformDefJson {
    #[serde(default)]
    translation: Option<Vec<f32>>,
    #[serde(default)]
    left_rotation: Option<Vec<f32>>,
    /// 欧拉角（度）
    #[serde(default)]
    rotation: Option<Vec<f32>>,
    #[serde(default)]
    scale: Option<Vec<f32>>,
    #[serde(default)]
    right_rotation: Option<Vec<f32>>,
}

/// 一个可渲染子模型（composite 展平后的项）
struct ChildModel {
    /// models/ 下的相对名（如 item/apple）
    model: String,
    /// tint 表（tintindex 索引）
    tints: Vec<[u8; 3]>,
    /// 节点自带 transformation（烘焙期直接作用于顶点）
    transform: Option<Mat4>,
}

/// 解析 model 节点为可渲染子模型列表；special/empty/未知类型返回 None（跳过该物品）
fn resolve_item_models(node: &ItemModelJson) -> Option<Vec<ChildModel>> {
    match node.kind.as_deref() {
        None | Some("minecraft:model") => {
            let model = strip_ns(node.model.as_ref()?);
            Some(vec![ChildModel {
                model,
                tints: node
                    .tints
                    .as_ref()
                    .map(|list| list.iter().map(tint_color).collect())
                    .unwrap_or_default(),
                transform: node.transformation.as_ref().map(transform_matrix),
            }])
        }
        Some("minecraft:composite") => {
            let mut out = Vec::new();
            for child in node.models.as_ref()? {
                out.extend(resolve_item_models(child)?);
            }
            Some(out)
        }
        // 图标取条件不满足的默认态（如指南针非 lodestone 绑定、bundle 未满）
        Some("minecraft:condition") => resolve_item_models(node.on_false.as_ref()?),
        Some("minecraft:select") => {
            let case = node.cases.as_ref()?.iter().find(|c| match &c.when {
                Some(SelectWhen::One(s)) => s == "gui",
                Some(SelectWhen::Many(list)) => list.iter().any(|s| s == "gui"),
                None => false,
            });
            match case {
                Some(c) => resolve_item_models(c.model.as_ref()?),
                None => resolve_item_models(node.fallback.as_ref()?),
            }
        }
        // range_dispatch：静态图标属性值恒为0，取最后一个阈值≤0的entry
        //（指南针/时钟等的entries从threshold 0.0起，fallback缺省）
        Some("minecraft:range_dispatch") => {
            let chosen = node
                .entries
                .as_ref()?
                .iter()
                .filter(|e| e.threshold <= 0.0)
                .last()
                .and_then(|e| e.model.as_ref())
                .or_else(|| node.fallback.as_deref())?;
            resolve_item_models(chosen)
        }
        // special（实体渲染）/ empty / 未知类型跳过
        _ => None,
    }
}

/// 去掉 "minecraft:" 命名空间前缀
fn strip_ns(name: &str) -> String {
    name.strip_prefix("minecraft:").unwrap_or(name).to_string()
}

/// tint 节点 → RGB（grass 类型按平原群系常量色，其余取 ARGB int）
fn tint_color(t: &TintJson) -> [u8; 3] {
    match t.kind.as_deref() {
        Some("minecraft:grass") => GRASS_TINT,
        _ => argb_rgb(t.value.or(t.default).unwrap_or(-1)),
    }
}

/// ARGB int（可为负）→ [r, g, b]
fn argb_rgb(v: i64) -> [u8; 3] {
    let u = v as u32;
    [(u >> 16) as u8, (u >> 8) as u8, u as u8]
}

/// 节点 transformation → 矩阵（照 ItemTransform 语义：T·leftRot·R(euler度)·S·rightRot）
fn transform_matrix(t: &TransformDefJson) -> Mat4 {
    let take = |v: &Option<Vec<f32>>, i: usize, d: f32| {
        v.as_ref().and_then(|v| v.get(i).copied()).unwrap_or(d)
    };
    let quat = |q: &Option<Vec<f32>>| {
        Quat::from_xyzw(
            take(q, 0, 0.0),
            take(q, 1, 0.0),
            take(q, 2, 0.0),
            take(q, 3, 1.0),
        )
    };
    Mat4::from_translation(Vec3::new(
        take(&t.translation, 0, 0.0),
        take(&t.translation, 1, 0.0),
        take(&t.translation, 2, 0.0),
    )) * Mat4::from_quat(quat(&t.left_rotation))
        * Mat4::from_rotation_x(take(&t.rotation, 0, 0.0).to_radians())
        * Mat4::from_rotation_y(take(&t.rotation, 1, 0.0).to_radians())
        * Mat4::from_rotation_z(take(&t.rotation, 2, 0.0).to_radians())
        * Mat4::from_scale(Vec3::new(
            take(&t.scale, 0, 1.0),
            take(&t.scale, 1, 1.0),
            take(&t.scale, 2, 1.0),
        ))
        * Mat4::from_quat(quat(&t.right_rotation))
}


/// ===== bake：extrude 挤出（照反编译 ItemModelGenerator）+ 物品模型烘焙 =====

/// bakeQuad 核心（无元素旋转，照 FaceBakery.bakeQuad）：FaceInfo 角选点/16 → uv 角映射
/// （Quadrant.R0）→ calculateFacing 量化 + 规范角序重排。
/// from/to 可为退化/翻转盒（extrude 裙边即是），照反编译原样传入即可
fn bake_face(
    from: &[f32; 3],
    to: &[f32; 3],
    facing: usize,
    uvs: [f32; 4],
) -> ([[f32; 3]; 4], [[f32; 2]; 4], [f32; 3]) {
    let corner = |i: usize| -> [f32; 3] {
        let info = FACE_INFO[facing][i];
        [
            select_extent(info.0, from, to) / 16.0,
            select_extent(info.1, from, to) / 16.0,
            select_extent(info.2, from, to) / 16.0,
        ]
    };
    let mut pos: [[f32; 3]; 4] = std::array::from_fn(corner);
    let mut uv: [[f32; 2]; 4] = std::array::from_fn(|i| {
        let idx = i % 4; // Quadrant.R0
        let u = if idx == 0 || idx == 1 { uvs[0] } else { uvs[2] };
        let v = if idx == 0 || idx == 3 { uvs[1] } else { uvs[3] };
        [u / 16.0, v / 16.0]
    });
    // 最终朝向与角序由 calculateFacing 量化归一（与游戏一致）
    let f = calculate_facing(&pos).and_then(|dir| face_index_of(Vec3::from(dir)));
    if let Some(f) = f {
        recalculate_winding(&mut pos, &mut uv, f);
    }
    let normal = f
        .map(|f| face_normal(f).to_array())
        .unwrap_or([0.0, 1.0, 0.0]);
    (pos, uv, normal)
}

/// extrude 挤出烘焙（照反编译 ItemModelGenerator）：
/// layer0..layer4 依次叠加遇缺失即止（layer0 缺失整体失败）；每层
/// front/back 全幅面 + 不透明像素边界裙边（动画贴图各帧并集）
fn bake_generated(
    archive: &BaseArchive,
    textures: &HashMap<String, crate::block::TextureRefObj>,
    tex_cache: &mut HashMap<String, Bitmap>,
    tints: &[[u8; 3]],
) -> Option<Vec<Quad>> {
    // 层贴图：layer0..layer4，遇缺失即止
    let layers: Vec<(String, Bitmap)> = (0..5)
        .map_while(|i| {
            let value = textures.get(&format!("layer{i}"))?.value()?;
            let path = texture_path(value)?;
            let tex = load_texture(archive, tex_cache, &path)?;
            Some((path, tex))
        })
        .collect();
    if layers.is_empty() {
        return None;
    }

    const MIN_Z: f32 = 7.5;
    const MAX_Z: f32 = 8.5;
    const UV_SHRINK: f32 = 0.1;

    let mut quads = Vec::new();
    for (layer_index, (path, tex)) in layers.iter().enumerate() {
        let anim = read_anim_meta(archive, path);
        // tintIndex = 层号，越界为白色（与游戏一致）
        let tint = tints.get(layer_index).copied().unwrap_or([255, 255, 255]);
        let color = [
            tint[0] as f32 / 255.0,
            tint[1] as f32 / 255.0,
            tint[2] as f32 / 255.0,
            1.0,
        ];

        // 不透明像素判定（alpha != 0，照 SpriteContents.isTransparent），
        // 动画贴图各帧并集（getSideFrames 走唯一帧集合）
        let Some((base, stride, w, h_total)) = crate::model::bitmap_rgba(tex) else {
            continue;
        };
        if w == 0 {
            continue;
        }
        let frame_count = (h_total / w).max(1);
        let frame_h = h_total / frame_count;
        let opaque = |x: usize, y: usize| -> bool {
            (0..frame_count).any(|frame| {
                let row = frame * w + y;
                row < h_total && base[row * stride + x * 4 + 3] != 0
            })
        };

        // front/back 全幅面（box (0,0,7.5)-(16,16,8.5)，
        // SOUTH uv [0,0,16,16] 正面 z=8.5；NORTH uv [16,0,0,16] 背面 z=7.5 镜像）
        for (facing, uv16) in [
            (3usize, [0.0f32, 0.0, 16.0, 16.0]), // SOUTH
            (2usize, [16.0, 0.0, 0.0, 16.0]),    // NORTH
        ] {
            let (pos, uv, normal) = bake_face(
                &[0.0, 0.0, MIN_Z],
                &[16.0, 16.0, MAX_Z],
                facing,
                uv16,
            );
            quads.push(Quad {
                pos,
                uv,
                normal,
                color: [color; 4],
                tex: path.clone(),
                translucent: rect_translucent(&tex, Some(&anim), &[0.0, 0.0, 1.0, 1.0]),
            });
        }

        // 裙边：不透明像素且邻边透明（出界算透明）→ 侧面。
        // SideDirection：UP=Direction.UP (0,1)、DOWN=DOWN (0,-1)、LEFT=EAST (1,0)、
        // RIGHT=WEST (-1,0)；邻居 = (x-stepX, y-stepY)
        let (fw, fh) = (w as f32, frame_h as f32);
        let (x_scale, y_scale) = (16.0 / fw, 16.0 / fh);
        for (facing, (dx, dy), horizontal) in [
            (1usize, (0isize, 1isize), true),    // UP
            (0usize, (0isize, -1isize), true),   // DOWN
            (5usize, (1isize, 0isize), false),   // LEFT = EAST
            (4usize, (-1isize, 0isize), false),  // RIGHT = WEST
        ] {
            for y in 0..frame_h {
                for x in 0..w {
                    if !opaque(x, y) {
                        continue;
                    }
                    // 邻居透明（出界算透明）才生成裙边
                    let (nx, ny) = (x as isize - dx, y as isize - dy);
                    if nx >= 0 && ny >= 0 && (nx as usize) < w && (ny as usize) < w
                        && opaque(nx as usize, ny as usize)
                    {
                        continue;
                    }

                    // uv：u0=x+0.1 u1=x+0.9；横向面 v0=y+0.1 v1=y+0.9，
                    // 纵向面 v 反转（v0=y+0.9 v1=y+0.1）；按帧尺寸缩放
                    let u0 = (x as f32 + UV_SHRINK) * x_scale;
                    let u1 = (x as f32 + 1.0 - UV_SHRINK) * x_scale;
                    let (v0, v1) = if horizontal {
                        (
                            (y as f32 + UV_SHRINK) * y_scale,
                            (y as f32 + 1.0 - UV_SHRINK) * y_scale,
                        )
                    } else {
                        (
                            (y as f32 + 1.0 - UV_SHRINK) * y_scale,
                            (y as f32 + UV_SHRINK) * y_scale,
                        )
                    };

                    // 几何：startY/endY 按帧尺寸缩放后 y 翻转（16 - 值），
                    // 各方向照反编译 switch 取 from/to（退化盒，由bakeFace归一角序）
                    let (start_x, end_x, start_y, end_y) = match (facing, horizontal) {
                        (1, _) | (0, _) => (
                            x as f32,
                            x as f32 + 1.0,
                            if facing == 1 { y as f32 } else { y as f32 + 1.0 },
                            if facing == 1 { y as f32 } else { y as f32 + 1.0 },
                        ),
                        (5, _) => (x as f32, x as f32, y as f32, y as f32 + 1.0),
                        _ => (x as f32 + 1.0, x as f32 + 1.0, y as f32, y as f32 + 1.0),
                    };
                    let start_x = start_x * x_scale;
                    let end_x = end_x * x_scale;
                    let start_y = 16.0 - start_y * y_scale;
                    let end_y = 16.0 - end_y * y_scale;

                    let (from, to) = match facing {
                        // UP / DOWN：水平 quad（XZ 平面，Y 固定）
                        1 => ([start_x, start_y, MIN_Z], [end_x, start_y, MAX_Z]),
                        0 => ([start_x, end_y, MIN_Z], [end_x, end_y, MAX_Z]),
                        // LEFT / RIGHT：垂直 quad（YZ 平面，X 固定）
                        5 => ([start_x, start_y, MIN_Z], [start_x, end_y, MAX_Z]),
                        _ => ([end_x, start_y, MIN_Z], [end_x, end_y, MAX_Z]),
                    };
                    let (pos, uv, normal) = bake_face(&from, &to, facing, [u0, v0, u1, v1]);
                    quads.push(Quad {
                        pos,
                        uv,
                        normal,
                        color: [color; 4],
                        tex: path.clone(),
                        translucent: rect_translucent(
                            &tex,
                            Some(&anim),
                            &[u0 / 16.0, v0 / 16.0, u1 / 16.0, v1 / 16.0],
                        ),
                    });
                }
            }
        }
    }
    Some(quads)
}

/// 物品模型烘焙：items/*.json 的 model 引用（models/ 相对名）。
/// generated 链（builtin/generated，无 elements）→ extrude；普通模型 → 元素烘焙。
/// `extra`：节点 transformation（composite 子模型拼合用），烘焙期直接作用于顶点
pub(crate) fn bake_item_model(
    archive: &BaseArchive,
    rel: &str,
    tex_cache: &mut HashMap<String, Bitmap>,
    tints: &[[u8; 3]],
    extra: Option<Mat4>,
) -> Option<(BakedModel, HashMap<String, Bitmap>)> {
    let Resolved {
        textures,
        elements,
        transform,
        gui_light_3d,
        generated,
    } = resolve(archive, rel)?;

    let mut quads = if generated && elements.is_empty() {
        bake_generated(archive, &textures, tex_cache, tints)?
    } else {
        let mut quads = Vec::new();
        for element in &elements {
            quads.extend(bake_element(element, &textures, archive, tex_cache, Some(tints)));
        }
        quads
    };
    if quads.is_empty() {
        return None;
    }

    // 节点 transformation 直接作用于顶点（法线按逆转置变换后重新量化）
    if let Some(m) = extra {
        let n3 = Mat3::from_mat4(m).inverse().transpose();
        for quad in &mut quads {
            for p in &mut quad.pos {
                *p = m.transform_point3(Vec3::from(*p)).to_array();
            }
            quad.normal = match calculate_facing(&quad.pos).and_then(|d| face_index_of(d.into())) {
                Some(f) => face_normal(f).to_array(),
                None => (n3 * Vec3::from(quad.normal)).normalize().to_array(),
            };
        }
    }

    let mut used_textures = HashMap::new();
    for quad in &quads {
        if !used_textures.contains_key(&quad.tex) {
            let tex = load_texture(archive, tex_cache, &quad.tex)?;
            used_textures.insert(quad.tex.clone(), tex);
        }
    }
    Some((
        BakedModel {
            quads,
            gui_light_3d,
            transform,
        },
        used_textures,
    ))
}


/// ===== 渲染入口 =====

/// 已渲染完成的版本号（未渲染为空，load_blocks短路用）
pub(crate) fn items_id() -> String {
    crate::items_read().id.clone()
}

/// 按jar内items/*.json渲染全部物品图标
///
/// 渲染流程与render_blocks一致：rayon并发，各工作线程thread_local持有只读jar句柄
/// 与贴图缓存；GPU优先逐图标回退CPU。物品定义解析失败（special/empty/引用缺失）即跳过
///
/// `gui`可选：按已处理的物品数上报进度（约每1%一次）
pub fn render_items(archive: &BaseArchive, gui: ProgressGui) -> CoreResult<()> {
    // 输出目录未初始化时报错
    crate::get_item_dir().ok_or(ErrorType::DownloadFileFail)?;

    let gpu = GpuCtx::try_new();
    let (block_names, item_names) = crate::extract_langs(archive);

    // 枚举jar内全部物品定义
    let defs: Vec<String> = archive
        .entries()
        .iter()
        .filter_map(|e| {
            let rest = e.name.strip_prefix("assets/minecraft/items/")?;
            let name = rest.strip_suffix(".json")?;
            (!e.is_dir && !name.contains('/')).then(|| name.to_string())
        })
        .collect();

    let total = defs.len();
    let done = AtomicUsize::new(0);
    // 每1%左右上报一次，避免高频回调刷爆GUI
    let step = (total / 100).max(1);

    // 与render_blocks相同的工作线程资源：只读jar句柄 + 贴图解码缓存
    thread_local! {
        static LOCAL_ARCHIVE: RefCell<Option<Option<BaseArchive>>> = const { RefCell::new(None) };
        static TEX_CACHE: RefCell<HashMap<String, Bitmap>> = RefCell::new(HashMap::new());
    }

    let rendered: Vec<Option<(String, String)>> = defs
        .par_iter()
        .map(|id| {
            LOCAL_ARCHIVE.with(|local_archive| {
                TEX_CACHE.with(|tex_cache| {
                    let mut tex_cache = tex_cache.borrow_mut();
                    let mut local_archive = local_archive.borrow_mut();
                    let archive = local_archive
                        .get_or_insert_with(|| BaseArchive::open_readonly(archive.path()).ok())
                        .as_ref()
                        .unwrap_or(archive);

                    // 处理完一个计一个（含跳过的），每跨过step阈值上报一次
                    let out = render_item(gpu.as_ref(), archive, &mut tex_cache, id);
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

    // 完成时补一次100%，避免进度停在最后一个step前
    if let Some(gui) = &gui {
        gui.set_progress_now(total, Some(total));
    }

    // 单线程汇总写入物品状态
    let mut items = crate::items_write();
    for (id, out_name) in rendered.into_iter().flatten() {
        let key = format!("minecraft:{id}");
        items.tex.insert(key.clone(), out_name);
        // 物品语言键优先，方块物品（stone等）回落block.minecraft.*
        if let Some(name) = item_names.get(&key).or_else(|| block_names.get(&key)) {
            items.name.insert(key, name.clone());
        }
    }

    Ok(())
}

/// 渲染单个物品图标，返回（物品ID, 输出文件名）
///
/// 公开供单物品诊断渲染（tests/render_one.rs）与render_items复用
pub fn render_item(
    gpu: Option<&GpuCtx>,
    archive: &BaseArchive,
    tex_cache: &mut HashMap<String, Bitmap>,
    id: &str,
) -> Option<(String, String)> {
    let data = archive
        .read(&format!("assets/minecraft/items/{id}.json"))
        .ok()?;
    let def = serialize_tools::json_from_bytes::<ItemDefJson>(&data).ok()?;
    let children = resolve_item_models(def.model.as_ref()?)?;
    if children.is_empty() {
        return None;
    }

    // glint：vanilla 默认注册带附魔光效的物品
    let glint_tex = if GLINT_IDS.contains(&id) {
        load_texture(archive, tex_cache, GLINT_TEX)
    } else {
        None
    };

    // 各子模型烘焙合并（composite）；gui变换与光照取首part
    let mut quads = Vec::new();
    let mut textures: HashMap<String, Bitmap> = HashMap::new();
    let mut base: Option<(bool, GuiTransform)> = None;
    // composite 子模型带 transformation（床等）超出单格，渲染前按投影包围盒适配画布
    let mut oversized = false;
    for child in &children {
        let (part, texs) =
            bake_item_model(archive, &child.model, tex_cache, &child.tints, child.transform)?;
        textures.extend(texs);
        oversized |= child.transform.is_some();
        if base.is_none() {
            base = Some((part.gui_light_3d, part.transform));
        }
        quads.extend(part.quads);
    }
    let (gui_light_3d, transform) = base?;
    let mut model = BakedModel {
        quads,
        gui_light_3d,
        transform,
    };
    if oversized {
        fit_composite(&mut model);
    }

    let data = render_baked(gpu, archive, model, textures, glint_tex.as_ref())?;
    let out_name = format!("minecraft_{id}.png");
    path_helper::write_bytes(&crate::get_item_dir()?.join(&out_name), &data).ok()?;
    Some((id.to_string(), out_name))
}


#[cfg(test)]
mod tests {
    use super::*;

    /// minecraft:model 节点解析：模型名去命名空间、tint 表换算
    #[test]
    fn item_model_parse() {
        let def: ItemDefJson = serialize_tools::json_from_str(
            r#"{"model":{"type":"minecraft:model","model":"minecraft:item/apple",
                "tints":[{"type":"minecraft:constant","value":-1}]}}"#,
        )
        .unwrap();
        let children = resolve_item_models(def.model.as_ref().unwrap()).unwrap();
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].model, "item/apple");
        assert_eq!(children[0].tints, vec![[255, 255, 255]]);
        assert!(children[0].transform.is_none());
    }

    /// select 节点：优先取 when 含 "gui" 的 case，无则 fallback
    #[test]
    fn item_model_select() {
        let def: ItemDefJson = serialize_tools::json_from_str(
            r#"{"model":{"type":"minecraft:select","property":"minecraft:display_context",
                "cases":[{"when":["gui","ground","fixed"],"model":{"type":"minecraft:model",
                "model":"minecraft:item/spyglass"}}],
                "fallback":{"type":"minecraft:model","model":"minecraft:item/spyglass_in_hand"}}}"#,
        )
        .unwrap();
        let children = resolve_item_models(def.model.as_ref().unwrap()).unwrap();
        assert_eq!(children[0].model, "item/spyglass");

        // 无 gui case（trim类）取 fallback
        let def: ItemDefJson = serialize_tools::json_from_str(
            r#"{"model":{"type":"minecraft:select","property":"minecraft:trim_material",
                "cases":[{"when":"minecraft:quartz","model":{"type":"minecraft:model",
                "model":"minecraft:item/leather_chestplate_quartz_trim"}}],
                "fallback":{"type":"minecraft:model","model":"minecraft:item/leather_chestplate"}}}"#,
        )
        .unwrap();
        let children = resolve_item_models(def.model.as_ref().unwrap()).unwrap();
        assert_eq!(children[0].model, "item/leather_chestplate");
    }

    /// composite 子模型带 transformation：矩阵解析 + special/condition 分派
    #[test]
    fn item_model_misc() {
        // special 跳过
        let def: ItemDefJson = serialize_tools::json_from_str(
            r#"{"model":{"type":"minecraft:special","base":"minecraft:item/chest",
                "wrapper":{"type":"minecraft:chest"}}}"#,
        )
        .unwrap();
        assert!(resolve_item_models(def.model.as_ref().unwrap()).is_none());

        // condition 取 on_false
        let def: ItemDefJson = serialize_tools::json_from_str(
            r#"{"model":{"type":"minecraft:condition","component":"minecraft:lodestone_tracker",
                "on_true":{"type":"minecraft:model","model":"minecraft:item/compass_16"},
                "on_false":{"type":"minecraft:model","model":"minecraft:item/compass"}}}"#,
        )
        .unwrap();
        let children = resolve_item_models(def.model.as_ref().unwrap()).unwrap();
        assert_eq!(children[0].model, "item/compass");

        // composite 子模型 transformation → 含平移的矩阵
        let def: ItemDefJson = serialize_tools::json_from_str(
            r#"{"model":{"type":"minecraft:composite","models":[
                {"type":"minecraft:model","model":"minecraft:block/white_bed_head"},
                {"type":"minecraft:model","model":"minecraft:block/white_bed_foot",
                 "transformation":{"translation":[0.0,0.0,1.0],
                 "left_rotation":[0.0,0.0,0.0,1.0],"right_rotation":[0.0,0.0,0.0,1.0],
                 "scale":[1.0,1.0,1.0]}}]}}"#,
        )
        .unwrap();
        let children = resolve_item_models(def.model.as_ref().unwrap()).unwrap();
        assert_eq!(children.len(), 2);
        let m = children[1].transform.expect("foot应有transformation");
        assert!((m.w_axis.z - 1.0).abs() < 1e-5, "foot z平移1");
    }

    /// ARGB int 换算：负数（如 -1 白）与普通色值
    #[test]
    fn argb_rgb_values() {
        assert_eq!(argb_rgb(-1), [255, 255, 255]);
        assert_eq!(argb_rgb(4603950), [0x46, 0x40, 0x2E]);
    }
}
