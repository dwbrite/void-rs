use crate::graphics::sprite::Sprite;
use crate::graphics::GraphicsContext;
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

    // TODO: remove this vvvvvv
    pub sprite: Sprite,
}

impl GameSystem {
    pub fn new(window: Window) -> Self {
        let mut gc = GraphicsContext::new(window);

        let controls = Controls::default();
        // todo: audio

        let io = IO { ticks: 0, controls };

        let sprite = Sprite::new(&mut gc);

        GameSystem { gc, io, sprite }
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

    pub fn render(&mut self) {
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

        self.sprite.draw(&mut encoder, &frame_tex);

        self.gc.queue.submit(std::iter::once(encoder.finish()));
    }
}

pub enum ShouldQuit {
    True,
    False,
}
