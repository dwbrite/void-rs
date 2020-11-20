use wgpu::{RequestAdapterOptions, VertexAttributeDescriptor, VertexBufferDescriptor, VertexFormat, SwapChainError, SwapChainFrame};
use wgpu::util::DeviceExt;
use wgpu_glyph::{ab_glyph, GlyphBrush, GlyphBrushBuilder, Section, Text};
use winit::event::WindowEvent;
use winit::window::Window;

trait VBDesc {
    fn desc<'a>() -> wgpu::VertexBufferDescriptor<'a>;
}

struct GraphicsContext {
    surface: wgpu::Surface,

    device: wgpu::Device,
    queue: wgpu::Queue,
    sc_desc: wgpu::SwapChainDescriptor,
    swap_chain: wgpu::SwapChain,

    size: winit::dpi::PhysicalSize<u32>,
}

impl GraphicsContext {
    async fn new(window: &Window) -> Self {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::BackendBit::PRIMARY);
        let surface = unsafe { instance.create_surface(window) };
        let adapter = instance.request_adapter(
            &RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::Default,
                compatible_surface: Some(&surface),
            },
        ).await.unwrap();

        let (device, queue) = adapter.request_device(
            &wgpu::DeviceDescriptor {
                features: Default::default(),
                limits: Default::default(),
                shader_validation: false,
            },
            None,
        ).await.unwrap();

        let sc_desc = wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
        };
        let swap_chain = device.create_swap_chain(&surface, &sc_desc);


        GraphicsContext {
            surface,
            device,
            queue,
            sc_desc,
            swap_chain,
            size,
        }
    }
}


struct BgRenderContext {
    vertex_buffer: wgpu::Buffer,
    render_pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
}

impl BgRenderContext {
    fn build(ctx: &GraphicsContext) -> BgRenderContext {

        let diffuse_bytes = include_bytes!("../res/bg.png");
        let diffuse_image = image::load_from_memory(diffuse_bytes).unwrap();
        let diffuse_rgb = diffuse_image.as_rgba8().unwrap();

        use image::GenericImageView;
        let dimensions = diffuse_image.dimensions();

        let size = wgpu::Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            depth: 1,
        };

        let diffuse_texture = ctx.device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsage::SAMPLED | wgpu::TextureUsage::COPY_DST,
        });

        ctx.queue.write_texture(
            wgpu::TextureCopyView {
                texture: &diffuse_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            diffuse_rgb,
            wgpu::TextureDataLayout {
                offset: 0,
                bytes_per_row: 4*dimensions.0,
                rows_per_image: dimensions.1,
            },
            size,
        );

        let diffuse_view = diffuse_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let diffuse_sampler = ctx.device.create_sampler(&wgpu::SamplerDescriptor {
            // address_mode_u: wgpu::AddressMode::ClampToEdge,
            // address_mode_v: wgpu::AddressMode::ClampToEdge,
            // address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let bind_group_layout = ctx.device.create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
                label: Some("bg_bind_group_layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::SampledTexture {
                            dimension: wgpu::TextureViewDimension::D2,
                            component_type: wgpu::TextureComponentType::Float,
                            multisampled: false
                        },
                        count: None
                    }, wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::Sampler {
                            comparison: false
                        },
                        count: None
                    }
                ]
            }
        );

        let bind_group = ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bg_bind_group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&diffuse_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&diffuse_sampler),
                }
            ]
        });


        let (vs_module, fs_module) = {
            let vs_src = include_str!("shader.vert");
            let fs_src = include_str!("shader.frag");
            let mut compiler = shaderc::Compiler::new().unwrap();
            let vs_spirv = compiler.compile_into_spirv(vs_src, shaderc::ShaderKind::Vertex, "shader.vert", "main", None).unwrap();
            let fs_spirv = compiler.compile_into_spirv(fs_src, shaderc::ShaderKind::Fragment, "shader.frag", "main", None).unwrap();
            let vs_module = ctx.device.create_shader_module(wgpu::util::make_spirv(&vs_spirv.as_binary_u8()));
            let fs_module = ctx.device.create_shader_module(wgpu::util::make_spirv(&fs_spirv.as_binary_u8()));
            (vs_module, fs_module)
        };

        let render_pipeline_layout = ctx.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let render_pipeline = ctx.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&render_pipeline_layout),
            vertex_stage: wgpu::ProgrammableStageDescriptor {
                module: &vs_module,
                entry_point: "main",
            },
            fragment_stage: Some(wgpu::ProgrammableStageDescriptor {
                module: &fs_module,
                entry_point: "main",
            }),
            rasterization_state: Some(wgpu::RasterizationStateDescriptor {
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: wgpu::CullMode::None,
                clamp_depth: false,
                depth_bias: 0,
                depth_bias_slope_scale: 0.0,
                depth_bias_clamp: 0.0,
            }),
            primitive_topology: wgpu::PrimitiveTopology::TriangleList,
            color_states: &[
                wgpu::ColorStateDescriptor {
                    format: ctx.sc_desc.format,
                    color_blend: wgpu::BlendDescriptor::REPLACE,
                    alpha_blend: wgpu::BlendDescriptor::REPLACE,
                    write_mask: wgpu::ColorWrite::ALL,
                },
            ],
            depth_stencil_state: None,
            vertex_state: wgpu::VertexStateDescriptor {
                index_format: wgpu::IndexFormat::Uint32,
                vertex_buffers: &[
                    VertexData::desc(),
                ],
            },
            sample_count: 1,
            sample_mask: !0,
            alpha_to_coverage_enabled: false,
        });

        let vertex_buffer = ctx.device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Buffer"),
                contents: bytemuck::cast_slice(&VERTICES),
                usage: wgpu::BufferUsage::VERTEX,
            }
        );

        BgRenderContext {
            vertex_buffer,
            render_pipeline,
            bind_group_layout,
            bind_group,
        }
    }

    fn draw(&self, f_ctx: &mut FrameContext) {
        let mut render_pass = f_ctx.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                attachment: &f_ctx.frame_tex.view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 1.0,
                        g: 1.0,
                        b: 1.0,
                        a: 1.0,
                    }),
                    store: true,
                },
            }],
            depth_stencil_attachment: None,
        });

        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);

        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.draw(0..3, 0..1)
    }
}

