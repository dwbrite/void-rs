use crate::graphics::{FrameContext, GraphicsContext};
use crate::resources;
use wgpu_glyph::{ab_glyph, Extra, GlyphBrush, GlyphBrushBuilder, Section, Text};

pub struct TextRenderContext {
    glyph_brush: GlyphBrush<()>,
    sections: Vec<Section<'static, Extra>>,
}

#[derive(Debug, Clone)]
pub struct BasicText {
    pub pos: (f32, f32),
    pub str: String,
    pub color: [f32; 4],
}

impl TextRenderContext {
    pub fn build(ctx: &GraphicsContext) -> TextRenderContext {
        let font = ab_glyph::FontArc::try_from_slice(resources::FONT).expect("Load font");

        let glyph_brush =
            GlyphBrushBuilder::using_font(font).build(&ctx.device, ctx.sc_desc.format);

        TextRenderContext {
            glyph_brush,
            sections: Vec::with_capacity(64),
        }
    }

    pub fn draw(&mut self, f_ctx: &mut FrameContext, text: BasicText) {
        let section = Section {
            screen_position: text.pos,
            text: vec![Text::new(&text.str).with_scale(8.0).with_color(text.color)],
            ..Section::default()
        };

        self.glyph_brush.queue(section);

        let mut staging_belt = wgpu::util::StagingBelt::new(0x400);

        self.glyph_brush
            .draw_queued(
                &f_ctx.ctx.device,
                &mut staging_belt,
                f_ctx.encoder,
                &f_ctx.frame_tex.view,
                f_ctx.ctx.size.width,
                f_ctx.ctx.size.height,
            )
            .expect("fix your shit bruh");
    }
}
