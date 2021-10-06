mod object;

use crate::graphics::GraphicsContext;
use crate::systems::controls::Controls;
use crate::systems::game::object::Object::Scenery;
use crate::systems::game::object::{Chunk, Expanse, Object, Position};

use wgpu::{
    BindGroup, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType,
    Buffer, BufferBindingType, BufferDescriptor, BufferUsages, Color, CommandEncoder,
    CommandEncoderDescriptor, FragmentState, LoadOp, MultisampleState, PipelineLayoutDescriptor,
    PolygonMode, PrimitiveState, PrimitiveTopology, RenderPassColorAttachment,
    RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor, ShaderModule,
    ShaderModuleDescriptor, ShaderSource, SurfaceError, SurfaceTexture, Texture, TextureAspect,
    TextureDescriptor, TextureDimension, TextureFormat, VertexState,
};

use rand::Rng;
use wgpu::ShaderStages;
use winit::event::Event;
use winit::window::Window;

pub struct IO {
    pub ticks: u64,
    pub controls: Controls,
    // pub audio_tx: Sender<AudioSysMsg>,
    // pub draw_queue: VecDeque<DrawCommand>,
}

pub struct BufferTracker {
    pub obj_count: usize,
    pub positions: Vec<[u32; 2]>, // size 65536
    pub uvs: Vec<[[f32; 2]; 4]>,  // size 131072
    pub sizes: Vec<[u32; 2]>,     // size: 32768
}

pub struct Compositor {
    pub tracker: BufferTracker,
    shader: ShaderModule,
    pipeline: RenderPipeline,
    pos_buf: Buffer,
    uv_buf: Buffer,
    size_buf: Buffer,
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

        let tracker = BufferTracker {
            obj_count: 0,
            positions: Vec::with_capacity(64000),
            uvs: Vec::with_capacity(64000),
            sizes: Vec::with_capacity(64000),
        };

        let pos_buf = {
            gc.device.create_buffer(&BufferDescriptor {
                label: Some("position buffer"),
                size: 8 * 64000, // this allows for 64000 objects on screen at a time
                usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        };

        let uv_buf = {
            gc.device.create_buffer(&BufferDescriptor {
                label: Some("UV buffer"),
                size: 32 * 64000,
                usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        };

        let size_buf = {
            gc.device.create_buffer(&BufferDescriptor {
                label: Some("size buffer"),
                size: 8 * 64000,
                usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        };

        let diffuse_bytes = include_bytes!("../../../resources/birb.png");
        let diffuse_image = image::load_from_memory(diffuse_bytes).unwrap();
        let diffuse_rgba = diffuse_image.as_rgba8().unwrap();

        use image::GenericImageView;
        let dimensions = diffuse_image.dimensions();

        let tex_buf = {
            let texture_size = wgpu::Extent3d {
                width: dimensions.0,
                height: dimensions.1,
                depth_or_array_layers: 1,
            };

            let tex = gc.device.create_texture(&TextureDescriptor {
                label: Some("just one texture"),
                size: texture_size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::Rgba8UnormSrgb, // probably srgb right?
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            });

            gc.queue.write_texture(
                // Tells wgpu where to copy the pixel data
                wgpu::ImageCopyTexture {
                    texture: &tex,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: TextureAspect::All,
                },
                // The actual pixel data
                diffuse_rgba,
                // The layout of the texture
                wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: std::num::NonZeroU32::new(4 * dimensions.0),
                    rows_per_image: std::num::NonZeroU32::new(dimensions.1),
                },
                texture_size,
            );

            tex
        };

        let bind_group_layout = gc
            .device
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("bind group layout I guess lol"),
                entries: &[
                    // position
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::VERTEX,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // uv
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // size
                    BindGroupLayoutEntry {
                        binding: 2,
                        visibility: ShaderStages::VERTEX,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        let bind_group = gc.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: pos_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: uv_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: size_buf.as_entire_binding(),
                },
            ],
            label: Some("bla bla data bind group"),
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
                    targets: &[gc.config.format.into()],
                }),
            })
        };

        Self {
            tracker,
            shader,
            pipeline,
            pos_buf,
            uv_buf,
            size_buf,
            tex_buf,
            bind_group_layout,
            bind_group,
        }
    }

    // TODO: send data to buffers, create pipeline (before rendering), and make one final draw call here :)
    pub fn render(
        &self,
        gc: &mut GraphicsContext,
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
                    load: LoadOp::Clear(Color::BLACK),
                    store: true,
                },
            }],
            depth_stencil_attachment: None,
        });

        // TODO: upload data to buffers
        gc.queue.write_buffer(
            &self.pos_buf,
            0,
            bytemuck::cast_slice(self.tracker.positions.as_slice()),
        );

        gc.queue.write_buffer(
            &self.uv_buf,
            0,
            bytemuck::cast_slice(self.tracker.uvs.as_slice()),
        );

        gc.queue.write_buffer(
            &self.size_buf,
            0,
            bytemuck::cast_slice(self.tracker.sizes.as_slice()),
        );

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.draw(0..(self.tracker.obj_count as u32 * 6), 0..1);
    }

    pub fn add_object(
        &mut self,
        position: Position,
        size: [u32; 2],
        uv: [[f32; 2]; 4],
        // texture: usize,
    ) {
        let mut t = &mut self.tracker;
        t.positions.push([position.x, position.y]);
        t.sizes.push(size);
        t.uvs.push(uv);
        t.obj_count += 1;
        // self.textures.push(texture);
    }

    pub fn reset(&mut self) {
        self.tracker.obj_count = 0;
        self.tracker.positions.clear();
        self.tracker.sizes.clear();
        self.tracker.uvs.clear();
        // don't waste cpu time resetting items?
    }
}

