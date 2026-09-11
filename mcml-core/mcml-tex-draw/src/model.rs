//! 26.2 模型 JSON → 渲染 quad 的精确烘焙（照反编译 FaceBakery/CuboidRotation/ItemTransform 规则）
//!
//! 顶点 0..1 模型空间（FaceBakery.bakeVertex 的 div(16)），gui display 变换由渲染后端在
//! shader 矩阵里叠加；quad 只携带几何/uv/法线/贴图名/tint 色/透明度分组。

use std::collections::HashMap;

use glam::{Mat4, Vec3};
use mcml_base::{archives::BaseArchive, serialize_tools};
use serde::Deserialize;
use skia_safe::Bitmap;

use crate::block::{
    decode_png, read_anim_meta, AnimMeta, TextureRefObj, BIRCH_TINT, FOLIAGE_TINT, GRASS_TINT,
    SPRUCE_TINT,
};

/// 一个渲染 quad（0..1 模型空间）
#[derive(Clone)]
pub struct Quad {
    /// 4 个顶点位置（角序照 FaceInfo，从面外侧看逆时针）
    pub pos: [[f32; 3]; 4],
    /// 4 个顶点 uv（0..1，v=0 为贴图顶行）
    pub uv: [[f32; 2]; 4],
    /// 光照法线（calculateFacing 量化后的轴向单位向量）
    pub normal: [f32; 3],
    /// 4 个顶点色（tint 后 RGBA，0..1）
    pub color: [[f32; 4]; 4],
    /// 蕐要使用的贴图相对路径
    pub tex: String,
    /// uv 矩形内含半透明像素 → translucent pass
    pub translucent: bool,
    /// 自发光（火焰等）：不参与方向光照，全亮渲染
    pub fullbright: bool,
}

/// 一个完整模型的烘焙产物
pub struct BakedModel {
    pub quads: Vec<Quad>,
    /// gui_light：true=side → ITEMS_3D 光照；false=front → ITEMS_FLAT
    pub gui_light_3d: bool,
    /// gui display 变换（ItemTransform，缺省即 NO_TRANSFORM）
    pub transform: GuiTransform,
}

/// gui display 变换参数（照 ItemTransform 语义，translation 已 ×0.0625）
#[derive(Clone, Copy)]
pub struct GuiTransform {
    pub rotation: [f32; 3],
    pub translation: [f32; 3],
    pub scale: [f32; 3],
}

impl Default for GuiTransform {
    /// NO_TRANSFORM：rotation/translation 零、scale 一
    fn default() -> Self {
        Self {
            rotation: [0.0; 3],
            translation: [0.0; 3],
            scale: [1.0; 3],
        }
    }
}

/// FaceInfo：六个面的 4 个顶点角（extent 枚举：0..5 = MIN_X..MAX_Z）
/// FaceInfo：六个面的 4 个顶点角，坐标选择器 0=MIN_X 1=MIN_Y 2=MIN_Z 3=MAX_X 4=MAX_Y 5=MAX_Z
/// （照抄 net.minecraft.client.renderer.FaceInfo 的六个枚举常量）
pub(crate) const FACE_INFO: [[(u8, u8, u8); 4]; 6] = [
    // down: (MIN_X,MIN_Y,MAX_Z) (MIN_X,MIN_Y,MIN_Z) (MAX_X,MIN_Y,MIN_Z) (MAX_X,MIN_Y,MAX_Z)
    [(0, 1, 5), (0, 1, 2), (3, 1, 2), (3, 1, 5)],
    // up: (MIN_X,MAX_Y,MIN_Z) (MIN_X,MAX_Y,MAX_Z) (MAX_X,MAX_Y,MAX_Z) (MAX_X,MAX_Y,MIN_Z)
    [(0, 4, 2), (0, 4, 5), (3, 4, 5), (3, 4, 2)],
    // north: (MAX_X,MAX_Y,MIN_Z) (MAX_X,MIN_Y,MIN_Z) (MIN_X,MIN_Y,MIN_Z) (MIN_X,MAX_Y,MIN_Z)
    [(3, 4, 2), (3, 1, 2), (0, 1, 2), (0, 4, 2)],
    // south: (MIN_X,MAX_Y,MAX_Z) (MIN_X,MIN_Y,MAX_Z) (MAX_X,MIN_Y,MAX_Z) (MAX_X,MAX_Y,MAX_Z)
    [(0, 4, 5), (0, 1, 5), (3, 1, 5), (3, 4, 5)],
    // west: (MIN_X,MAX_Y,MIN_Z) (MIN_X,MIN_Y,MIN_Z) (MIN_X,MIN_Y,MAX_Z) (MIN_X,MAX_Y,MAX_Z)
    [(0, 4, 2), (0, 1, 2), (0, 1, 5), (0, 4, 5)],
    // east: (MAX_X,MAX_Y,MAX_Z) (MAX_X,MIN_Y,MAX_Z) (MAX_X,MIN_Y,MIN_Z) (MAX_X,MAX_Y,MIN_Z)
    [(3, 4, 5), (3, 1, 5), (3, 1, 2), (3, 4, 2)],
];

