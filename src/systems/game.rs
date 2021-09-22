use crate::graphics::GraphicsContext;
use crate::systems::controls::Controls;
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
                Err(Outdated) => {
                    self.gc.surface.configure(&self.gc.device, &self.gc.config);
                    self.gc
                        .surface
                        .get_current_frame()
                        .expect("swapchain failed to get current frame (twice)")
                }
                Err(Timeout) => {
                    return; /*assume gpu is asleep?*/
                }
                _ => frame.expect("swapchain failed to get current frame"),
            }
        }
        .output;
    }
}

pub enum ShouldQuit {
    True,
    False,
}
