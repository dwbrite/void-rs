use winit::event::{WindowEvent, ElementState};
use winit::window::Window;
use crate::graphics::{GraphicsContext, FrameContext};
use crate::graphics::background::BgRenderContext;
use crate::graphics::text::{TextRenderContext, BasicText};
use crate::systems::controls::Controls;
use crate::systems::audio::{AudioSystem, AudioSysMsg};
use crossbeam_channel::Sender;
use crate::graphics::draw::DrawCommand;
use std::collections::VecDeque;

pub struct GameSystem {
    pub(crate) gc: GraphicsContext,
    bg_render: BgRenderContext,
    text_render: TextRenderContext,
    controls: Controls,
    _audio_tx: Sender<AudioSysMsg>,
    pub(crate) _ticks: u64,
    draw_queue: VecDeque<DrawCommand>,
}

impl GameSystem {
    pub async fn new(window: Window) -> Self {
        let gc = GraphicsContext::new(window).await;
        let bg_render = BgRenderContext::build(&gc);
        let text_render = TextRenderContext::build(&gc);

        let controls = Controls::default();
        let audio_tx =  AudioSystem::start();

        GameSystem {
            gc,
            bg_render,
            text_render,
            controls,
            _audio_tx: audio_tx,
            _ticks: 0,
            draw_queue: VecDeque::with_capacity(64),
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
                            self.controls.key_pressed(keycode);
                        }
                    }
                    ElementState::Released => {
                        if let Some(keycode) = input.virtual_keycode {
                            self.controls.key_released(keycode);
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
        self._ticks+=1;

        self.draw_queue.push_back(DrawCommand::DrawBg);
        self.draw_queue.push_back(DrawCommand::DrawString(BasicText{
            pos: (16.0, 288.0 - 20.0),
            str: "o hej".to_string(),
            color: [1.0, 1.0, 1.0, 1.0],
        }))
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

        while !self.draw_queue.is_empty() {
            match self.draw_queue.pop_front().unwrap() {
                DrawCommand::DrawBg => { self.bg_render.draw(&mut f_ctx); }
                DrawCommand::DrawChar => {}
                DrawCommand::DrawString(txt) => { self.text_render.draw(&mut f_ctx, txt); }
            }
        }

        // self.bg_render.draw(&mut f_ctx);
        // self.text_render.draw(&mut f_ctx, BasicText {
        //     pos: (0.0, 0.0),
        //     str: "idk bro".to_string(),
        //     color: [1.0, 1.0, 1.0, 1.0],
        // });


        self.gc.queue.submit(std::iter::once(encoder.finish()));
    }
}