/// 面名 → FACE_INFO 下标（与游戏 Direction 枚举序一致）
const FACE_INDEX: [(&str, usize); 6] = [
    ("down", 0),
    ("up", 1),
    ("north", 2),
    ("south", 3),
    ("west", 4),
    ("east", 5),
];

/// defaultFaceUV（照 FaceBakery.defaultFaceUV）：面无显式 uv 时从盒子边界投影
fn default_face_uv(facing: usize, from: &[f32], to: &[f32]) -> [f32; 4] {
    match facing {
        0 => [from[0], 16.0 - to[2], to[0], 16.0 - from[2]],   // down
        1 => [from[0], from[2], to[0], to[2]],                 // up
        2 => [16.0 - to[0], 16.0 - to[1], 16.0 - from[0], 16.0 - from[1]], // north
        3 => [from[0], 16.0 - to[1], to[0], 16.0 - from[1]],   // south
        4 => [from[2], 16.0 - to[1], to[2], 16.0 - from[1]],   // west
        _ => [16.0 - to[2], 16.0 - to[1], 16.0 - from[2], 16.0 - from[1]], // east
    }
}

/// 元素 rotation 的变换矩阵（照 CuboidRotation）：绕 origin 旋转（origin 已 /16），
/// rescale 时每轴缩放 1/max|R·axis|（先缩放后旋转：M = R·S）
fn element_rotation_matrix(origin: [f32; 3], axis: usize, angle_deg: f32, rescale: bool) -> Mat4 {
    let angle = angle_deg.to_radians();
    let rot = match axis {
        0 => Mat4::from_rotation_x(angle),
        1 => Mat4::from_rotation_y(angle),
        _ => Mat4::from_rotation_z(angle),
    };
    let mut rot = rot;
    if rescale {
        let mut s = [1.0f32; 3];
        for (i, item) in s.iter_mut().enumerate() {
            let unit = match i {
                0 => Vec3::X,
                1 => Vec3::Y,
                _ => Vec3::Z,
            };
            let t = rot.transform_vector3(unit);
            *item = 1.0 / t.abs().max_element();
        }
        rot = rot * Mat4::from_scale(Vec3::from_slice(&s));
    }
    let origin = Vec3::from(origin);
    Mat4::from_translation(origin) * rot * Mat4::from_translation(-origin)
}


/// ===== 模型 JSON 结构（自带完整 display 解析，26.2 语义） =====

#[derive(Deserialize)]
struct ModelJson {
    #[serde(default)]
    parent: Option<String>,
    #[serde(default)]
    textures: HashMap<String, TextureRefObj>,
    #[serde(default)]
    elements: Option<Vec<ElementJson>>,
    #[serde(default)]
    display: Option<DisplayJson>,
    /// gui_light：side（默认，双灯3D）/ front（单面平光）
    #[serde(default)]
    gui_light: Option<String>,
}

