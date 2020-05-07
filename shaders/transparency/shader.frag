#version 450

layout(location=0) in vec2 v_tex_coords;

layout(set=0, binding=0) uniform texture2D t_tex;
layout(set=0, binding=1) uniform sampler s_tex;

layout(location=0) out vec4 _accum;
layout(location=1) out float _revealage;

void writePixel(vec4 premultipliedReflect, vec3 transmit, float csZ) {
    /* Modulate the net coverage for composition by the transmission. This does not affect the color channels of the
       transparent surface because the caller's BSDF model should have already taken into account if transmission modulates
       reflection. This model doesn't handled colored transmission, so it averages the color channels. See

          McGuire and Enderton, Colored Stochastic Shadow Maps, ACM I3D, February 2011
          http://graphics.cs.williams.edu/papers/CSSM/

       for a full explanation and derivation.*/

    premultipliedReflect.a *= 1.0 - clamp((transmit.r + transmit.g + transmit.b) * (1.0 / 3.0), 0, 1);

    /* You may need to adjust the w function if you have a very large or very small view volume; see the paper and
       presentation slides at http://jcgt.org/published/0002/02/09/ */
    // Intermediate terms to be cubed
    float a = min(1.0, premultipliedReflect.a) * 8.0 + 0.01;
    float b = -gl_FragCoord.z * 0.95 + 1.0;

    /* If your scene has a lot of content very close to the far plane,
       then include this line (one rsqrt instruction):
       b /= sqrt(1e4 * abs(csZ)); */
    // float w    = clamp(a * a * a * 1e8 * b * b * b, 1e-2, 3e2);
    float w = clamp(pow(min(1.0, premultipliedReflect.a * 10.0) + 0.01, 3.0) * 1e8 * pow(1.0 - gl_FragCoord.z * 0.9, 3.0), 1e-2, 3e3);
    _accum     = premultipliedReflect * w;
    _revealage = premultipliedReflect.a;
}

void main(){
  vec4 color = texture(sampler2D(t_tex, s_tex), v_tex_coords);
  float csZ;

  // vec3 transmit = vec3(1., 1., 1.);
  // writePixel(color, transmit, csZ);
  float weight =
      max(min(1.0, max(max(color.r, color.g), color.b) * color.a), color.a) *
      clamp(0.03 / (1e-5 + pow(gl_FragCoord.z / 200, 4.0)), 1e-2, 3e3);
  _accum = vec4(color.rgb * color.a, color.a) * weight;
  _revealage = color.a;
}
