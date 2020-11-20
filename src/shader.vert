#version 450

layout(location=0) in vec3 a_position;

layout(location=0) out vec3 v_color;
layout(location=1) out vec2 v_tex_coords;

void main() {
    v_color = vec3(0.1, 0.0, 0.2);
    v_tex_coords = a_position.xy;
    gl_Position = vec4(a_position, 1.0);
}