#[derive(Deserialize)]
struct DisplayJson {
    #[serde(default)]
    gui: Option<TransformJson>,
}

/// gui display 变换（ItemTransform 反序列化语义：translation×0.0625后clamp±5，scale clamp±4）
#[derive(Deserialize)]
struct TransformJson {
    #[serde(default)]
    rotation: Vec<f32>,
    #[serde(default)]
    translation: Vec<f32>,
    #[serde(default)]
    scale: Vec<f32>,
}

impl TransformJson {
    /// 反序列化即完成 clamp（照 ItemTransform.Deserializer）
    fn resolved(&self) -> GuiTransform {
        let take = |v: &Vec<f32>, i: usize| v.get(i).copied().unwrap_or(0.0);
        let mut translation = [
            take(&self.translation, 0) * 0.0625,
            take(&self.translation, 1) * 0.0625,
            take(&self.translation, 2) * 0.0625,
        ];
        for t in &mut translation {
            *t = t.clamp(-5.0, 5.0);
        }
        let scale = |i: usize| {
            // 未写scale整体为1；写了的按分量缺省1，clamp±4（允许负值，照Mth.clamp）
            if self.scale.is_empty() {
                1.0
            } else {
                self.scale.get(i).copied().unwrap_or(1.0).clamp(-4.0, 4.0)
            }
        };
        GuiTransform {
            rotation: [take(&self.rotation, 0), take(&self.rotation, 1), take(&self.rotation, 2)],
            translation,
            scale: [scale(0), scale(1), scale(2)],
        }
    }
}

#[derive(Clone, Deserialize)]
pub(crate) struct ElementJson {
    #[serde(default = "d_from")]
    from: Vec<f32>,
    #[serde(default = "d_to")]
    to: Vec<f32>,
    #[serde(default)]
    faces: Option<HashMap<String, FaceJson>>,
    #[serde(default)]
    rotation: Option<RotationJson>,
}

fn d_from() -> Vec<f32> {
    vec![0.0, 0.0, 0.0]
}

fn d_to() -> Vec<f32> {
    vec![16.0, 16.0, 16.0]
}

#[derive(Clone, Deserialize)]
struct RotationJson {
    #[serde(default = "d_rot_origin")]
    origin: Vec<f32>,
    #[serde(default)]
    axis: Option<String>,
    #[serde(default)]
    angle: Option<f32>,
    #[serde(default)]
    rescale: Option<bool>,
}

fn d_rot_origin() -> Vec<f32> {
    vec![8.0, 8.0, 8.0]
}

#[derive(Clone, Deserialize)]
struct FaceJson {
    #[serde(default)]
    texture: Option<String>,
    #[serde(default)]
    uv: Option<Vec<f32>>,
    /// 0/90/180/270
    #[serde(default)]
    rotation: Option<u32>,
    #[serde(default)]
    tintindex: Option<u32>,
}


/// ===== 模型解析与 quad 烘焙（照 FaceBakery/ResolvedModel.findTop 语义） =====

/// 父链解析产物：合并后的贴图表、元素集、gui display、gui_light
pub(crate) struct Resolved {
    pub(crate) textures: HashMap<String, TextureRefObj>,
    pub(crate) elements: Vec<ElementJson>,
    pub(crate) transform: GuiTransform,
    pub(crate) gui_light_3d: bool,
    /// 链终止于 builtin/generated（平面物品，无模型文件，走extrude挤出）
    pub(crate) generated: bool,
}

