#version 450

#extension GL_ARB_separate_shader_objects : enable

// We dont need vertex buffer input.

// push constants can handle 128 bytes, but not necessarily more
// (we are safely at 4*13 = 52 bytes)
layout (push_constant) uniform PushConsts {
  float uv_x1; // unnormalized
  float uv_y1; // unnormalized
  float uv_width;
  float uv_height;
  float screen_pin_x1;
  float screen_pin_y1;
  float screen_pin_width;
  float screen_pin_height;
  float screen_area_x1;
  float screen_area_y1;
  float screen_area_width;
  float screen_area_height;
  float alpha;
} push;

out gl_PerVertex {
    vec4 gl_Position;
};

layout (location = 0) out flat float uv_x1;
layout (location = 1) out flat float uv_y1;
layout (location = 2) out flat float uv_width;
layout (location = 3) out flat float uv_height;
layout (location = 4) out flat float alpha;
layout (location = 5) out vec2 rel_uv;

void main() {

  vec2 screen = vec2(
    mix(
      push.screen_area_x1,
      push.screen_area_x1 + push.screen_area_width,
      (gl_VertexIndex & 0x02) >> 1),
    mix(
      push.screen_area_y1,
      push.screen_area_y1 + push.screen_area_height,
      gl_VertexIndex & 0x01)
  );

  // Relative UV coordinates (would be correct if image was the full texture)
  rel_uv = vec2(
    (screen.x - push.screen_pin_x1) / push.screen_pin_width,
    (screen.y - push.screen_pin_y1) / push.screen_pin_height
  );

  uv_x1 = push.uv_x1;
  uv_y1 = push.uv_y1;
  uv_width = push.uv_width;
  uv_height = push.uv_height;
  alpha = push.alpha;

  // depth=1.0 (back of the constraining viewport)
  gl_Position = vec4(
    2.0 * screen.x - 1.0,
    2.0 * screen.y - 1.0,
    1.0, 1.0);
}
