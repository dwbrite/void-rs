use winit::event::WindowEvent;
use winit::window::Window;
use crate::graphics::{GraphicsContext, FrameContext};
use crate::graphics::background::BgRenderContext;
use crate::graphics::text::{TextRenderContext, BasicText};

pub(crate) struct GameState {
    pub(crate) ctx: GraphicsContext,
    idk_bg: BgRenderContext,
    txt_lol: TextRenderContext,
}

impl GameState {
    pub(crate) async fn new(window: Window) -> Self {
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
        self.txt_lol.draw(&mut f_ctx, BasicText {
            pos: (0.0, 0.0),
            str: "idk bro".to_string(),
            color: [1.0, 1.0, 1.0, 1.0],
        });


        self.ctx.queue.submit(std::iter::once(encoder.finish()));
    }
}
