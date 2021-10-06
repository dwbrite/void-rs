[[stage(fragment)]]
fn main([[builtin(position)]] pos: vec4<f32>) -> [[location(0)]] vec4<f32> {
    return vec4<f32>(1.0, 1.0, 1.0, 1.0);
}

[[block]]
struct Position {
    data: array<vec2<u32>, 64000>;
};

[[group(0), binding(0)]]
var<storage,read> buf1: Position;

[[block]]
struct UV {
    data: array<mat2x4<f32>, 64000>;
};

[[group(0), binding(1)]]
var<storage,read> buf2: UV;

[[block]]
struct Size {
    data: array<vec2<u32>, 64000>;
};

[[group(0), binding(2)]]
var<storage,read> buf3: Size;


[[stage(vertex)]]
fn vert_main([[builtin(vertex_index)]] vert_idx: u32) -> [[builtin(position)]] vec4<f32> {
    let obj_idx = u32(floor(f32(vert_idx) / 6.0));
    let rel_idx = vert_idx % 6u;

    let pos = buf1.data[obj_idx];
    var vertex = vec4<f32>(0.0, 0.0, 0.0, 1.0);

    let size = buf3.data[obj_idx];


    if (rel_idx == 0u || rel_idx == 3u ) {
        vertex.x = f32(pos.x + size.x);
        vertex.y = f32(pos.y);
    } elseif (rel_idx == 1u) {
        vertex.x = f32(pos.x);
        vertex.y = f32(pos.y);
    } elseif (rel_idx == 2u || rel_idx == 4u ) {
        vertex.x = f32(pos.x);
        vertex.y = f32(pos.y + size.y);
    } elseif (rel_idx == 5u) {
        vertex.x = f32(pos.x + size.x);
        vertex.y = f32(pos.y + size.y);
    }

    vertex.x = vertex.x / 640.0;
    vertex.y = vertex.y / 360.0;
    vertex.x = vertex.x * 2.0 - 1.0;
    vertex.y = vertex.y * 2.0 - 1.0;

    return vertex;
}
