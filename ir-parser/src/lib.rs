#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(dead_code)]

use std::fs::File;
use std::path::Path;
use std::rc::Rc;

use roxmltree;
use roxmltree::{ExpandedName, Node, NodeType};

use ir::ast;
use ir::ast::Action::Await;
use ir::ast::ChExpr::{Action, Line};
use ir::ast::Instruction::Play;
use ir::ast::{ChExpr, Chapter, Instruction, LineChild, Props, Span, TextProperties};
use std::io::{Read, Write};

// 🦆
// the idea of the DialogueIntermediate is that I want to store
// an ordered list of Expressions which represent text or control flow.
//
// A chapter expression must be either:
// - a line
// - an await
// - [an instruction]
// - [a prompt]
// - [a jump/goto?]
//
// Text is generally/always represented as lines.
// A line is an ordered list of partial phrases.
// A partial phrase is:
// - text with some properties, or
// - [an instruction (e.g., play a sound, shake the screen - things of that nature)]
//
// Text properties can be:
// - speed,
// - [etc]
//

// ChapterParser reads an xml file and turns it into the appropriate

pub fn compile_ir(in_path: &Path, mut chapter: File, out_path: &Path) {
    // in_path might be used for relative file locations
    let mut s = String::new();
    let _ = chapter.read_to_string(&mut s);

    let parser = ChapterParser::from(s.as_str());
    let encoded = bincode::serialize(&parser.chapter).unwrap();

    // TODO: fix
    let mut file = File::create(Path::new("../game/dialogue/en/intro.bincode")).unwrap();
    file.write_all(&encoded).expect("couldn't write file???");
}

pub struct ChapterParser<'a> {
    doc: Rc<roxmltree::Document<'a>>,
    chapter: Option<ast::Chapter>,
    expr_stack: Option<Vec<ast::ChExpr>>,
    prop_stack: Vec<ast::Props>,
}

impl<'a> ChapterParser<'a> {
    pub fn from(source: &'a str) -> Self {
        let doc = Rc::new(roxmltree::Document::parse(source).expect("invalid xml"));

        let mut parser = Self {
            doc,
            chapter: None,
            expr_stack: Some(vec![]),
            prop_stack: vec![],
        };

        parser.parse();
        parser
    }

    pub fn parse(&mut self) {
        let doc = self.doc.clone();
        let e = doc.root_element();
        if e.tag_name().name() == "chapter" {
            self.parse_chapter(self.doc.clone().root_element());
        } else {
            panic!("xyz");
        }
    }

    pub fn parse_chapter(&mut self, node: roxmltree::Node) {
        let voice = node.attribute("voice").unwrap();

        for child in node.children() {
            self.parse_chexpr(child);
        }

        self.chapter = Some(Chapter {
            voice: voice.to_string(),
            content: self.expr_stack.take().unwrap(),
        });
    }

    fn parse_chexpr(&mut self, node: Node) {
        let t = node.node_type();
        match t {
            NodeType::Element => self.parse_element(node),
            NodeType::PI => {
                let pi = self.parse_instruction(node);
                if let Some(stack) = &mut self.expr_stack {
                    stack.push(ChExpr::Instruction(pi));
                }
            }
            NodeType::Text => {
                if !Self::text_is_whitespace(&node) {
                    panic!("text cannot be the child of a chapter");
                }
            }
            NodeType::Comment => {}
            _ => panic!("illegal element: {:?}", node),
        }
    }

    fn parse_element(&mut self, node: Node) {
        match node.tag_name().name() {
            // style
            "s0" | "s1" | "s2" | "s3" | "s4" | "s5" => {}
            // structure
            "line" => {
                self.parse_line(node);
            }
            "await" => {
                self.parse_await(node);
            }
            _s => unimplemented!("<{}> not implemented", _s),
        }
    }

    fn parse_instruction(&mut self, node: Node) -> Instruction {
        let pi = node.pi().unwrap();
        match pi.target {
            "play" => Play {
                sound: pi.value.unwrap().to_string(),
            },
            _ => {
                panic!("unsupported processing instruction: {:?}", pi);
            }
        }
    }

    fn parse_line(&mut self, node: Node) {
        if let Some(stack) = &mut self.expr_stack {
            stack.push(Line { content: vec![] });
        }

        for child in node.children() {
            self.parse_line_child(child);
        }
    }

    fn parse_line_child(&mut self, node: Node) {
        match node.node_type() {
            NodeType::Element => self.parse_property(node),
            NodeType::PI => {
                let pi = self.parse_instruction(node);
                if let Some(stack) = &mut self.expr_stack {
                    if let Line { content } = stack.last_mut().unwrap() {
                        content.push(LineChild::Instruction(pi));
                    }
                }
            }
            NodeType::Text => {
                if !Self::text_is_whitespace(&node) {
                    let mut span = Span {
                        text: node.text().unwrap().to_string(),
                        properties: TextProperties { speed: 3 }, // default speed of 3
                    };

                    for prop in &self.prop_stack {
                        match prop {
                            Props::Speed(n) => {
                                span.properties.speed = n.clone();
                            }
                        }
                    }

                    if let Some(stack) = &mut self.expr_stack {
                        if let ChExpr::Line { content } = stack.last_mut().unwrap() {
                            content.push(LineChild::Span(span));
                        }
                    }
                }
            }
            _ => println!("possibly malformed line, found: {:?}", node), // warn about other shit
        }
    }

    fn parse_property(&mut self, node: Node) {
        match node.tag_name().name() {
            "s0" => {
                self.prop_stack.push(ast::Props::Speed(0));
            }
            "s1" => {
                self.prop_stack.push(ast::Props::Speed(1));
            }
            "s2" => {
                self.prop_stack.push(ast::Props::Speed(2));
            }
            "s3" => {
                self.prop_stack.push(ast::Props::Speed(3));
            }
            "s4" => {
                self.prop_stack.push(ast::Props::Speed(4));
            }
            _ => {
                panic!("unsupported property {:?}", node);
            }
        }

        for child in node.children() {
            self.parse_line_child(child);
        }

        self.prop_stack.pop();
    }

    fn parse_await(&mut self, node: Node) {
        if let Some(stack) = &mut self.expr_stack {
            stack.push(Action(Await));
        }
    }

    fn text_is_whitespace(node: &Node) -> bool {
        if let Some(t) = node.text() {
            return t.trim().is_empty();
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use crate::ChapterParser;

    // TODO: create tests for each parse function

    #[test]
    fn parse() {
        let p = ChapterParser::from(
            r#"
        <chapter voice="universe">
            <line><s0>...</s0></line><await/>
            <line>The <s4>universe</s4> is silent</line><await/>
            <line><s0>...</s0></line><await/>
            <line>What's <s4>that?</s4></line><await/>
            <line>A faint murmur <s4>masquerades</s4> amongst the <s4>silence.</s4></line><await/>
            <?play song/lowtide?>
            <line>It's <s1>you.</s1></line><await/>
            <line><s0>...</s0></line>
        </chapter>"#,
        );

        println!("{:?}", p.chapter);
    }
}
