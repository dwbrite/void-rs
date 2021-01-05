use crate::graphics::text::BasicText;

pub enum DrawCommand {
    DrawBg,
    DrawChar,
    DrawString(BasicText),
    //TODO: drawsprite?
}