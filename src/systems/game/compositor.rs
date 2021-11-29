use crate::graphics::GraphicsContext;
use wgpu::{
    BindGroup, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType,
    BlendComponent, BlendFactor, BlendOperation, BlendState, Buffer, BufferBindingType,
    BufferDescriptor, BufferUsages, Color, ColorTargetState, ColorWrites, CommandEncoder, Extent3d,
    FragmentState, LoadOp, MultisampleState, PipelineLayoutDescriptor, PolygonMode, PrimitiveState,
    PrimitiveTopology, RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline,
    RenderPipelineDescriptor, ShaderModule, ShaderModuleDescriptor, ShaderSource, ShaderStages,
    SurfaceTexture, Texture, TextureDescriptor, TextureDimension, TextureFormat, VertexState,
};

pub struct Compositor {
    shader: ShaderModule,
    pipeline: RenderPipeline,
    data_buf: Buffer,
    tex_buf: Texture,
    bind_group_layout: BindGroupLayout,
    bind_group: BindGroup,
}

impl Compositor {
    pub fn new(gc: &mut GraphicsContext) -> Self {
        let shader = gc.device.create_shader_module(&ShaderModuleDescriptor {
            label: None,
            source: ShaderSource::Wgsl(
                include_str!("../../graphics/shaders/basic_batch.wgsl").into(),
            ),
        });

        let data_buf = {
            gc.device.create_buffer(&BufferDescriptor {
                label: Some("object buffer"),
                size: 48 * 500000, // this allows for 500000 objects on screen at a time
                usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        };

        let tex_buf = {
            let tex = gc.device.create_texture(&TextureDescriptor {
                label: Some("just one texture"),
                size: Extent3d {
                    width: 256,
                    height: 256,
                    depth_or_array_layers: 256,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::Rgba8UnormSrgb, // probably srgb right?
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            });
            tex
        };

        let bind_group_layout = gc
            .device
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("bind group layout I guess lol"),
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let bind_group = gc.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: data_buf.as_entire_binding(),
            }],
            label: None,
        });

        let pipeline = {
            let layout = gc.device.create_pipeline_layout(&PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            });

            gc.device.create_render_pipeline(&RenderPipelineDescriptor {
                label: None,
                layout: Some(&layout),
                vertex: VertexState {
                    module: &shader,
                    entry_point: "vert_main",
                    buffers: &[], // no vertex buffers
                },
                primitive: PrimitiveState {
                    topology: PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: None,
                    clamp_depth: false,
                    polygon_mode: PolygonMode::Fill,
                    conservative: false,
                },
                depth_stencil: None,
                multisample: MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                fragment: Some(FragmentState {
                    module: &shader,
                    entry_point: "main",
                    targets: &[ColorTargetState {
                        format: gc.config.format,
                        blend: Some(BlendState {
                            color: BlendComponent {
                                src_factor: BlendFactor::SrcAlpha,
                                dst_factor: BlendFactor::OneMinusDstAlpha,
                                operation: BlendOperation::Add,
                            },
                            alpha: BlendComponent::OVER,
                        }),
                        write_mask: ColorWrites::ALL,
                    }],
                }),
            })
        };

        Self {
            shader,
            pipeline,
            data_buf,
            tex_buf,
            bind_group_layout,
            bind_group,
        }
    }

    // TODO: send data to buffers, create pipeline (before rendering), and make one final draw call here :)
    pub fn render(
        &self,
        _gc: &mut GraphicsContext,
        encoder: &mut CommandEncoder,
        frame_tex: &SurfaceTexture,
    ) {
        let view = &frame_tex
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("Render pass descriptor"),
            color_attachments: &[RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: LoadOp::Clear(Color {
                        r: 1.0,
                        g: 0.0,
                        b: 1.0,
                        a: 1.0,
                    }),
                    store: true,
                },
            }],
            depth_stencil_attachment: None,
        });

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.draw(0..0, 0..1);
    }
}