/// 沿 parent 链向上收集：textures 逐层合并（子优先）、elements 取第一个带定义的祖先、
/// display.gui 与 gui_light 取离当前模型最近的祖先
/// （findTop 走完整条链：block/block 的 gui 旋转在 cube 找到 elements 之后仍在更上层）
pub(crate) fn resolve(archive: &BaseArchive, rel: &str) -> Option<Resolved> {
    let mut textures: HashMap<String, TextureRefObj> = HashMap::new();
    let mut transform: Option<GuiTransform> = None;
    let mut gui_light: Option<bool> = None;
    let mut elements: Option<Vec<ElementJson>> = None;
    let mut generated = false;
    let mut current = rel.to_string();
    for _ in 0..16 {
        // builtin/generated 是物品生成器标记（extrude几何），无模型文件，到此为止
        if current == "builtin/generated" {
            generated = true;
            break;
        }
        let data = archive
            .read(&format!("assets/minecraft/models/{current}.json"))
            .ok()?;
        let model = serialize_tools::json_from_bytes::<ModelJson>(&data).ok()?;

        // 子模型同名键覆盖父模板（"#up"等自引用占位必须被子模型的实际贴图替换，
        // 如template_bed_head的east:"#east"由white_bed_head的east覆盖）
        let mut merged = model.textures;
        for (k, v) in textures {
            merged.insert(k, v);
        }
        textures = merged;
        if transform.is_none() {
            if let Some(t) = model.display.and_then(|d| d.gui) {
                transform = Some(t.resolved());
            }
        }
        if gui_light.is_none() {
            if let Some(l) = model.gui_light {
                gui_light = Some(l == "front");
            }
        }
        if elements.is_none() {
            elements = model.elements;
        }

        let Some(parent) = model.parent else {
            break;
        };
        current = parent.strip_prefix("minecraft:").unwrap_or(&parent).to_string();
    }
    Some(Resolved {
        textures,
        // generated链（builtin/generated）无elements也有效（extrude路径）；
        // 普通链缺elements视为无效（父模板）
        elements: elements.filter(|e| !e.is_empty() || generated).unwrap_or_default(),
        transform: transform.unwrap_or_default(),
        gui_light_3d: !gui_light.unwrap_or(false),
        generated,
    })
}

/// 解析贴图引用链（"#side" -> textures值，值可能还是引用）
fn resolve_ref(textures: &HashMap<String, TextureRefObj>, r: &str) -> Option<String> {
    let mut r = r;
    for _ in 0..8 {
        let rest = r.strip_prefix('#').unwrap_or(r);
        match textures.get(rest) {
            Some(v) => r = v.value()?,
            None if r.starts_with('#') => return None,
            None => return Some(r.to_string()),
        }
    }
    None
}

/// 贴图引用值 -> jar内路径
pub(crate) fn texture_path(value: &str) -> Option<String> {
    let (ns, path) = value.split_once(':').unwrap_or(("minecraft", value));
    if ns != "minecraft" {
        return None;
    }
    Some(format!("assets/minecraft/textures/{path}.png"))
}


/// 加载贴图（带缓存），返回 RGBA 位图
pub(crate) fn load_texture(
    archive: &BaseArchive,
    cache: &mut HashMap<String, Bitmap>,
    path: &str,
) -> Option<Bitmap> {
    if let Some(tex) = cache.get(path) {
        return Some(tex.clone());
    }
    let bytes = archive.read(path).ok()?;
    let tex = decode_png(&bytes)?;
    cache.insert(path.to_string(), tex.clone());
    Some(tex)
}

/// 读取位图为 straight-alpha RGBA（read_pixels 顺带完成 premul->unpremul）
pub(crate) fn bitmap_rgba(tex: &Bitmap) -> Option<(Vec<u8>, usize, usize, usize)> {
    let (w, h) = (tex.width(), tex.height());
    let info = skia_safe::ImageInfo::new(
        (w, h),
        skia_safe::ColorType::RGBA8888,
        skia_safe::AlphaType::Unpremul,
        None,
    );
    let row = info.min_row_bytes() as usize;
    let mut buf = vec![0u8; row * h as usize];
    let ok = tex.as_image().read_pixels(
        &info,
        &mut buf,
        row as usize,
        (0, 0),
        skia_safe::image::CachingHint::Disallow,
    );
    ok.then_some((buf, row, w as usize, h as usize))
}

