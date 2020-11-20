// shader.frag
#version 450

layout(location=0) in vec3 v_color;
layout(location=1) in vec2 v_tex_coords;

layout(location=0) out vec4 f_color;

layout(set = 0, binding = 0) uniform texture2D t_diffuse;
layout(set = 0, binding = 1) uniform sampler s_diffuse;

void main() {
    vec2 tex_coords = vec2(v_tex_coords.x/2 + 0.5, (-v_tex_coords.y/2) + 0.5);
    f_color = texture(sampler2D(t_diffuse, s_diffuse), tex_coords);
}
