//! GPU 渲染后端（wgpu）：离屏渲染 0..1 模型空间 quad -> RGBA 读回
//!
//! 后端回退链 DX12 → VK → GL（无可用适配器时由调用方回退 CPU skia 路径）。
//! 变换链与光照公式照 26.2 反编译语义：
//! M = T(slot/2) · S(slot,-slot,slot) · Tt · Rx·Ry·Rz · Ss · T(-0.5)
//! 光照：accum = min(1, (max(0,dot(L0,N)) + max(0,dot(L1,N))) · 0.6 + 0.4)

use std::collections::HashMap;

use glam::{Mat3, Mat4, Vec3};
use pollster::block_on;
use skia_safe::Bitmap;

use crate::model::BakedModel;

/// 渲染 shader（cutout/translucent 两个片元入口）
const WGSL: &str = include_str!("gpu_render.wgsl");

/// uniform 布局（mat4x2 + 两个光向量）
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniforms {
    mvp: [[f32; 4]; 4],
    normal_mat: [[f32; 4]; 4],
    light0: [f32; 4],
    light1: [f32; 4],
}

/// 顶点布局（pos/uv/normal/color，48字节）
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct GpuVertex {
    pos: [f32; 3],
    uv: [f32; 2],
    normal: [f32; 3],
    color: [f32; 4],
}

const VERTEX_LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
    array_stride: std::mem::size_of::<GpuVertex>() as u64,
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
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x3,
            offset: 20,
            shader_location: 2,
        },
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x4,
            offset: 32,
            shader_location: 3,
        },
    ],
};


pub struct GpuCtx {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline_cutout: wgpu::RenderPipeline,
    pipeline_translucent: wgpu::RenderPipeline,
    bind_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
}

impl GpuCtx {
    /// 按系统选择后端回退链，全失败返回 None（调用方回退CPU skia）：
    /// - Windows：DX12 → VK → GL
    /// - macOS：Metal → VK → GL
    /// - Linux/其他Unix：VK → GL
    #[cfg(windows)]
    pub fn try_new() -> Option<GpuCtx> {
        block_on(Self::try_new_async(&[
            ("DX12", wgpu::Backends::DX12),
            ("VULKAN", wgpu::Backends::VULKAN),
            ("GL", wgpu::Backends::GL),
        ]))
    }

    #[cfg(target_os = "macos")]
    pub fn try_new() -> Option<GpuCtx> {
        block_on(Self::try_new_async(&[
            ("METAL", wgpu::Backends::METAL),
            ("VULKAN", wgpu::Backends::VULKAN),
            ("GL", wgpu::Backends::GL),
        ]))
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    pub fn try_new() -> Option<GpuCtx> {
        block_on(Self::try_new_async(&[
            ("VULKAN", wgpu::Backends::VULKAN),
            ("GL", wgpu::Backends::GL),
        ]))
    }

    #[cfg(not(any(windows, unix)))]
    pub fn try_new() -> Option<GpuCtx> {
        block_on(Self::try_new_async(&[("GL", wgpu::Backends::GL)]))
    }

    /// 指定单一后端初始化（测试/调试用）
    pub fn try_new_backend(name: &str, backends: wgpu::Backends) -> Option<GpuCtx> {
        block_on(Self::try_new_async(&[(name, backends)]))
    }

    async fn try_new_async(chain: &[(&str, wgpu::Backends)]) -> Option<GpuCtx> {
        for (name, backends) in chain {
            let mut desc = wgpu::InstanceDescriptor::new_without_display_handle();
            desc.backends = *backends;
            let instance = wgpu::Instance::new(desc);
            let Ok(adapter) = instance
                .request_adapter(&wgpu::RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::LowPower,
                    compatible_surface: None,
                    force_fallback_adapter: false,
                    apply_limit_buckets: false,
                })
                .await
            else {
                continue;
            };
            let info = adapter.get_info();
            let Ok((device, queue)) = adapter
                .request_device(&wgpu::DeviceDescriptor {
                    label: Some("mcml-tex-draw"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    experimental_features: wgpu::ExperimentalFeatures::default(),
                    memory_hints: wgpu::MemoryHints::default(),
                    trace: wgpu::Trace::Off,
                })
                .await
            else {
                continue;
            };
            println!("[渲染] GPU后端 {name}：{}（{:?}）", info.name, info.device_type);
            return Some(Self::build(device, queue));
        }
        None
    }

    fn build(device: wgpu::Device, queue: wgpu::Queue) -> GpuCtx {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("mcml-tex-draw"),
            source: wgpu::ShaderSource::Wgsl(WGSL.into()),
        });

