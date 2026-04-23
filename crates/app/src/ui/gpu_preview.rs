//! wgpu PaintCallback 直接渲染截屏画面
//! 绕过 egui texture atlas，截屏 BGRA 数据直接 upload 到 wgpu texture

use std::sync::Arc;
use wgpu::util::DeviceExt;

// ============================================================================
// WGSL Shader
// ============================================================================

const SHADER_CODE: &str = r#"
struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) tex_coord: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coord: vec2<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = vec4<f32>(in.position, 0.0, 1.0);
    out.tex_coord = in.tex_coord;
    return out;
}

@group(0) @binding(0)
var t_diffuse: texture_2d<f32>;

@group(0) @binding(1)
var s_diffuse: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let color = textureSample(t_diffuse, s_diffuse, in.tex_coord);
    // scrap 返回的 BGRA alpha=0，强制为 1.0
    return vec4<f32>(color.rgb, 1.0);
}
"#;

// ============================================================================
// Vertex Data
// ============================================================================

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 2],
    tex_coord: [f32; 2],
}

/// 全屏 NDC quad，左上角 UV = (0,0)
const VERTICES: &[Vertex] = &[
    Vertex { position: [-1.0, -1.0], tex_coord: [0.0, 1.0] }, // 左下
    Vertex { position: [ 1.0, -1.0], tex_coord: [1.0, 1.0] }, // 右下
    Vertex { position: [ 1.0,  1.0], tex_coord: [1.0, 0.0] }, // 右上
    Vertex { position: [-1.0,  1.0], tex_coord: [0.0, 0.0] }, // 左上
];

const INDICES: &[u16] = &[0, 1, 2, 0, 2, 3];

// ============================================================================
// GpuPreview - wgpu 资源管理
// ============================================================================

pub struct GpuPreview {
    device: wgpu::Device,
    sampler: wgpu::Sampler,
    bind_group_layout: wgpu::BindGroupLayout,
    pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_indices: u32,

    // 动态创建/销毁（随截屏尺寸变化）
    texture: Option<wgpu::Texture>,
    texture_view: Option<wgpu::TextureView>,
    bind_group: Option<wgpu::BindGroup>,
    current_size: [u32; 2],
}

impl GpuPreview {
    pub fn new(device: &wgpu::Device, target_format: wgpu::TextureFormat) -> Self {
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("gpu_preview_bind_group_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("gpu_preview_pipeline_layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("gpu_preview_shader"),
            source: wgpu::ShaderSource::Wgsl(SHADER_CODE.into()),
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("gpu_preview_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute {
                            offset: 0,
                            shader_location: 0,
                            format: wgpu::VertexFormat::Float32x2,
                        },
                        wgpu::VertexAttribute {
                            offset: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                            shader_location: 1,
                            format: wgpu::VertexFormat::Float32x2,
                        },
                    ],
                }],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: target_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("gpu_preview_vertex_buffer"),
            contents: bytemuck::cast_slice(VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("gpu_preview_index_buffer"),
            contents: bytemuck::cast_slice(INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });

        Self {
            device: device.clone(),
            sampler,
            bind_group_layout,
            pipeline,
            vertex_buffer,
            index_buffer,
            num_indices: INDICES.len() as u32,
            texture: None,
            texture_view: None,
            bind_group: None,
            current_size: [0, 0],
        }
    }

    /// 确保 texture 尺寸匹配，必要时重新创建
    fn ensure_texture(&mut self, width: u32, height: u32) {
        if self.texture.is_some() && self.current_size == [width, height] {
            return;
        }

        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("gpu_preview_texture"),
            size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            // Bgra8Unorm: 直接上传 scrap 的 BGRA 数据，shader 采样自动转为 RGB
            format: wgpu::TextureFormat::Bgra8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("gpu_preview_bind_group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });

        self.texture = Some(texture);
        self.texture_view = Some(texture_view);
        self.bind_group = Some(bind_group);
        self.current_size = [width, height];
    }

    /// 上传新的截屏数据（BGRA）
    pub fn update(&mut self, queue: &wgpu::Queue, data: &[u8], width: u32, height: u32) {
        self.ensure_texture(width, height);
        if let Some(ref texture) = self.texture {
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    aspect: wgpu::TextureAspect::All,
                    texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                },
                data,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(width * 4),
                    rows_per_image: Some(height),
                },
                wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
            );
        }
    }

    /// 渲染到当前 render pass（调用方已设置好 viewport / scissor）
    pub fn render(&self, render_pass: &mut wgpu::RenderPass<'_>) {
        render_pass.set_pipeline(&self.pipeline);
        if let Some(ref bind_group) = self.bind_group {
            render_pass.set_bind_group(0, bind_group, &[]);
        }
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.draw_indexed(0..self.num_indices, 0, 0..1);
    }
}

