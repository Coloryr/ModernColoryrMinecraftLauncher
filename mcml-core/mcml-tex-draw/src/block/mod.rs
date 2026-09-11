//! 方块渲染：图标表、数据、渲染管线
pub mod icons;
pub mod obj;

use std::{
    cell::RefCell,
    collections::HashMap,
    io::Cursor,
    slice,
    sync::{LazyLock, atomic::{AtomicU64, AtomicUsize, Ordering}},
};

use crate::block::icons::{BLOCK_ICONS, IconSpec};
use crate::gpu::GpuCtx;
use crate::model::{BakedModel, bake_model};

use mcml_base::{archives::BaseArchive, serialize_tools};
use mcml_game::gui_hook::ProgressGui;
use rayon::prelude::*;
use mcml_names::i18_items::error_type::{CoreResult, ErrorType};
use mcml_sys::path_helper;
use serde::{Deserialize, Serialize};
use skia_safe::{AlphaType, Bitmap, ColorType, Data, IRect, Image, ImageInfo};

/// 输出图片尺寸
const BLOCK_SIZE: i32 = 256;

/// 游戏帧率（用于APNG延迟：frametime / FRAME_RATE 秒）
const FRAME_RATE: u16 = 20;

/// 平原群系颜色（与wiki物品图标一致）：草/树叶等灰度贴图按此染色（tintindex）
pub(crate) const GRASS_TINT: [u8; 3] = [145, 189, 89]; // #91BD59
pub(crate) const FOLIAGE_TINT: [u8; 3] = [119, 171, 47]; // #77AB2F
/// 固定色树叶（游戏内不随群系变化）
pub(crate) const BIRCH_TINT: [u8; 3] = [128, 167, 85]; // #80A755
pub(crate) const SPRUCE_TINT: [u8; 3] = [97, 153, 97]; // #619961

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

/// RGBA像素编码为PNG（尺寸size×size，直通alpha）
fn encode_png(size: u32, rgba: &[u8]) -> Option<Vec<u8>> {
    let mut cursor = Cursor::new(Vec::new());
    {
        let mut encoder = png::Encoder::new(&mut cursor, size, size);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().ok()?;
        writer.write_image_data(rgba).ok()?;
        writer.finish().ok()?;
    }
    Some(cursor.into_inner())
}

/// 合并成APNG（每帧附带显示时长，单位：刻，1刻 = FRAME_RATE分之一秒）
fn encode_apng(size: u32, frames: Vec<(Vec<u8>, u16)>) -> Option<Vec<u8>> {
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


/// textures表的值：老版为字符串，新版可为对象（如glass的force_translucent）
/// （公开供手动测试调试单个方块用）
#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum TextureRefObj {
    Plain(String),
    Object { sprite: String },
}

