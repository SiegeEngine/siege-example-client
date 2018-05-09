#version 450

layout(location = 0) in vec3 vertex_position_mspace;
layout(location = 1) in vec3 vertex_normal_mspace;

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
  vec4 material; // roughness, metallicity, ao
} graybox_uniforms;

out gl_PerVertex {
    vec4 gl_Position;
};

layout(location = 0) out vec4 vertex_normal_wspace;

void main() {
  gl_Position = camera_uniforms.projection_x_view_matrix
    * graybox_uniforms.model_matrix * vec4(vertex_position_mspace, 1.0);

  vertex_normal_wspace = graybox_uniforms.model_matrix * vec4(vertex_normal_mspace, 0.0);
}