        let bind_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("mcml-tex-draw"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
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
            label: Some("mcml-tex-draw"),
            bind_group_layouts: &[Some(&bind_layout)],
            immediate_size: 0,
        });

        let depth = wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: Some(true),
            depth_compare: Some(wgpu::CompareFunction::LessEqual),
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        };
        let target = wgpu::ColorTargetState {
            format: wgpu::TextureFormat::Rgba8Unorm,
            blend: None,
            write_mask: wgpu::ColorWrites::ALL,
        };

        let mk_pipeline = |blend: Option<wgpu::BlendState>,
                           depth_write: bool,
                           entry: &str| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("mcml-tex-draw"),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs"),
                    compilation_options: Default::default(),
                    buffers: &[Some(VERTEX_LAYOUT)],
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
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: Some(wgpu::Face::Back),
                    ..Default::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    depth_write_enabled: Some(depth_write),
                    ..depth.clone()
                }),
                multisample: wgpu::MultisampleState::default(),
                multiview_mask: None,
                cache: None,
            })
        };

        let pipeline_cutout = mk_pipeline(None, true, "fs_cutout");
        // 半透明：Straight-alpha 混合（color SrcAlpha/1-SrcAlpha，alpha One/1-One），
        // 读回后按 premultiplied 语义反除回 straight（与 skia 离屏结果一致）
        let pipeline_translucent = mk_pipeline(
            Some(wgpu::BlendState {
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
            }),
            false,
            "fs_translucent",
        );

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("mcml-tex-draw"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            ..Default::default()
        });

        GpuCtx {
            device,
            queue,
            pipeline_cutout,
            pipeline_translucent,
            bind_layout,
            sampler,
        }
    }
}

/// gui display 变换矩阵（照 ItemTransform.apply：translate·rotationXYZ·scale·translate(-0.5)）
pub fn item_transform_matrix(t: &crate::model::GuiTransform) -> Mat4 {
    let rot = t.rotation.map(|deg| deg.to_radians());
    Mat4::from_translation(Vec3::from(t.translation))
        * Mat4::from_rotation_x(rot[0])
        * Mat4::from_rotation_y(rot[1])
        * Mat4::from_rotation_z(rot[2])
        * Mat4::from_scale(Vec3::from(t.scale))
        * Mat4::from_translation(Vec3::new(-0.5, -0.5, -0.5))
}

/// 光照方向（照 Lighting.java：ITEMS_3D / ITEMS_FLAT 两个UBO槽位）
pub fn light_dirs(item3d: bool) -> ([f32; 4], [f32; 4]) {
    let l0 = Vec3::new(0.2, 1.0, -0.7).normalize();
    let l1 = Vec3::new(-0.2, 1.0, 0.7).normalize();
    let m = if item3d {
        // new Matrix4f().scaling(1,-1,1).rotateYXZ(a,b,0).rotateYXZ(-π/8, 3π/4, 0)
        // JOML rotateYXZ(y,x,z) = Ry·Rx·Rz
        Mat4::from_scale(Vec3::new(1.0, -1.0, 1.0))
            * Mat4::from_rotation_y(1.0821041)
            * Mat4::from_rotation_x(3.2375858)
            * Mat4::from_rotation_y(-std::f32::consts::FRAC_PI_8)
            * Mat4::from_rotation_x(3.0 * std::f32::consts::FRAC_PI_4)
    } else {
        // flatPose = rotY(-π/8) · rotX(3π/4)
        Mat4::from_rotation_y(-std::f32::consts::FRAC_PI_8)
            * Mat4::from_rotation_x(3.0 * std::f32::consts::FRAC_PI_4)
    };
    let v = |d: Vec3| {
        let t = m.transform_vector3(d).normalize();
        [t.x, t.y, t.z, 0.0]
    };
    (v(l0), v(l1))
}

