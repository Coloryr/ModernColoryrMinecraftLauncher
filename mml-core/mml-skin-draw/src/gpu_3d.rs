//! 3D 渲染公共管线（wgpu 离屏）：头 / 皮肤 3D 共用的顶点格式、管线与渲染流程
//!
//! 设备来自全局 [`mml_gpu::device`]（后端回退链见其文档）；管线进程内只建一次。
//! 像素约定：贴图上传 premul BGRA → straight RGBA，读回 premul RGBA → premul
//! BGRA（`Pixmap` 内存序，仅交换 R/B），全程预乘语义与 tiny_skia 一致。

use std::sync::OnceLock;

use bytemuck::{Pod, Zeroable};
use glam::Mat4;
use tiny_skia::Pixmap;
use wgpu::util::DeviceExt;

use crate::skin_draw::downsample;

/// 渲染 shader（底层 / 顶层两个片元入口）
const WGSL: &str = include_str!("gpu_3d.wgsl");

/// 顶点（模型坐标 + 整张贴图归一化 uv）
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(crate) struct Vert {
    pub pos: [f32; 3],
    pub uv: [f32; 2],
}

/// 一个面的四个角 → 6 顶点（两三角形 0,1,2 / 0,2,3），uv 按贴图尺寸归一化
///
/// - `corners`: 面四角（模型坐标 + 0..1 块内比例，v=0 在贴图块顶部）
/// - `rect`: 贴图块 (tx, ty, tw, th)，像素坐标
/// - `tex_w` / `tex_h`: 贴图尺寸（HD 皮肤按实际尺寸归一化）
pub(crate) fn face_verts(
    corners: [([f32; 3], [f32; 2]); 4],
    rect: [i32; 4],
    tex_w: f32,
    tex_h: f32,
) -> [Vert; 6] {
    let (tx, ty, tw, th) = (rect[0] as f32, rect[1] as f32, rect[2] as f32, rect[3] as f32);
    let mut out = [Vert { pos: [0.0; 3], uv: [0.0; 2] }; 6];
    for (i, &ci) in [0usize, 1, 2, 0, 2, 3].iter().enumerate() {
        let (pos, uv) = corners[ci];
        out[i] = Vert {
            pos,
            uv: [(tx + uv[0] * tw) / tex_w, (ty + uv[1] * th) / tex_h],
        };
    }
    out
}

/// 像素空间（0..width / 0..height，y 向下）→ NDC 的正交投影
///
/// z 像素值越大离观察者越近，深度反向映射（近处深度小，配合 LessEqual + 清 1.0）
pub(crate) fn ortho(width: u32, height: u32) -> Mat4 {
    let (w, h) = (width as f32, height as f32);
    Mat4::from_cols_array(&[
        2.0 / w, 0.0, 0.0, 0.0,
        0.0, -2.0 / h, 0.0, 0.0,
        0.0, 0.0, -1.0 / 1000.0, 0.0,
        -1.0, 1.0, 0.5, 1.0,
    ])
}

/// 底层（不透明 / cutout）+ 顶层（半透明）两条管线
struct Pipelines {
    base: wgpu::RenderPipeline,
    overlay: wgpu::RenderPipeline,
    bind_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
}

/// 管线缓存（设备全局唯一，失败也记入避免反复重试）
static PIPE: OnceLock<Option<Pipelines>> = OnceLock::new();

fn pipelines() -> Option<&'static Pipelines> {
    PIPE.get_or_init(|| {
        let gpu = mml_gpu::device()?;
        Some(Pipelines::new(gpu.device()))
    })
    .as_ref()
}

