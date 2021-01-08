use crate::dialogue::DialogueSpan::Text;
use crate::graphics::draw::DrawCommand::DrawString;
use crate::graphics::text::BasicText;
use crate::systems::audio::AudioSysMsg;
use crate::systems::game::IO;
use ir::ast;
use ir::ast::LineChild::Span;
use ir::ast::{Action, ChExpr, Instruction, LineChild, TextProperties};
use std::str::Chars;
use std::vec::IntoIter;

#[derive(Debug)]
pub struct LineBuffer {
    lines: Vec<Option<DialogueLine>>,
}

impl LineBuffer {
    fn new(size: usize) -> Self {
        let mut lines = vec![];
        for _ in 0..size {
            lines.push(None);
        }

        Self { lines }
    }

    fn push(&mut self, line: DialogueLine) {
        let size = self.lines.len();

        for i in 0..size {
            if i == size - 1 {
                self.lines[i] = Some(line);
                break;
            } else {
                self.lines.swap(i, i + 1);
            }
        }
    }

    fn clear(&mut self) {
        for item in &mut self.lines {
            item.take();
        }
    }

    fn replace_last(&mut self, line: DialogueLine) {
        let old_line = self.lines.last_mut().unwrap();
        *old_line = Some(line);
    }
}

#[derive(Debug, Clone)]
enum DialogueSpan {
    Text(BasicText),
    Instruction(ast::Instruction),
}

#[derive(Debug, Clone)]
pub struct DialogueLine {
    content: Vec<DialogueSpan>,
}

pub struct DialogueSystem {
    // chapter: ast::Chapter,
    it: IntoIter<ast::ChExpr>,
    linebuf: LineBuffer,
    directive: Directive,
    voice: String,
}

#[derive(Debug)]
enum Directive {
    Await,
    OutputLine(OutputLine),
    None,
}

#[derive(Debug)]
struct SpanIter {
    char_iter: IntoIter<char>,
    properties: TextProperties,
}

#[derive(Debug)]
struct OutputLine {
    it: IntoIter<LineChild>,
    out: DialogueLine,
    next_update: u64,
    wip: Option<SpanIter>,
}

impl DialogueSystem {
    pub fn init(chapter: ast::Chapter) -> Self {
        let voice = chapter.voice;
        let it = chapter.content.into_iter();

        Self {
            // chapter,
            it,
            linebuf: LineBuffer::new(4),
            directive: Directive::None,
            voice,
        }
    }

    pub fn update(&mut self, io: &mut IO) {
        match &mut self.directive {
            Directive::Await => {
                if io.controls.enter {
                    self.directive = Directive::None;
                }
            }
            Directive::OutputLine(_) => {
                self.update_line(io);
            }
            Directive::None => {
                self.next_directive();
                println!("{:?}", self.directive)
            }
        }
    }

    fn update_line(&mut self, io: &mut IO) {
        let mut new_directive = None;
        let mut retry = false;
        if let Directive::OutputLine(line) = &mut self.directive {
            if io.ticks >= line.next_update {
                if let Some(span_iter) = &mut line.wip {
                    if let Some(ch) = span_iter.char_iter.next() {
                        if let Some(mut span) = line.out.content.last_mut() {
                            if let Text(t) = &mut span {
                                t.str.push(ch);
                            }
                            line.next_update =
                                io.ticks + (2 * (6 - span_iter.properties.speed) as u64);
                        } else {
                            line.out.content.push(DialogueSpan::Text(BasicText {
                                pos: (0.0, 0.0),
                                str: ch.to_string(),
                                color: [1.0, 1.0, 1.0, 1.0], // TODO: voice, smh
                            }));
                            line.next_update =
                                io.ticks + (2 * (6 - span_iter.properties.speed) as u64);
                        }

                        if ch == ' ' {
                            line.next_update = io.ticks + 2;
                            retry = true;
                        } else {
                            io.audio_tx.send(AudioSysMsg::PlayEffect(0));
                        }
                    } else {
                        line.wip = None;
                    }
                } else {
                    if let Some(child) = line.it.next() {
                        match child {
                            Span(s) => {
                                let char_iter = s.text.chars().collect::<Vec<_>>().into_iter();
                                let properties = s.properties;
                                line.wip = Some(SpanIter {
                                    char_iter,
                                    properties,
                                });
                            }
                            LineChild::Instruction(i) => {
                                // TODO: do instruction
                                // TODO: call this fn again
                                // line.update(io, directive);
                            }
                        }
                    } else {
                        new_directive = Some(Directive::None);
                    }
                }
            }

            self.linebuf.replace_last(line.out.clone());
        }
        if let Some(directive) = new_directive {
            self.directive = directive;
        }
        if retry {
            self.update_line(io);
        }
    }

    fn next_directive(&mut self) {
        match self.it.next().expect("new chapter should open before eof") {
            ChExpr::Action(action) => match action {
                Action::Await => self.directive = Directive::Await,
            },
            ChExpr::Instruction(instruction) => match instruction {
                Instruction::Play { sound } => {
                    // TODO: play a sound
                }
            },
            ChExpr::Line { content } => {
                self.directive = Directive::OutputLine(OutputLine {
                    it: content.into_iter(),
                    out: DialogueLine { content: vec![] },
                    next_update: 0,
                    wip: None,
                });
                self.linebuf.push(DialogueLine { content: vec![] });
            }
        }
    }

    pub fn draw(&mut self, io: &mut IO) {
        for (idx, line) in self.linebuf.lines.iter().enumerate() {
            if line.is_none() {
                continue;
            }

            let y: f32 = 226.0 + (idx * 12) as f32;
            let mut x: f32 = 12.0;
            if let Some(lineee) = &line {
                for span in &lineee.content {
                    if let Text(tmp) = span {
                        let text = BasicText {
                            pos: (x + tmp.pos.0, y + tmp.pos.1),
                            str: tmp.str.clone(),
                            color: tmp.color,
                        };

                        x += (text.str.len() * 8) as f32;

                        io.draw_queue.push_back(DrawString(text));
                    }
                }
            }
        }
    }
}
