mod compositor;
mod object;

use crate::graphics::GraphicsContext;
use crate::systems::controls::Controls;
use crate::systems::game::object::{Chunk, Expanse};

use wgpu::{CommandEncoderDescriptor, SurfaceError};

use cgmath::Vector2;

use crate::systems::game::compositor::Compositor;

use winit::event::Event;
use winit::window::Window;

pub struct IO {
    pub ticks: u64,
    pub controls: Controls,
    // pub audio_tx: Sender<AudioSysMsg>,
    // pub draw_queue: VecDeque<DrawCommand>,
}

// #[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
// #[repr(C)]
// pub struct Obj {
//     pos: [u32; 2],
//     size: [u32; 2],
//     uv: [[f32; 2]; 4],
// }

pub struct GameSystem {
    pub gc: GraphicsContext,
    pub io: IO,
    pub expanse: Expanse,
    pub camera: Vector2<i32>,
    pub compositor: Compositor,
}

impl GameSystem {
    pub fn new(window: Window) -> Self {
        let mut gc = GraphicsContext::new(window);

        let controls = Controls::default();

        let io = IO { ticks: 0, controls };

        let compositor = Compositor::new(&mut gc);

        GameSystem {
            gc,
            io,
            expanse: Expanse {
                chunks: vec![Chunk {
                    size: Vector2::new(64, 64),
                    position: Vector2::new(0, 0),
                }],
            },
            camera: Vector2::new(0, 0),
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
            let frame = self.gc.surface.get_current_texture();

            match frame {
                Ok(_f) => _f,
                Err(SurfaceError::Outdated) => {
                    self.gc.surface.configure(&self.gc.device, &self.gc.config);
                    self.gc
                        .surface
                        .get_current_texture()
                        .expect("swapchain failed to get current frame (twice)")
                }
                Err(SurfaceError::Timeout) => {
                    return; /*assume gpu is asleep?*/
                }
                _ => frame.expect("swapchain failed to get current frame"),
            }
        };

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
        frame_tex.present();
    }

    #[profiling::function]
    fn update(&mut self) {}

    #[profiling::function]
    fn draw(&mut self) {
        let _loaded_chunks: Vec<Chunk> = {
            profiling::scope!("Cull Chunks");
            self.expanse
                .chunks
                .iter()
                .filter(|chunk| {
                    // TODO: bounds check better
                    let test = chunk.position.x;
                    let starts_in_cam_x = test >= self.camera.x && test <= self.camera.x + 640;
                    let test = chunk.position.x + (chunk.size[0] as i32);
                    let ends_in_cam_x = test >= self.camera.x && test <= self.camera.x + 640;

                    let test = chunk.position.y;
                    let starts_in_cam_y = test >= self.camera.y && test <= self.camera.y + 360;
                    let test = chunk.position.y + (chunk.size[1] as i32);
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
    }
}

pub enum ShouldQuit {
    True,
    False,
}
