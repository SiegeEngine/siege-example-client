#version 450

#extension GL_ARB_separate_shader_objects : enable

layout(location = 0) in uint encoded_position;
layout(location = 1) in uint encoded_uv;
layout(location = 2) in uint in_props;

layout (push_constant) uniform PushConsts {
  float depth;
} push;

out gl_PerVertex {
    vec4 gl_Position;
};
layout (location = 0) out vec2 uv;
layout (location = 1) out uint props;

vec2 decode_screen(uint encoded) {
  // unpack
  float x = (encoded & 0xFFFF0000) >> 16;
  float y = encoded & 0x0000FFFF;

  // decode
  x = (x - 10000) / 45535.0;
  y = (y - 10000) / 45535.0;

  // modify range
  return vec2(
    x * 2.0 - 1.0,
    y * 2.0 - 1.0
  );
}

vec2 decode_uv(uint encoded) {
  // unpack
  float x = (encoded & 0xFFFF0000) >> 16;
  float y = encoded & 0x0000FFFF;

  // decode
  x = (x - 10000) / 10.0;
  y = (y - 10000) / 10.0;

  return vec2(x, y);
}

void main() {
  vec2 position = decode_screen(encoded_position);
  vec2 in_uv = decode_uv(encoded_uv);

  gl_Position = vec4(position, push.depth, 1.0);
  uv = in_uv;
  props = in_props;
}
