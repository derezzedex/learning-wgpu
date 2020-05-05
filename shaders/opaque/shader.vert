#version 450

layout(location=0) in vec3 position;
layout(location=1) in vec2 tex_coords;

layout(location=0) out vec2 f_tex_coords;

layout(set=1, binding=0)
uniform Uniforms {
  mat4 model;
  mat4 view;
  mat4 projection;
};

void main() {
  f_tex_coords = tex_coords;
  // gl_Position = view * projection * vec4(position, 1.0);
  gl_Position =  projection * view * model * vec4(position, 1.0);
}
