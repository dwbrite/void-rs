use cgmath::Vector2;


#[derive(Debug, Clone)]
pub struct Chunk {
    pub size: Vector2<u32>,
    pub position: Vector2<i32>,
}

pub struct Expanse {
    pub chunks: Vec<Chunk>, // list of chunks with offset
}