/// 按 NativeImage.computeTransparency 扫 uv 矩形：仅当含 1..254 之间的 alpha
/// 才算 translucent（alpha==0 只是镂空，仍走cutout管线）；
/// x0=floor(u0·w) y0=floor(v0·h) x1=ceil(u1·w) y1=ceil(v1·h)，动画贴图所有唯一帧合并判定
pub(crate) fn rect_translucent(tex: &Bitmap, anim: Option<&AnimMeta>, uv: &[f32; 4]) -> bool {
    let Some((base, stride, w, h_total)) = bitmap_rgba(tex) else {
        return false;
    };
    if w == 0 || h_total == 0 {
        return false;
    }
    // 动画贴图按方形帧竖向堆叠（帧高=宽），uv矩形以单帧归一化
    let frame_h = if anim.is_some() { w } else { h_total };
    let frame_count = (h_total / frame_h).max(1);
    let frames: Vec<u32> = match anim {
        Some(meta) => {
            let mut seen = Vec::new();
            let list = meta
                .frames
                .as_ref()
                .map(|list| list.iter().map(|f| f.index).collect::<Vec<_>>());
            for i in list.unwrap_or_else(|| (0..frame_count as u32).collect()) {
                let idx = i % frame_count as u32;
                if !seen.contains(&idx) {
                    seen.push(idx);
                }
            }
            if seen.is_empty() {
                vec![0]
            } else {
                seen
            }
        }
        None => vec![0],
    };

    let x0 = (uv[0] * w as f32).floor() as usize;
    let y0 = (uv[1] * frame_h as f32).floor() as usize;
    let x1 = ((uv[2] * w as f32).ceil() as usize).min(w);
    let y1 = ((uv[3] * frame_h as f32).ceil() as usize).min(frame_h);
    if x0 >= x1 || y0 >= y1 {
        return false;
    }
    for &frame in &frames {
        let fy = frame as usize * frame_h;
        for y in y0..y1 {
            let row = fy + y;
            if row >= h_total {
                break;
            }
            for x in x0..x1 {
                let a = base[row * stride + x * 4 + 3];
                // 与游戏一致：a==0为镂空（cutout），仅1..254算半透明
                if a > 0 && a < 255 {
                    return true;
                }
            }
        }
    }
    false
}

/// tintindex -> 染色（灰度贴图按群系色，与游戏平原群系常量色一致）
fn tint_for_path(path: &str) -> [u8; 3] {
    if path.contains("birch_leaves") {
        BIRCH_TINT
    } else if path.contains("spruce_leaves") {
        SPRUCE_TINT
    } else if path.contains("leaves") {
        FOLIAGE_TINT
    } else {
        GRASS_TINT
    }
}


/// ===== bake：照 FaceBakery.bakeQuad 逐条对应 =====

/// 取盒子边界分量（extent 枚举 -> from/to 的分量）
pub(crate) fn select_extent(extent: u8, from: &[f32], to: &[f32]) -> f32 {
    match extent {
        0 => from[0],
        1 => from[1],
        2 => from[2],
        3 => to[0],
        4 => to[1],
        _ => to[2],
    }
}

/// 面索引 -> 法线轴（与 FACE_INDEX 一致：down=0..east=5）
pub(crate) fn face_normal(i: usize) -> Vec3 {
    match i {
        0 => Vec3::NEG_Y,
        1 => Vec3::Y,
        2 => Vec3::NEG_Z,
        3 => Vec3::Z,
        4 => Vec3::NEG_X,
        _ => Vec3::X,
    }
}

/// 法线 -> 面索引（点积最接近1的轴）
pub(crate) fn face_index_of(dir: Vec3) -> Option<usize> {
    (0..6).find(|&i| dir.dot(face_normal(i)) > 0.999)
}