impl Pipelines {
    fn new(device: &wgpu::Device) -> Pipelines {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("mml-skin-3d"),
            source: wgpu::ShaderSource::Wgsl(WGSL.into()),
        });

        let bind_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("mml-skin-3d"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
            ],
        });

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("mml-skin-3d"),
            bind_group_layouts: &[Some(&bind_layout)],
            immediate_size: 0,
        });

        // 两条管线都写深度：overlay 部件互相穿插（手臂 overlay 内侧面伸进
        // 身体 overlay），只读深度会退化成按绘制顺序取胜；底层透明镂空
        // 不写深度，overlay 仍能透过镂空显示
        let depth_write = wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: Some(true),
            depth_compare: Some(wgpu::CompareFunction::LessEqual),
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        };
        let depth_ro = depth_write.clone();
        let target = wgpu::ColorTargetState {
            format: wgpu::TextureFormat::Rgba8Unorm,
            blend: None,
            write_mask: wgpu::ColorWrites::ALL,
        };

        let mk = |blend: Option<wgpu::BlendState>, depth: wgpu::DepthStencilState, entry: &str| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("mml-skin-3d"),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs"),
                    compilation_options: Default::default(),
                    buffers: &[Some(wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<Vert>() as u64,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &[
                            wgpu::VertexAttribute {
                                format: wgpu::VertexFormat::Float32x3,
                                offset: 0,
                                shader_location: 0,
                            },
                            wgpu::VertexAttribute {
                                format: wgpu::VertexFormat::Float32x2,
                                offset: 12,
                                shader_location: 1,
                            },
                        ],
                    })],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some(entry),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        blend,
                        ..target.clone()
                    })],
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    // 面绕序为外侧 CCW（NDC y 向下时屏幕绕序保持 CCW）；
                    // 剔除背面：部件间存在共面面（头底/身顶、腿顶/身底、臂内/身侧），
                    // 不剔除会 z-fighting，背面盖住正面
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: Some(wgpu::Face::Back),
                    ..Default::default()
                },
                depth_stencil: Some(depth),
                multisample: wgpu::MultisampleState::default(),
                multiview_mask: None,
                cache: None,
            })
        };

        // 顶层：straight-alpha 混合（color SrcAlpha/1-SrcAlpha，alpha One/1-One），
        // 透明底上的累积结果即 premul 语义，与读回后的 Pixmap 一致
        let overlay_blend = Some(wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::SrcAlpha,
                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                operation: wgpu::BlendOperation::Add,
            },
        });

        Pipelines {
            base: mk(None, depth_write, "fs_base"),
            overlay: mk(overlay_blend, depth_ro, "fs_overlay"),
            bind_layout,
            sampler: device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("mml-skin-3d"),
                mag_filter: wgpu::FilterMode::Nearest,
                min_filter: wgpu::FilterMode::Nearest,
                mipmap_filter: wgpu::MipmapFilterMode::Nearest,
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                ..Default::default()
            }),
        }
    }
}

/// 渲染一组立方体面（超采样 → 读回 → 降采样，输出预乘 BGRA `Pixmap`）
///
/// - `texture`: 皮肤贴图
/// - `base` / `overlay`: 底层 / 顶层顶点（每面 6 顶点，[`face_verts`] 产出）
/// - `tran`: 模型变换矩阵（旋转 + 缩放 + 平移到像素空间）
/// - `width` / `height`: 输出尺寸（像素）
/// - `supersample`: 超采样倍数
pub(crate) fn render_3d(
    texture: &Pixmap,
    base: &[Vert],
    overlay: &[Vert],
    tran: Mat4,
    width: u32,
    height: u32,
    supersample: u32,
) -> Option<Pixmap> {
    let gpu = mml_gpu::device()?;
    let pipes = pipelines()?;
    let (device, queue) = (gpu.device(), gpu.queue());
    let (w, h) = (width * supersample, height * supersample);
    // 正交投影按最终尺寸算（变换矩阵的平移 / 缩放以最终像素为单位），
    // 超采样目标渲染出来正好放大 supersample 倍，降采样后回到最终坐标
    let mvp = ortho(width, height) * tran;

    // 贴图 + uniform + 绑定组
    let tex = mml_gpu::upload_texture(device, queue, texture)?;
    let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
    let uniform_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("mml-skin-3d uniform"),
        contents: bytemuck::bytes_of(&Mvp {
            mvp: mvp.to_cols_array_2d(),
        }),
        usage: wgpu::BufferUsages::UNIFORM,
    });
    let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("mml-skin-3d"),
        layout: &pipes.bind_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buf.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&pipes.sampler),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::TextureView(&view),
            },
        ],
    });

    let vbase = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("mml-skin-3d base"),
        contents: bytemuck::cast_slice(base),
        usage: wgpu::BufferUsages::VERTEX,
    });
    let voverlay = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("mml-skin-3d overlay"),
        contents: bytemuck::cast_slice(overlay),
        usage: wgpu::BufferUsages::VERTEX,
    });

    // 离屏渲染：先底层（写深度），后顶层（混合、只读深度）
    let target = mml_gpu::offscreen_target(device, w, h);
    let color_view = target.color.create_view(&wgpu::TextureViewDescriptor::default());
    let depth_view = target.depth.create_view(&wgpu::TextureViewDescriptor::default());
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("mml-skin-3d"),
    });
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("mml-skin-3d"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &color_view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_bind_group(0, &bind, &[]);
        pass.set_pipeline(&pipes.base);
        pass.set_vertex_buffer(0, vbase.slice(..));
        pass.draw(0..base.len() as u32, 0..1);
        pass.set_pipeline(&pipes.overlay);
        pass.set_vertex_buffer(0, voverlay.slice(..));
        pass.draw(0..overlay.len() as u32, 0..1);
    }
    queue.submit(Some(encoder.finish()));

    // 读回（premul RGBA）→ Pixmap（premul BGRA）→ 降采样
    let premul = mml_gpu::readback(device, queue, &target.color, w, h)?;
    let img = mml_gpu::pixmap_from_premul_rgba(&premul, w, h)?;
    downsample(&img, supersample)
}

/// uniform 布局（单个 MVP 矩阵）
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Mvp {
    mvp: [[f32; 4]; 4],
}