pub struct GameSystem {
    pub gc: GraphicsContext,
    pub io: IO,
    // TODO: move me vvvvvv
    pub expanse: Expanse,
    pub camera: Position,

    pub compositor: Compositor,
}

impl GameSystem {
    pub fn new(window: Window) -> Self {
        let mut gc = GraphicsContext::new(window);

        let controls = Controls::default();

        let io = IO { ticks: 0, controls };

        let compositor = Compositor::new(&mut gc);

        let mut objects = vec![];
        let mut rng = rand::thread_rng();
        for _idx in 0..64000 {
            objects.push(Object::Scenery(object::Scenery {
                texture: "resources/birb.png".to_string(),
                uv: [[0.0, 0.0], [0.5, 0.0], [0.5, 0.5], [0.0, 0.5]],
                position: Position {
                    x: rng.gen_range(0..640),
                    y: rng.gen_range(0..360),
                },
                size: [1, 1],
            }));
        }

        GameSystem {
            gc,
            io,
            expanse: Expanse {
                chunks: vec![Chunk {
                    objects,
                    size: [64, 64],
                    position: Position { x: 0, y: 0 },
                }],
            },
            camera: Position { x: 0, y: 0 },
            compositor,
        }
    }

    #[profiling::function]
    pub fn handle_events(&mut self, event: &Event<()>) -> ShouldQuit {
        let has_events = self.io.controls.input_helper.update(event);

        // if events cleared
        if has_events {
            profiling::scope!("Main Thread");
            self.update();
            self.draw();
            self.render();
            profiling::finish_frame!();
        }

        let input_helper = &mut self.io.controls.input_helper;

        if let Some(size) = input_helper.window_resized() {
            self.gc.resize(size);
        }

        if input_helper.quit() {
            ShouldQuit::True
        } else {
            ShouldQuit::False
        }
    }

    #[profiling::function]
    fn render(&mut self) {
        let frame_tex = {
            let frame = self.gc.surface.get_current_frame();

            match frame {
                Ok(_f) => _f,
                Err(SurfaceError::Outdated) => {
                    self.gc.surface.configure(&self.gc.device, &self.gc.config);
                    self.gc
                        .surface
                        .get_current_frame()
                        .expect("swapchain failed to get current frame (twice)")
                }
                Err(SurfaceError::Timeout) => {
                    return; /*assume gpu is asleep?*/
                }
                _ => frame.expect("swapchain failed to get current frame"),
            }
        }
        .output;

        let mut encoder = {
            self.gc
                .device
                .create_command_encoder(&CommandEncoderDescriptor {
                    label: Some("Command Encoder"),
                })
        };

        self.compositor
            .render(&mut self.gc, &mut encoder, &frame_tex);

        self.gc.queue.submit(std::iter::once(encoder.finish()));
    }

    #[profiling::function]
    fn update(&mut self) {}

    #[profiling::function]
    fn draw(&mut self) {
        self.compositor.reset();
        // for each loaded chunk (check camera position),
        // place each object

        let loaded_chunks: Vec<Chunk> = {
            profiling::scope!("Cull Chunks");
            self
                .expanse
                .chunks
                .iter()
                .filter(|chunk| {
                    // TODO: bounds check better
                    let test = chunk.position.x;
                    let starts_in_cam_x = test >= self.camera.x && test <= self.camera.x + 640;
                    let test = chunk.position.x + (chunk.size[0]);
                    let ends_in_cam_x = test >= self.camera.x && test <= self.camera.x + 640;

                    let test = chunk.position.y;
                    let starts_in_cam_y = test >= self.camera.y && test <= self.camera.y + 360;
                    let test = chunk.position.y + (chunk.size[1]);
                    let ends_in_cam_y = test >= self.camera.y && test <= self.camera.y + 360;

                    starts_in_cam_x || starts_in_cam_y || ends_in_cam_x || ends_in_cam_y
                })
                .map(|chunk| {
                    let mut chunk = chunk.clone();
                    chunk.position.x -= self.camera.x;
                    chunk.position.y -= self.camera.y;
                    chunk
                })
                .collect()
        };

        for chunk in loaded_chunks {
            profiling::scope!("for chunk in loaded_chunks");
            for object in chunk.objects {
                profiling::scope!("add objects to compositor");
                match object {
                    Scenery(o) => self.compositor.add_object(o.position, o.size, o.uv),
                    Object::Character => {}
                }
            }
        }
    }
}

pub enum ShouldQuit {
    True,
    False,
}
