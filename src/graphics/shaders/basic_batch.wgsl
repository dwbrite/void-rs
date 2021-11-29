[[stage(fragment)]]
fn main([[builtin(position)]] pos: vec4<f32>) -> [[location(0)]] vec4<f32> {
    return vec4<f32>(1.0, 1.0, 1.0, 1.0);
}

[[stage(vertex)]]
fn vert_main([[builtin(vertex_index)]] vert_idx: u32) -> [[builtin(position)]] vec4<f32> {
//    let obj_idx = u32(floor(f32(vert_idx) / 6.0));
//    let rel_idx = vert_idx % 6u;
//
//    let obj = objects.data[obj_idx];
//
//    let pos = obj.pos;
//    let size = obj.size;

    var vertex = vec4<f32>(0.0, 0.0, 0.0, 1.0);
//
//    if (rel_idx == 0u || rel_idx == 3u ) {
//        vertex.x = f32(pos.x + size.x);
//        vertex.y = f32(pos.y);
//    } elseif (rel_idx == 1u) {
//        vertex.x = f32(pos.x);
//        vertex.y = f32(pos.y);
//    } elseif (rel_idx == 2u || rel_idx == 4u ) {
//        vertex.x = f32(pos.x);
//        vertex.y = f32(pos.y + size.y);
//    } elseif (rel_idx == 5u) {
//        vertex.x = f32(pos.x + size.x);
//        vertex.y = f32(pos.y + size.y);
//    }
//
//    vertex.x = vertex.x / 640.0;
//    vertex.y = vertex.y / 360.0;
//    vertex.x = vertex.x * 2.0 - 1.0;
//    vertex.y = vertex.y * 2.0 - 1.0;

    return vertex;
}
