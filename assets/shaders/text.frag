#version 450

layout(early_fragment_tests) in;

layout(set = 0, binding = 0) uniform sampler2D atlas_main;

layout(constant_id = 0) const int surface_needs_gamma = 0;
layout(location = 0) in vec2 uv;
layout(location = 1) in flat uint props;

layout(location = 0) out vec4 outColor;

vec3 colors[8] = vec3[](
  vec3(0.0, 0.0, 0.0), // BLACK
  vec3(1.0, 1.0, 1.0), // WHITE
  vec3(1.0, 0.0, 0.0), // RED
  vec3(0.5, 0.5, 0.5), // GRAY
  vec3(1.0, 0.83984375, 0.0), // GOLD #FFD700
  vec3(0.2, 1.0, 0.2), // GREEN
  vec3(0.0, 0.0, 1.0), // BLUE
  vec3(0.5, 0.5, 0.8984375) // LAVENDAR #8080e6
);

float srgb_ungamma(float srgb) {
  if (srgb < 0.04045) {
    return srgb / 12.92;
  } else {
    return pow((srgb + 0.055)/(1 + 0.055), 2.4);
  }
}

void main() {
  // decode props:
  // margin is the outside thickness (from edge 127 to 0) in pixels
  float margin = ((props & 0xFF000000) >> 24) / 10.0;
  float alpha = (props & 0x000000FF) / 255.0;
  uint font = (props & 0x00000300) >> 8;
  vec3 color3 = colors[(props & 0x00001C00) >> 10];
  vec3 outline_color3 = colors[(props & 0x0000E000) >> 13];
  bool outline = (props & 0x00010000) != 0;

  // retrieve distance from texture
  // note: Conditional sampling hurts performance more than multiple sampling.
  //       so we are better off sampling all the textures
  float mask = texture( atlas_main, uv )[0];

  float px = 0.5 / margin; // distance value range across 1 pixel
  float cutoff_high = min(0.99, 0.5 + px/2.0); // one-half on inside
  float cutoff_low = max(0.01, 0.5 - px/2.0); // one-half on outside

  // if text is small, avoid subpixel thinning of letters:
  if (margin < 1.4) {
    cutoff_high = min(0.99, 0.5 + px/3.0); // one-third on inside
    cutoff_low = max(0.01, 0.5 - 2.0*px/3.0); // two-thirds on outside
  }

  vec4 color = vec4(0.0, 0.0, 0.0, 0.0); // in case conditionals fall through
  if (outline) {
    float cutoff_outline = max(0.01, cutoff_low - px * 3.0); // 3 pixel border
    if (mask >= cutoff_high) {
      color = vec4(color3, 1.0); // full alpha
    }
    else if (mask >= cutoff_low) {
      color = vec4(
        mix(outline_color3, color3,
          smoothstep(cutoff_low, cutoff_high, mask)), 1.0); // color mix ramp
    }
    else if (mask > cutoff_outline) {
      color = vec4(outline_color3, smoothstep(cutoff_outline, cutoff_low, mask)); // alpha ramp
    }
  } else {
    if (mask >= cutoff_high) {
      color = vec4(color3, 1.0); // full alpha
    }
    else if (mask > cutoff_low) {
      color = vec4(color3, smoothstep(cutoff_low, cutoff_high, mask)); // alpha ramp
    }
  }

  // Apply requested alpha
  color.a *= alpha;

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
