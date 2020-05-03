#version 450

layout(location=0) in vec3 position;
layout(location=1) in vec2 tex_coords;

layout(location=0) out vec2 f_tex_coords;

void main() {
  f_tex_coords = tex_coords;
  gl_Position = vec4(position, 1.0);
}