impl TextureRefObj {
    pub(crate) fn value(&self) -> Option<&str> {
        match self {
            Self::Plain(value) => Some(value),
            Self::Object { sprite } => Some(sprite),
        }
    }
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

/// 渲染一个方块图标（block_icons表的一行）
///
/// - Model：等轴测渲染3D模型（GPU优先，逐图标回退CPU）
/// - IsoModel：无display的模型强制标准gui旋转等轴测渲染
/// - Composite：多模型按transformation拼合（床/门）
/// - Form：特殊形态，按内层规格渲染（ID非真实方块名，注册由render_blocks跳过）
/// - Skip：实体渲染（箱子/头颅/旗帜等），跳过
///
/// 返回（方块ID, 输出文件名）
fn render_icon(
    gpu: Option<&GpuCtx>,
    archive: &BaseArchive,
    tex_cache: &mut HashMap<String, Bitmap>,
    id_rel: &str,
    spec: &IconSpec,
) -> Option<(String, String)> {
    // 表ID带命名空间（minecraft:xx，为mod支持预留），输出文件名把':'换成'_'
    let id = id_rel;
    let out_name = format!("{}.png", id_rel.replace(':', "_"));
    let data = match spec {
        IconSpec::Skip => return None,
        IconSpec::Form(_, inner) => return render_icon(gpu, archive, tex_cache, id_rel, inner),
        IconSpec::Model(rel) => {
            // 烘焙失败（无elements的父模板、贴图引用无值）即跳过
            let (model, textures) = bake_model(archive, rel, tex_cache)?;
            render_baked(gpu, archive, model, textures, None)?
        }
        IconSpec::IsoModel(rel) => {
            // 模型无display变换（游戏创造栏对这些走平面贴图），
            // 按需求强制标准方块gui旋转（block/block的gui display）出等轴测3D观感
            let (mut model, textures) = bake_model(archive, rel, tex_cache)?;
            model.transform = crate::model::GuiTransform {
                rotation: [30.0, 225.0, 0.0],
                translation: [0.0; 3],
                scale: [0.625; 3],
            };
            render_baked(gpu, archive, model, textures, None)?
        }
        IconSpec::Composite(parts) => {
            // 各part独立烘焙，quad按transformation（先绕Y轴旋转再平移）后拼合
            // （0..1空间，1.0 = 1方块），gui变换与光照取首个part（床/门的各part相同）
            let Some((first, rest)) = parts.split_first() else {
                return None;
            };
            let (mut model, mut textures) = bake_model(archive, first.0, tex_cache)?;
            transform_composite_part(&mut model, first.1, first.2);
            for (rel, tr, yaw) in rest {
                let (mut part, texs) = bake_model(archive, rel, tex_cache)?;
                textures.extend(texs);
                transform_composite_part(&mut part, *tr, *yaw);
                model.quads.extend(part.quads);
            }
            // 无display变换的拼合模型（门等）强制标准方块gui旋转，出等轴测3D观感；
            // 有display的（床）保持原变换
            if model.transform.rotation == [0.0, 0.0, 0.0] {
                model.transform.rotation = [30.0, 225.0, 0.0];
            }
            // 多格拼合模型超出单格gui变换范围，按自身投影包围盒适配画布
            fit_composite(&mut model);
            render_baked(gpu, archive, model, textures, None)?
        }
    };

    path_helper::write_bytes(&crate::get_block_dir()?.join(&out_name), &data).ok()?;
    Some((id.to_string(), out_name))
}

/// 渲染烘焙模型出图：贴图带动画（h>w竖排条带）时按时间线逐帧渲染合成APNG，
/// 静态模型单帧出图。动画帧的APNG延迟规则同前（等距采样，相同帧合并延迟）
/// 渲染烘焙模型出图：贴图带动画（h>w竖排条带）时按时间线逐帧渲染合成APNG，
/// 静态模型单帧出图。动画帧的APNG延迟规则同前（等距采样，相同帧合并延迟）
///
/// `glint`：Some(贴图)时叠加附魔光效（仅GPU路径，CPU回退忽略）
pub(crate) fn render_baked(
    gpu: Option<&GpuCtx>,
    archive: &BaseArchive,
    model: BakedModel,
    mut textures: HashMap<String, Bitmap>,
    glint: Option<&Bitmap>,
) -> Option<Vec<u8>> {
    // 动画贴图：quad的uv是单帧空间，时间线从条带展开；先把条带换成首帧供渲染上传
    let mut timelines: Vec<(String, AnimTimeline)> = Vec::new();
    for path in textures.keys().cloned().collect::<Vec<_>>() {
        let strip = textures.get(&path)?;
        if strip.height() > strip.width() {
            let meta = read_anim_meta(archive, &path);
            let tl = AnimTimeline::build(strip, &meta)?;
            let first = extract_frame(strip, 0).unwrap_or_else(|| strip.clone());
            textures.insert(path.clone(), first);
            timelines.push((path, tl));
        }
    }
    if timelines.is_empty() {
        // 静态模型：渲染出的RGBA像素编码为PNG
        return encode_png(BLOCK_SIZE as u32, &render_once(gpu, &model, &textures, glint)?);
    }

    // 最多渲染MAX_APNG_FRAMES帧，对逐刻时间线等距采样，
    // 采样步长代表的持续刻数并入APNG帧延迟；渲染结果相同的相邻帧合并延迟
    let totals: Vec<u32> = timelines.iter().map(|(_, t)| t.total_ticks()).collect();
    let total = totals.iter().copied().max()?;
    const MAX_APNG_FRAMES: usize = 64;
    let stride = total.div_ceil(MAX_APNG_FRAMES as u32);

    let mut frames: Vec<(Vec<u8>, u16)> = Vec::with_capacity((total / stride + 1) as usize);
    let mut k: u32 = 0;
    while k < total {
        for (j, (path, tl)) in timelines.iter().enumerate() {
            let frame = tl.frame_at(k % totals[j])?;
            textures.insert(path.clone(), frame);
        }
        let img = render_once(gpu, &model, &textures, None)?;
        // 该帧在逐刻时间线上代表的刻数，即APNG显示时长
        let ticks = stride.min(total - k) as u16;
        match frames.last_mut() {
            // 渲染结果与上一帧相同：并入其显示时长，不重复存帧
            Some((last_img, last_ticks)) if *last_img == img => *last_ticks += ticks,
            _ => frames.push((img, ticks)),
        }
        k += stride;
    }
    encode_apng(BLOCK_SIZE as u32, frames)
}

/// 多格拼合模型（床等）适配画布：按合并后quad的投影包围盒等比缩放居中（等效旧FitMode::Own）。
/// 单格模型不改gui变换（游戏图标按完整方块尺度渲染，半砖等矮模型不放大）
/// composite part 变换：绕Y轴旋转（门板朝向镜头）后平移；法线同角旋转
fn transform_composite_part(model: &mut BakedModel, tr: [f32; 3], yaw_deg: f32) {
    let (s, c) = yaw_deg.to_radians().sin_cos();
    for quad in &mut model.quads {
        for p in &mut quad.pos {
            let (x, z) = (p[0], p[2]);
            p[0] = x * c + z * s + tr[0];
            p[1] += tr[1];
            p[2] = -x * s + z * c + tr[2];
        }
        let (nx, nz) = (quad.normal[0], quad.normal[2]);
        quad.normal[0] = nx * c + nz * s;
        quad.normal[2] = -nx * s + nz * c;
    }
}

pub(crate) fn fit_composite(model: &mut BakedModel) {
    const MARGIN: f32 = 2.0;
    let slot = BLOCK_SIZE as f32;
    let inner = slot - MARGIN * 2.0;

    // gui变换只取旋转（平移/缩放置零后的线性部分）
    let rot = crate::gpu::item_transform_matrix(&crate::model::GuiTransform {
        rotation: model.transform.rotation,
        translation: [0.0; 3],
        scale: [1.0; 3],
    });

    // 投影全部角点（rot含T(-0.5)，输出即相对画布中心的归一化坐标，y向下）
    let mut min_q = glam::Vec3::splat(f32::INFINITY);
    let mut max_q = glam::Vec3::splat(f32::NEG_INFINITY);
    for quad in &model.quads {
        for v in &quad.pos {
            let p = rot.transform_point3(glam::Vec3::from(*v));
            min_q = min_q.min(p);
            max_q = max_q.max(p);
        }
    }

    // 等比缩放至长边贴合画布，平移使包围盒中心落在画布中心
    let k = (inner / slot) / (max_q.x - min_q.x).max(max_q.y - min_q.y);
    model.transform.scale = [k, k, k];
    model.transform.translation = [
        -k * (min_q.x + max_q.x) / 2.0,
        -k * (min_q.y + max_q.y) / 2.0,
        0.0,
    ];
}

/// 单帧渲染：GPU优先，未成功逐图标回退CPU软件光栅化
fn render_once(
    gpu: Option<&GpuCtx>,
    model: &BakedModel,
    textures: &HashMap<String, Bitmap>,
    glint: Option<&Bitmap>,
) -> Option<Vec<u8>> {
    gpu.and_then(|ctx| ctx.render(model, textures, glint, BLOCK_SIZE as u32))
        .or_else(|| crate::cpu::render_cpu(model, textures, BLOCK_SIZE as u32))
}

/// 按创造模式图标表渲染全部方块
///
/// 渲染清单与分类来自block_icons::BLOCK_ICONS（生成脚本从26.2客户端jar一次性
/// 提取的固定表：items/*.json图标定义 + CreativeModeTabs创造分类，之后手工维护）。
/// 下载流程（版本清单 → 客户端jar）见lib的load_blocks，这里只做解包渲染。
///
/// 渲染按CPU核数并发（rayon）：各工作线程经thread_local持有一份只读jar句柄
/// 与跨方块复用的贴图缓存，完成后单线程汇总写入方块状态
///
/// `gui`可选：按已处理的表条目数上报进度（约每1%一次）
pub fn render_blocks(archive: &BaseArchive, gui: ProgressGui) -> CoreResult<()> {
    let render_start = std::time::Instant::now();
    // 输出目录未初始化时报错（各图标写盘前也各自取一次）
    crate::get_block_dir().ok_or(ErrorType::DownloadFileFail)?;

    // GPU上下文按平台条件编译自动选择后端（DX12→VK→GL等链），
    // 创建一次跨线程复用；全不可用时render_icon内逐图标回退CPU
    let gpu = GpuCtx::try_new();

    // 提取语言文件并构建 ID->语言键 映射
    let (names, _) = crate::extract_langs(archive);

    let total = BLOCK_ICONS.len();
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
    let rendered: Vec<Option<(String, String, &str)>> = BLOCK_ICONS
        .par_iter()
        .map(|(id_rel, cat, spec)| {
            LOCAL_ARCHIVE.with(|local_archive| {
                TEX_CACHE.with(|tex_cache| {
                    let mut tex_cache = tex_cache.borrow_mut();
                    let mut local_archive = local_archive.borrow_mut();
                    let archive = local_archive
                        .get_or_insert_with(|| BaseArchive::open_readonly(archive.path()).ok())
                        .as_ref()
                        .unwrap_or(archive);

                    // 单个表条目的处理（进度计数对跳过的条目也要生效）
                    let item_start = std::time::Instant::now();
                    let out = render_icon(gpu.as_ref(), archive, &mut tex_cache, id_rel, spec);

                    // 定位异常慢的单方块（如超大动画贴图）
                    if PROFILE.enabled {
                        let secs = item_start.elapsed().as_secs_f64();
                        if secs > 1.0 {
                            println!("[性能] 慢方块 {id_rel}：{secs:.1}s");
                        }
                    }

                    // 处理完一个计一个（含跳过的），每跨过step阈值上报一次
                    let now = done.fetch_add(1, Ordering::Relaxed) + 1;
                    if now % step == 0
                        && let Some(gui) = &gui
                    {
                        gui.set_progress_now(now, Some(total));
                    }

                    // Form形态：只出图不注册（ID非真实方块名，不进blocks()列表）
                    out.filter(|_| !matches!(spec, IconSpec::Form(..)))
                        .map(|(id, out_name)| (id, out_name, *cat))
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
    for (id, out_name, cat) in rendered.into_iter().flatten() {
        blocks.tex.insert(id.clone(), out_name);
        blocks.cat.insert(id.clone(), cat.to_string());
        if let Some(lang_key) = names.get(&id) {
            blocks.name.insert(id, lang_key.clone());
        }
    }

    Ok(())
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
mod icon_table_tests {
    use super::*;
    use std::collections::HashSet;

    use crate::block::icons::{SpecialForm, special_form};

    /// 固定表完整性：方块ID唯一、分类非空、各规格内容非空
    #[test]
    fn icon_table_consistent() {
        assert!(BLOCK_ICONS.len() > 800, "表应覆盖全部创造栏方块");
        let mut seen = HashSet::new();
        for (id, cat, spec) in BLOCK_ICONS {
            assert!(seen.insert(*id), "重复方块ID：{id}");
            assert!(!cat.is_empty(), "{id} 分类为空");
            match spec {
                IconSpec::Model(rel) | IconSpec::IsoModel(rel) => {
                    assert!(!rel.is_empty(), "{id} 模型名为空")
                }
                IconSpec::Composite(parts) => assert!(!parts.is_empty(), "{id} composite无part"),
                // Form形态：ID必须以形态后缀结尾，内层规格不能是Skip/Form
                IconSpec::Form(form, inner) => {
                    assert!(
                        id.ends_with(&format!("_{}", form.suffix())),
                        "{id} 形态后缀与SpecialForm不符"
                    );
                    assert!(
                        matches!(
                            inner,
                            IconSpec::Model(_) | IconSpec::IsoModel(_) | IconSpec::Composite(_)
                        ),
                        "{id} Form内层规格非法"
                    );
                }
                IconSpec::Skip => {}
            }
        }
    }

    /// 抽查关键行：渲染规格与分类应与游戏创造栏一致
    #[test]
    fn icon_table_spot_checks() {
        let table: HashMap<&str, (&str, &IconSpec)> =
            BLOCK_ICONS.iter().map(|(id, cat, spec)| (*id, (*cat, spec))).collect();
        assert!(matches!(table["minecraft:stone"].1, IconSpec::Model("block/stone")));
        assert_eq!(table["minecraft:stone"].0, "buildingBlocks");
        assert!(matches!(table["minecraft:white_bed"].1, IconSpec::Composite(_)));
        // special实体渲染跳过
        assert!(matches!(table["minecraft:chest"].1, IconSpec::Skip));
        assert!(matches!(table["minecraft:skeleton_skull"].1, IconSpec::Skip));
        // 原木在游戏的建筑方块栏
        assert_eq!(table["minecraft:oak_log"].0, "buildingBlocks");
        assert_eq!(table["minecraft:grass_block"].0, "natural");
    }

    /// 特殊形态：基础ID应能查到形态，且对应"基础ID_后缀"图标确实在表里
    #[test]
    fn special_form_lookup() {
        assert_eq!(special_form("minecraft:furnace"), Some(SpecialForm::Lit));
        assert_eq!(special_form("minecraft:redstone_lamp"), Some(SpecialForm::Lit));
        assert_eq!(special_form("minecraft:oak_button"), Some(SpecialForm::Pressed));
        assert_eq!(special_form("minecraft:oak_pressure_plate"), Some(SpecialForm::Down));
        assert_eq!(special_form("minecraft:oak_door"), Some(SpecialForm::Open));
        assert_eq!(special_form("minecraft:piston"), Some(SpecialForm::Extended));
        assert_eq!(special_form("minecraft:vault"), Some(SpecialForm::Active));
        assert_eq!(special_form("minecraft:campfire"), Some(SpecialForm::Off));
        // 无特殊形态的方块
        assert_eq!(special_form("minecraft:stone"), None);
        assert_eq!(special_form("minecraft:oak_stairs"), None);

        // 每个查到的形态都应有对应后缀图标，且图标ID后缀与形态匹配
        for (id, cat, _) in BLOCK_ICONS {
            if let Some(form) = special_form(id) {
                let alt = format!("{id}_{}", form.suffix());
                let hit = BLOCK_ICONS
                    .iter()
                    .any(|(name, c, spec)| *name == alt && matches!(spec, IconSpec::Form(..)));
                assert!(hit, "形态图标缺失：{alt}（{cat}）");
            }
        }
    }
}

#[cfg(test)]
mod sprite_tests {
    use super::*;

    /// 构造竖排动画条带（n帧16x16，每帧灰度渐变便于区分）
    fn anim_strip(frames: usize) -> Bitmap {
        let info = ImageInfo::new(
            (16, 16 * frames as i32),
            ColorType::RGBA8888,
            AlphaType::Premul,
            None,
        );
        let mut bm = Bitmap::new();
        assert!(bm.set_info(&info, None));
        bm.alloc_pixels();
        let size = bm.compute_byte_size();
        let px = unsafe { slice::from_raw_parts_mut(bm.pixels() as *mut u8, size) };
        for (i, px_chunk) in px.chunks_exact_mut(4).enumerate() {
            // 每帧整体一种不透明颜色，帧间可区分（去重不会合并）
            let frame = (i / (16 * 16)) as u8;
            px_chunk.copy_from_slice(&[frame * 60, 128, 64, 255]);
        }
        bm
    }

    /// 动画条带时间线逐帧取图 + APNG编码（render_baked的核心环节）
    #[test]
    fn sprite_apng_smoke() {
        let strip = anim_strip(4);
        let meta = AnimMeta {
            frametime: 20,
            interpolate: true,
            frames: None,
        };
        // 逐步定位失败环节
        let tl = AnimTimeline::build(&strip, &meta);
        assert!(tl.is_some(), "timeline构建失败");
        let tl = tl.unwrap();
        assert_eq!(tl.total_ticks(), 80, "total_ticks应=80");
        let mut frames = Vec::new();
        for k in (0..tl.total_ticks()).step_by(20) {
            let bm = tl.frame_at(k).expect("frame_at应成功");
            let (px, _, w, h) = crate::model::bitmap_rgba(&bm).expect("位图读出应成功");
            assert_eq!((w, h), (16, 16));
            frames.push((px, 20u16));
        }
        let data = encode_apng(16, frames);
        assert!(data.is_some(), "APNG应成功");
        // 16×16纯色帧压缩后很小，仅验证编码产物有效
        assert!(data.unwrap().len() > 100, "APNG应有实际内容");
    }
}
