use crate::graphics::text::BasicText;

pub(crate) enum DrawCommand {
    DrawBg,
    DrawChar,
    DrawString(BasicText),
    //TODO: drawsprite?
}