/// calculateFacing：由前三顶点叉积求几何法线，量化到点积最大的正向坐标轴；
/// 退化面返回 None（游戏回退 Direction.UP，等价零法线）
pub(crate) fn calculate_facing(pos: &[[f32; 3]; 4]) -> Option<[f32; 3]> {
    let p0 = Vec3::from(pos[0]);
    let p1 = Vec3::from(pos[1]);
    let p2 = Vec3::from(pos[2]);
    let normal = (p1 - p0).cross(p2 - p0);
    if !normal.is_finite() {
        return None;
    }
    let mut best: Option<(f32, [f32; 3])> = None;
    for candidate in [
        Vec3::NEG_X, Vec3::NEG_Y, Vec3::NEG_Z, Vec3::X, Vec3::Y, Vec3::Z,
    ] {
        let product = normal.dot(candidate);
        if product >= 0.0 && product > best.map_or(0.0, |(c, _): (f32, [f32; 3])| c) {
            best = Some((product, candidate.to_array()));
        }
    }
    best.map(|(_, dir)| dir)
}

/// recalculateWinding：把顶点顺序换成该朝向 FaceInfo 的规范角序（uv随之同步交换）
pub(crate) fn recalculate_winding(pos: &mut [[f32; 3]; 4], uv: &mut [[f32; 2]; 4], facing: usize) {
    let mut min = [999.0f32; 3];
    let mut max = [-999.0f32; 3];
    for p in pos.iter() {
        for c in 0..3 {
            min[c] = min[c].min(p[c]);
            max[c] = max[c].max(p[c]);
        }
    }
    for vertex in 0..4 {
        let info = FACE_INFO[facing][vertex];
        let want: [f32; 3] = [
            select_extent(info.0, &min, &max),
            select_extent(info.1, &min, &max),
            select_extent(info.2, &min, &max),
        ];
        let found = (vertex..4).find(|&i| {
            (0..3).all(|c| (pos[i][c] - want[c]).abs() < 1e-5)
        });
        if let Some(i) = found {
            if i != vertex {
                pos.swap(i, vertex);
                uv.swap(i, vertex);
            }
        }
    }
}