struct TextRenderContext {
    glyph_brush: GlyphBrush<()>,
}

struct FancyText {
    pos: (f32, f32),
    str: String,
    color: [f32; 4],
}

impl TextRenderContext {
    fn build(ctx: &GraphicsContext) -> TextRenderContext {
        let font = ab_glyph::FontArc::try_from_slice(include_bytes!("../res/PressStart2P.ttf"))
            .expect("Load font");

        let glyph_brush = GlyphBrushBuilder::using_font(font)
            .build(&ctx.device, ctx.sc_desc.format);

        TextRenderContext {
            glyph_brush,
        }
    }

    fn draw(&mut self, f_ctx: &mut FrameContext, text: FancyText) {
        let section = Section {
            screen_position: text.pos,
            text: vec![
                Text::new(&text.str)
                    .with_scale(8.0)
                    .with_color(text.color),
            ],
            ..Section::default()
        };

        self.glyph_brush.queue(section);

        let mut staging_belt = wgpu::util::StagingBelt::new(0x400);

        self.glyph_brush.draw_queued(
            &f_ctx.ctx.device,
            &mut staging_belt,
            f_ctx.encoder,
            &f_ctx.frame_tex.view,
            f_ctx.ctx.size.width,
            f_ctx.ctx.size.height,
        ).expect("fix your shit bruh");
    }
}

struct FrameContext<'a> {
    ctx: &'a GraphicsContext,
    encoder: &'a mut wgpu::CommandEncoder,
    frame_tex: &'a wgpu::SwapChainTexture,
}

pub(crate) struct GameState {
    ctx: GraphicsContext,
    idk_bg: BgRenderContext,
    txt_lol: TextRenderContext,
}

impl GameState {
    pub(crate) async fn new(window: &Window) -> Self {
        let ctx = GraphicsContext::new(window).await;
        let idk_bg = BgRenderContext::build(&ctx);
        let txt_lol = TextRenderContext::build(&ctx);

        GameState {
            ctx,
            idk_bg,
            txt_lol,
        }
    }

    pub(crate) fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.ctx.size = new_size;
        self.ctx.sc_desc.width = new_size.width;
        self.ctx.sc_desc.height = new_size.height;
        self.ctx.swap_chain = self.ctx.device.create_swap_chain(&self.ctx.surface, &self.ctx.sc_desc);
    }

    pub(crate) fn input(&mut self, _event: &WindowEvent) -> bool {
        false
    }

    pub(crate) fn update(&mut self) {}

    pub(crate) fn render(&mut self) {
        let frame_tex = {
            let frame = self.ctx.swap_chain.get_current_frame();
            use wgpu::SwapChainError::*;
            match frame {
                Ok(_f) => { _f }
                Err(Outdated) => {
                    self.resize(self.ctx.size);
                    self.ctx.swap_chain.get_current_frame()
                        .expect("swapchain failed to get current frame (twice)")
                }
                Err(Timeout) => { return /*assume gpu is asleep?*/ }
                _ => { frame.expect("swapchain failed to get current frame") }
            }
        }.output;

        let mut encoder = self.ctx
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });


        let mut f_ctx = FrameContext {
            ctx: &self.ctx,
            encoder: &mut encoder,
            frame_tex: &frame_tex,
        };

        self.idk_bg.draw(&mut f_ctx);
        self.txt_lol.draw(&mut f_ctx, FancyText {
            pos: (0.0, 0.0),
            str: "idk bro".to_string(),
            color: [1.0, 1.0, 1.0, 1.0],
        });


        self.ctx.queue.submit(std::iter::once(encoder.finish()));
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct VertexData {
    position: [f32; 3],
}

impl VBDesc for VertexData {
    fn desc<'a>() -> VertexBufferDescriptor<'a> {
        VertexBufferDescriptor {
            stride: std::mem::size_of::<VertexData>() as wgpu::BufferAddress, // 1.
            step_mode: wgpu::InputStepMode::Vertex, // 2.
            attributes: &[
                VertexAttributeDescriptor {
                    offset: 0,
                    format: VertexFormat::Float3,
                    shader_location: 0,
                },
            ],
        }
    }
}

const VERTICES: &[VertexData] = &[
    VertexData { position: [-4.0, -4.0, 0.0] },
    VertexData { position: [-4.0, 4.0, 0.0] },
    VertexData { position: [3.0, 0.0, 0.0] },
];


unsafe impl bytemuck::Zeroable for VertexData {}

unsafe impl bytemuck::Pod for VertexData {}