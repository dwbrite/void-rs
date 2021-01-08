pub mod ast {
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
    pub enum Instruction {
        Play { sound: String },
    }

    #[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
    pub enum LineChild {
        Span(Span),
        Instruction(Instruction),
    }

    // enums
    #[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
    pub enum Action {
        Await,
    }

    #[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
    pub enum ChExpr {
        Action(Action),
        Instruction(Instruction),
        Line { content: Vec<LineChild> },
    }

    // you know what it is 😎
    #[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
    pub struct Chapter {
        pub voice: String,
        pub content: Vec<ChExpr>,
    }

    #[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
    pub enum Props {
        Speed(u32),
    }

    #[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
    pub struct TextProperties {
        pub speed: u32,
    }

    #[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
    pub struct Span {
        pub text: String,
        pub properties: TextProperties,
    }
}
