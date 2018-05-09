#version 450

layout(early_fragment_tests) in;

layout(constant_id = 0) const int surface_needs_gamma = 0;

layout(location = 0) in flat vec4 color;

layout(location = 0) out vec4 outColor;

float srgb_ungamma(float srgb) {
  if (srgb < 0.04045) {
    return srgb / 12.92;
  } else {
    return pow((srgb + 0.055)/(1 + 0.055), 2.4);
  }
}

void main() {
  if (surface_needs_gamma == 0) {
    outColor = vec4(
      srgb_ungamma(color.r),
      srgb_ungamma(color.g),
      srgb_ungamma(color.b),
      color.a); // sRGB does not apply to alpha channel.
  } else {
    outColor = color;
  }
}