/// 烘焙一个元素的全部面 -> quad 列表
///
/// `item_tints`：None=方块路径（tintindex按贴图路径取群系常量色）；
/// Some=物品路径（tintindex索引items/*.json的tint表，越界/无tintindex为白色，与游戏一致）
pub(crate) fn bake_element(
    element: &ElementJson,
    textures: &HashMap<String, TextureRefObj>,
    archive: &BaseArchive,
    tex_cache: &mut HashMap<String, Bitmap>,
    item_tints: Option<&[[u8; 3]]>,
) -> Vec<Quad> {
    let mut quads = Vec::new();
    let Some(faces) = &element.faces else {
        return quads;
    };
    for (name, face) in faces {
        let Some(facing) = FACE_INDEX
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, i)| *i)
        else {
            continue;
        };
        // uv：显式或 defaultFaceUV
        let uvs: [f32; 4] = match &face.uv {
            Some(v) if v.len() == 4 => [v[0], v[1], v[2], v[3]],
            _ => default_face_uv(facing, &element.from, &element.to),
        };

        // 元素旋转（origin为0..16坐标，矩阵内已/16）
        let elem_rot = element.rotation.as_ref().map(|r| {
            let axis = match r.axis.as_deref() {
                Some("x") => 0usize,
                Some("z") => 2usize,
                _ => 1usize,
            };
            element_rotation_matrix(
                [r.origin[0] / 16.0, r.origin[1] / 16.0, r.origin[2] / 16.0],
                axis,
                r.angle.unwrap_or(0.0),
                r.rescale.unwrap_or(false),
            )
        });

        // 四顶点位置与uv（bakeVertex：角选点/16 -> 元素旋转）
        let corner = |i: usize| -> [f32; 3] {
            let info = FACE_INFO[facing][i];
            [
                select_extent(info.0, &element.from, &element.to) / 16.0,
                select_extent(info.1, &element.from, &element.to) / 16.0,
                select_extent(info.2, &element.from, &element.to) / 16.0,
            ]
        };
        let mut pos: [[f32; 3]; 4] = std::array::from_fn(corner);
        let mut uv: [[f32; 2]; 4] = std::array::from_fn(|i| {
            // CuboidFace.getU/getV：角i对应UVs下标 (i + rotation/90) % 4
            let idx = ((i as u32 + face.rotation.unwrap_or(0) / 90) % 4) as usize;
            let u = if idx == 0 || idx == 1 { uvs[0] } else { uvs[2] };
            let v = if idx == 0 || idx == 3 { uvs[1] } else { uvs[3] };
            [u / 16.0, v / 16.0]
        });
        if let Some(m) = &elem_rot {
            for p in &mut pos {
                let v = m.transform_point3(Vec3::from(*p));
                *p = v.to_array();
            }
        }

        // 法线量化 + 规范角序（无元素旋转时才做winding重排）
        let facing_final = calculate_facing(&pos).and_then(|dir| face_index_of(Vec3::from(dir)));
        if elem_rot.is_none() {
            if let Some(f) = facing_final {
                recalculate_winding(&mut pos, &mut uv, f);
            }
        }

        // 贴图与透明度
        let Some(value) = face.texture.as_deref().and_then(|r| resolve_ref(textures, r)) else {
            continue;
        };
        let Some(path) = texture_path(&value) else {
            continue;
        };
        let Some(tex) = load_texture(archive, tex_cache, &path) else {
            continue;
        };
        let anim = read_anim_meta(archive, &path);

        // tint：方块路径按贴图路径取群系常量色，物品路径索引items/*.json的tint表
        let tint = match item_tints {
            None => face.tintindex.map(|_| tint_for_path(&path)),
            Some(list) => face.tintindex.and_then(|i| list.get(i as usize).copied()),
        }
        .unwrap_or([255, 255, 255]);
        let color = [
            tint[0] as f32 / 255.0,
            tint[1] as f32 / 255.0,
            tint[2] as f32 / 255.0,
            1.0,
        ];

        quads.push(Quad {
            pos,
            uv,
            normal: facing_final
                .map(|f| face_normal(f).to_array())
                .unwrap_or([0.0, 1.0, 0.0]),
            color: [color; 4],
            tex: path.clone(),
            translucent: rect_translucent(
                &tex,
                Some(&anim),
                &[uvs[0] / 16.0, uvs[1] / 16.0, uvs[2] / 16.0, uvs[3] / 16.0],
            ),
            // 火焰贴图自发光（篝火火焰在游戏里不受方向光变暗）
            fullbright: is_fullbright_tex(&path),
        });
    }
    quads
}


/// 烘焙一个模型为 quad 列表（含贴图缓存），供 GPU/CPU 两种后端共用
/// 自发光贴图：篝火/灵魂篝火的火焰，渲染时不做方向光衰减
fn is_fullbright_tex(path: &str) -> bool {
    // 路径带.png后缀；soul_campfire_fire 也包含 campfire_fire
    path.contains("campfire_fire")
}

pub fn bake_model(
    archive: &BaseArchive,
    rel: &str,
    tex_cache: &mut HashMap<String, Bitmap>,
) -> Option<(BakedModel, HashMap<String, Bitmap>)> {
    let resolved = resolve(archive, rel)?;
    let mut quads = Vec::new();
    for element in &resolved.elements {
        quads.extend(bake_element(element, &resolved.textures, archive, tex_cache, None));
    }
    if quads.is_empty() {
        return None;
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
            gui_light_3d: resolved.gui_light_3d,
            transform: resolved.transform,
        },
        used_textures,
    ))
}
