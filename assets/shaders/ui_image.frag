#version 450

layout(early_fragment_tests) in;

layout(constant_id = 0) const int surface_needs_gamma = 0;
layout(set = 0, binding = 0) uniform sampler2D ui_atlas;

layout(location = 0) in flat float uv_x1;
layout(location = 1) in flat float uv_y1;
layout(location = 2) in flat float uv_width;
layout(location = 3) in flat float uv_height;
layout(location = 4) in flat float alpha;
layout(location = 5) in vec2 rel_uv;

layout(location = 0) out vec4 outColor;

float srgb_ungamma(float srgb) {
  if (srgb < 0.04045) {
    return srgb / 12.92;
  } else {
    return pow((srgb + 0.055)/(1 + 0.055), 2.4);
  }
}

void main() {

  vec2 uv = vec2(
    uv_x1 + fract(rel_uv.x) * uv_width,
    uv_y1 + fract(rel_uv.y) * uv_height
  );

  vec4 color = texture(ui_atlas, uv);
  color.a = color.a * alpha;

  if (surface_needs_gamma == 0) {
    // The surface does not need us to apply the gamma curve
    // but our data is already in sRGB.  We will have to un-gamma it.
    outColor = vec4(
      srgb_ungamma(color.r),
      srgb_ungamma(color.g),
      srgb_ungamma(color.b),
      color.a); // sRGB does not apply to alpha channel.
  } else {
    outColor = color;
  }
}
