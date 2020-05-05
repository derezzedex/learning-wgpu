#version 450

layout(location=0) out vec4 color;

layout(set=0, binding=0) uniform texture2D t_tex;
layout(set=0, binding=1) uniform sampler s_tex;

void main(){
  color = vec4(1.0, 0.0, 0.0, 1.0);
}
