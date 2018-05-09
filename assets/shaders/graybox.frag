#version 450

layout(early_fragment_tests) in;

layout(location = 0) in vec4 frag_normal_wspace_needs_renormalization;

layout(set = 0, binding = 0) uniform CameraUniforms {
  mat4 projection_x_view_matrix;
  mat4 view_matrix;
  mat4 projection_matrix;
  vec4 camera_position_wspace;
  float ambient;
  float white_level;
  int width;
  int height;
} camera_uniforms;

layout(set = 1, binding = 0) uniform GrayboxUBO {
  mat4 model_matrix;
  vec4 diffuse; // 4th component ignored
  vec4 material; // roughness, metallicity, ao, cavity
} graybox_uniforms;

layout(location = 0) out vec4 diffuse;
layout(location = 1) out vec4 normals;
layout(location = 2) out vec4 material;

vec4 encode_normal(vec4 n) {
  return vec4((n.xyz / 2.0) + 0.5, 0.0);
}

void main() {
  diffuse = graybox_uniforms.diffuse;
  material = graybox_uniforms.material;

  // Renormalize the interpolated fragment normal
  vec4 frag_normal_wspace = normalize(frag_normal_wspace_needs_renormalization);
  // must be in view space and encoded
  normals = encode_normal(camera_uniforms.view_matrix * frag_normal_wspace);
}