impl GpuCtx {
    /// 渲染一个烘焙模型到 size×size RGBA，返回 straight-alpha 像素（未成功返回 None）
    pub fn render(
        &self,
        model: &BakedModel,
        textures: &HashMap<String, Bitmap>,
        size: u32,
    ) -> Option<Vec<u8>> {
        use wgpu::util::DeviceExt;

        let slot = size as f32;

        // 完整变换链 + 正交投影（NDC z∈[0,1]）。注意：屏幕像素空间里离观察者越近
        // 像素z越大，深度需反向映射（z_ndc = 0.5 - z_pix/2000，近处深度小），
        // 否则LessEqual深度测试永远保留最远的面，效果如同未开深度测试：
        // MVP = P · T(slot/2) · S(slot,-slot,slot) · Tt·R·Ss·T(-0.5)
        let model_m = Mat4::from_translation(Vec3::new(slot / 2.0, slot / 2.0, 0.0))
            * Mat4::from_scale(Vec3::new(slot, -slot, slot))
            * item_transform_matrix(&model.transform);
        let mvp = Mat4::from_cols_array(&[
            2.0 / slot, 0.0, 0.0, 0.0,
            0.0, -2.0 / slot, 0.0, 0.0,
            0.0, 0.0, -1.0 / 2000.0, 0.0,
            -1.0, 1.0, 0.5, 1.0,
        ]) * model_m;

        // 法线矩阵 = 逆转置（逐顶点归一化，等价MC normal matrix + trustedNormals规则）
        let n3 = Mat3::from_mat4(model_m).inverse().transpose();

        let (light0, light1) = light_dirs(model.gui_light_3d);
        let uniforms = Uniforms {
            mvp: mvp.to_cols_array_2d(),
            normal_mat: [
                [n3.x_axis.x, n3.x_axis.y, n3.x_axis.z, 0.0],
                [n3.y_axis.x, n3.y_axis.y, n3.y_axis.z, 0.0],
                [n3.z_axis.x, n3.z_axis.y, n3.z_axis.z, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
            light0,
            light1,
        };
        let uniform_buf = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("uniform"),
                contents: bytemuck::bytes_of(&uniforms),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        // 按贴图分组、组内每quad 6顶点（0,1,2,0,2,3）
        let mut groups: Vec<(String, Vec<GpuVertex>)> = Vec::new();
        let mut lookup: HashMap<String, usize> = HashMap::new();
        for quad in &model.quads {
            let idx = match lookup.get(&quad.tex) {
                Some(i) => *i,
                None => {
                    groups.push((quad.tex.clone(), Vec::new()));
                    lookup.insert(quad.tex.clone(), groups.len() - 1);
                    groups.len() - 1
                }
            };
            let bucket = &mut groups[idx].1;
            for i in [0usize, 1, 2, 0, 2, 3] {
                bucket.push(GpuVertex {
                    pos: quad.pos[i],
                    uv: quad.uv[i],
                    normal: quad.normal,
                    color: quad.color[i],
                });
            }
        }

        // 半透明组内quad按中心z从小到大排（远→近，配合深度只读的LessEqual；
        // 屏幕像素z越大越近，远=小z先画）
        let mut pass_groups: Vec<(String, wgpu::Buffer, u32, bool)> = Vec::new();
        for (tex_path, verts) in groups {
            if verts.is_empty() {
                continue;
            }
            let translucent = model
                .quads
                .iter()
                .any(|q| q.tex == tex_path && q.translucent);
            let mut verts = verts;
            if translucent {
                let mut quads: Vec<[GpuVertex; 6]> =
                    verts.chunks(6).map(|c| c.try_into().unwrap()).collect();
                quads.sort_by(|a, b| {
                    let z = |q: &[GpuVertex; 6]| q.iter().take(4).map(|v| v.pos[2]).sum::<f32>();
                    z(a).partial_cmp(&z(b)).unwrap_or(std::cmp::Ordering::Equal)
                });
                verts = quads.iter().flat_map(|q| *q).collect();
            }
            let vbuf = self
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("vertex"),
                    contents: bytemuck::cast_slice(&verts),
                    usage: wgpu::BufferUsages::VERTEX,
                });
            pass_groups.push((tex_path, vbuf, verts.len() as u32, translucent));
        }

        // 上传贴图（Bitmap为premul，转straight后建绑定组）
        let mut binds: HashMap<String, wgpu::BindGroup> = HashMap::new();
        let mut views: Vec<(String, wgpu::Texture)> = Vec::new();
        for (tex_path, ..) in &pass_groups {
            if binds.contains_key(tex_path) {
                continue;
            }
            let Some(tex) = textures.get(tex_path) else {
                return None;
            };
            // read_pixels 顺带完成 premul->straight（jar内贴图为straight alpha）
            let Some((rgba, stride, w, h)) = crate::model::bitmap_rgba(tex) else {
                return None;
            };
            let (w, h) = (w as u32, h as u32);
            let texture = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("sprite"),
                size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
            self.queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                &rgba,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(stride as u32),
                    rows_per_image: Some(h),
                },
                wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
            );
            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            let bind = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("sprite"),
                layout: &self.bind_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: uniform_buf.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(&view),
                    },
                ],
            });
            views.push((tex_path.clone(), texture));
            binds.insert(tex_path.clone(), bind);
        }

        // 离屏目标 + 深度
        let color_tex = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("target"),
            size: wgpu::Extent3d { width: size, height: size, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let color_view = color_tex.create_view(&wgpu::TextureViewDescriptor::default());
        let depth_tex = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("depth"),
            size: wgpu::Extent3d { width: size, height: size, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let depth_view = depth_tex.create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("mcml-tex-draw"),
            });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("icon"),
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

            // 先不透明/cutout（深度写入），后半透明（深度只读）
            for (tex_path, vbuf, count, translucent) in &pass_groups {
                let bind = binds.get(tex_path)?;
                pass.set_bind_group(0, bind, &[]);
                pass.set_pipeline(if *translucent {
                    &self.pipeline_translucent
                } else {
                    &self.pipeline_cutout
                });
                pass.set_vertex_buffer(0, vbuf.slice(..));
                pass.draw(0..*count, 0..1);
            }
        }

        // 读回（拷贝到对齐缓冲）
        let bytes_per_row = size * 4;
        let read_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("readback"),
            size: (bytes_per_row * size) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        encoder.copy_texture_to_buffer(
            color_tex.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &read_buf,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(bytes_per_row),
                    rows_per_image: Some(size),
                },
            },
            wgpu::Extent3d { width: size, height: size, depth_or_array_layers: 1 },
        );
        self.queue.submit(Some(encoder.finish()));

        let slice = read_buf.slice(..);
        slice.map_async(wgpu::MapMode::Read, |_| {});
        self.device.poll(wgpu::PollType::wait_indefinitely()).ok()?;
        let data = slice.get_mapped_range().ok()?;
        if data.len() != (bytes_per_row * size) as usize {
            return None;
        }

        // 离屏混合为premultiplied语义，反除回straight（透明背景PNG）
        let mut out = vec![0u8; data.len()];
        for (dst, src) in out.chunks_exact_mut(4).zip(data.chunks_exact(4)) {
            let a = src[3];
            dst.copy_from_slice(src);
            if a > 0 && a < 255 {
                for c in &mut dst[0..3] {
                    *c = ((u16::from(*c) * 255) / u16::from(a)).min(255) as u8;
                }
            }
        }
        drop(data);
        read_buf.unmap();
        Some(out)
    }
}
