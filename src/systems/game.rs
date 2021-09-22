use crate::graphics::{GraphicsContext, VertexData};
use crate::systems::controls::Controls;
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use wgpu::{
    BlendState, BufferDescriptor, BufferUsages, Color, ColorTargetState, ColorWrites,
    CommandEncoderDescriptor, FragmentState, IndexFormat, LoadOp, MultisampleState,
    PipelineLayoutDescriptor, PolygonMode, PrimitiveState, PrimitiveTopology,
    RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor,
    ShaderModuleDescriptor, ShaderSource, SurfaceError, TextureViewDescriptor, VertexState,
};
use winit::event::Event;
use winit::window::Window;

pub struct IO {
    pub ticks: u64,
    pub controls: Controls,
    // pub audio_tx: Sender<AudioSysMsg>,
    // pub draw_queue: VecDeque<DrawCommand>,
}

pub struct GameSystem {
    pub gc: GraphicsContext,
    pub io: IO,
}

impl GameSystem {
    pub fn new(window: Window) -> Self {
        let gc = GraphicsContext::new(window);

        let controls = Controls::default();
        // todo: audio

        let io = IO { ticks: 0, controls };

        GameSystem { gc, io }
    }

    pub fn handle_events(&mut self, event: &Event<()>) -> ShouldQuit {
        // if events cleared
        if self.io.controls.input_helper.update(event) {
            // TODO: update and draw
            self.render();
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

    pub fn render(&mut self /*, nothing else here?*/) {
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

        let view = &frame_tex
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let tris = vec![
            VertexData {
                position: [-1.0, -1.0, 0.0],
            },
            VertexData {
                position: [-1.0, 3.0, 0.0],
            },
            VertexData {
                position: [3.0, -1.0, 0.0],
            },
        ];

        let vertexbuf = self.gc.device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&tris), // <- insert vertex data
            usage: BufferUsages::VERTEX,
        });

        let shader = self
            .gc
            .device
            .create_shader_module(&ShaderModuleDescriptor {
                label: None,
                // TODO: put shader somewhere else
                source: ShaderSource::Wgsl(include_str!("../shader.wgsl").into()),
            });

        let layout = self
            .gc
            .device
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[],
                push_constant_ranges: &[],
            });

        let pipeline = self
            .gc
            .device
            .create_render_pipeline(&RenderPipelineDescriptor {
                label: None,
                layout: Some(&layout),
                vertex: VertexState {
                    module: &shader,
                    entry_point: "vert_main",
                    buffers: &[VertexData::layout()],
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
                    targets: &[self.gc.config.format.into()],
                }),
            });

        // create render pass and pipeline
        {
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

            render_pass.set_pipeline(&pipeline);
            render_pass.set_vertex_buffer(0, vertexbuf.slice(..));
            render_pass.draw(0..3, 0..1);
        }

        self.gc.queue.submit(std::iter::once(encoder.finish()));
    }
}

pub enum ShouldQuit {
    True,
    False,
}