// ============================================================================
// GpuPreviewCallback - egui_wgpu CallbackTrait 实现
// ============================================================================

pub struct GpuPreviewCallback {
    /// (BGRA 数据, width, height)
    pub frame: Option<Arc<(Vec<u8>, u32, u32)>>,
    /// 截屏原始宽高（用于等比缩放计算）
    pub frame_wh: [u32; 2],
}

impl egui_wgpu::CallbackTrait for GpuPreviewCallback {
    fn prepare(
        &self,
        _device: &wgpu::Device,
        queue: &wgpu::Queue,
        _screen_descriptor: &egui_wgpu::ScreenDescriptor,
        _egui_encoder: &mut wgpu::CommandEncoder,
        callback_resources: &mut egui_wgpu::CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        if let Some(preview) = callback_resources.get_mut::<GpuPreview>() {
            if let Some(ref frame) = self.frame {
                let (data, width, height) = &**frame;
                preview.update(queue, data, *width, *height);
            }
        }
        Vec::new()
    }

    fn paint(
        &self,
        info: egui::PaintCallbackInfo,
        render_pass: &mut wgpu::RenderPass<'static>,
        callback_resources: &egui_wgpu::CallbackResources,
    ) {
        let preview = match callback_resources.get::<GpuPreview>() {
            Some(p) => p,
            None => return,
        };

        let px = info.pixels_per_point;
        let viewport = info.viewport;
        let vp_w = viewport.width();
        let vp_h = viewport.height();

        // 等比缩放：计算 texture 在 viewport 中的显示区域
        let [fw, fh] = self.frame_wh;
        let img_aspect = fw as f32 / fh.max(1) as f32;
        let vp_aspect = vp_w / vp_h.max(1.0);
        let (disp_w, disp_h) = if img_aspect > vp_aspect {
            (vp_w, vp_w / img_aspect)
        } else {
            (vp_h * img_aspect, vp_h)
        };
        let disp_x = viewport.min.x + (vp_w - disp_w) * 0.5;
        let disp_y = viewport.min.y + (vp_h - disp_h) * 0.5;

        // 转换为物理像素并设置 viewport
        let vx = (disp_x * px) as f32;
        let vy = (disp_y * px) as f32;
        let vw = (disp_w * px) as f32;
        let vh = (disp_h * px) as f32;
        render_pass.set_viewport(vx, vy, vw, vh, 0.0, 1.0);

        // Scissor（基于 clip_rect，防止绘制到 viewport 外）
        let clip = info.clip_rect;
        let screen = info.screen_size_px;
        let clip_min_x = (clip.min.x * px).max(0.0) as u32;
        let clip_min_y = (clip.min.y * px).max(0.0) as u32;
        let clip_max_x = (clip.max.x * px).min(screen[0] as f32) as u32;
        let clip_max_y = (clip.max.y * px).min(screen[1] as f32) as u32;
        let cw = clip_max_x.saturating_sub(clip_min_x);
        let ch = clip_max_y.saturating_sub(clip_min_y);
        render_pass.set_scissor_rect(clip_min_x, clip_min_y, cw, ch);

        preview.render(render_pass);
    }
}
