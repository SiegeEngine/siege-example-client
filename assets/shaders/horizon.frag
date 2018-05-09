#version 450

layout(early_fragment_tests) in;

layout(set = 0, binding = 0) uniform CameraUBO {
  mat4 projection_x_view_matrix;
  mat4 view_matrix;
  mat4 projection_matrix;
  vec4 camera_position_wspace;
  float ambient;
  float white_level;
  int width;
  int height;
} camera_uniforms;

layout(location = 0) in vec4 in_position;

layout(location = 0) out vec4 diffuse;
layout(location = 1) out vec4 normals;
layout(location = 2) out vec4 material;

vec4 encode_normal(vec4 n) {
  return vec4((n.xyz / 2.0) + 0.5, 0.0);
}

void main() {
  // Deep ocean blue
  // diffuse = vec4(0.006, 0.099, 0.200, 1.0); // srgb value
  diffuse = vec4(0.0004643, 0.010726, 0.036636, 1.0);

  // normal is always up for this flat horizon
  // must be encoded and in view space
  normals = encode_normal(camera_uniforms.view_matrix * vec4(0.0, -1.0, 0.0, 0.0));

  // roughness
  material.x = 0.2; // a bit, to simulate waves perhaps
  // metallicity
  material.y = 0.5; // reflects, but not really metallic... not sure.
  // ambient occlusion
  material.z = 1.0; // uniform ambient occlusion (no occlusion)
  // cavity
  material.a = 1.0;
}
