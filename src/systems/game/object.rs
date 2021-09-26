#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Position {
    pub x: u32,
    pub y: u32,
}

#[derive(Debug, Clone)]
pub enum Object {
    Scenery(Scenery),
    Character, // PC or otherwise
}

#[derive(Debug, Clone)]
pub struct Scenery {
    // would likely have a specific collision mode here too
    pub texture: String,
    pub uv: [[f32; 2]; 4],
    pub position: Position, // relative to chunk
    pub size: [u32; 2],
}

#[derive(Debug, Clone)]
pub struct Chunk {
    pub objects: Vec<Object>,
    pub size: [u32; 2],
    pub position: Position,
}

// TODO: maybe there should be a zone with one solid background that contains chunks?

pub struct Expanse {
    pub chunks: Vec<Chunk>, // list of chunks with offset
}
