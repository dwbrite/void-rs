use winit::event::{WindowEvent, ElementState};
use winit::window::Window;
use crate::graphics::{GraphicsContext, FrameContext};
use crate::graphics::background::BgRenderContext;
use crate::graphics::text::{TextRenderContext};
use crate::systems::controls::Controls;
use crate::systems::audio::{AudioSystem, AudioSysMsg};
use crossbeam_channel::Sender;
use crate::graphics::draw::DrawCommand;
use std::collections::VecDeque;

pub struct IO {
    pub ticks: u64,
    pub controls: Controls,
    pub audio_tx: Sender<AudioSysMsg>,
    pub draw_queue: VecDeque<DrawCommand>,
}

pub struct GameSystem {
    pub gc: GraphicsContext,
    bg_render: BgRenderContext,
    text_render: TextRenderContext,
    pub io: IO,
    // dialogue: DialogueSystem,
}

impl GameSystem {
    pub async fn new(window: Window) -> Self {
        let gc = GraphicsContext::new(window).await;
        let bg_render = BgRenderContext::build(&gc);
        let text_render = TextRenderContext::build(&gc);

        let controls = Controls::default();
        let audio_tx =  AudioSystem::start();

        // let dialogue = DialogueSystem::default();

        let io = IO {
            ticks: 0,
            controls,
            audio_tx,
            draw_queue: VecDeque::with_capacity(64)
        };

        GameSystem {
            gc,
            bg_render,
            text_render,
            io,
            // dialogue,
        }
    }

    pub fn recreate_swapchain(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.gc.size = new_size;
        self.gc.sc_desc.width = new_size.width;
        self.gc.sc_desc.height = new_size.height;
        self.gc.swap_chain = self.gc.device.create_swap_chain(&self.gc.surface, &self.gc.sc_desc);
    }

    pub fn handle_input_events(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::KeyboardInput { input, ..  } => {
                match input.state {
                    ElementState::Pressed => {
                        if let Some(keycode) = input.virtual_keycode {
                            self.io.controls.key_pressed(keycode);
                        }
                    }
                    ElementState::Released => {
                        if let Some(keycode) = input.virtual_keycode {
                            self.io.controls.key_released(keycode);
                        }
                    }
                }
            }
            // WindowEvent::ModifiersChanged(_) => {}
            _ => {}
        }
        false
    }

    pub fn update(&mut self) {
        self.io.ticks +=1;
        // self.dialogue.update(&mut self.io);
    }

    pub fn draw(&mut self) {
        self.io.draw_queue.push_back(DrawCommand::DrawBg);
        // self.dialogue.draw(&mut self.io);
    }

    pub fn render(&mut self) {
        let frame_tex = {
            let frame = self.gc.swap_chain.get_current_frame();
            use wgpu::SwapChainError::*;
            match frame {
                Ok(_f) => { _f }
                Err(Outdated) => {
                    self.recreate_swapchain(self.gc.size);
                    self.gc.swap_chain.get_current_frame()
                        .expect("swapchain failed to get current frame (twice)")
                }
                Err(Timeout) => { return /*assume gpu is asleep?*/ }
                _ => { frame.expect("swapchain failed to get current frame") }
            }
        }.output;

        let mut encoder = self.gc
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });


        let mut f_ctx = FrameContext {
            ctx: &self.gc,
            encoder: &mut encoder,
            frame_tex: &frame_tex,
        };

        while !self.io.draw_queue.is_empty() {
            match self.io.draw_queue.pop_front().unwrap() {
                DrawCommand::DrawBg => { self.bg_render.draw(&mut f_ctx); }
                DrawCommand::DrawChar => {}
                DrawCommand::DrawString(txt) => { self.text_render.draw(&mut f_ctx, txt); }
            }
        }

        self.gc.queue.submit(std::iter::once(encoder.finish()));
    }
}
