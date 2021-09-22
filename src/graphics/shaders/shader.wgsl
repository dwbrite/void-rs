[[stage(fragment)]]
fn main([[builtin(position)]] pos: vec4<f32>) -> [[location(0)]] vec4<f32> {
    return vec4<f32>(pos.xyz, 1.0);
}

[[stage(vertex)]]
fn vert_main([[location(0)]] input: vec3<f32>, [[location(1)]] tex_coords: vec2<f32>) -> [[builtin(position)]] vec4<f32> {
    return vec4<f32>(input, 1.0);
}
