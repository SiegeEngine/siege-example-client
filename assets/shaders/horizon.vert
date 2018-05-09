#version 450

#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable

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

layout(location = 0) out vec4 world_position;

// 32 is the lowest y point that the terrain (and camera) can go.
// We put this horizon underneath so you cant get beneath it,
// and far enough to avoid any z-fighting
vec4 vertices[4] = vec4[](
  vec4( -10000.0,  40,  10000.0, 1.0),
  vec4( -10000.0,  40, -10000.0, 1.0),
  vec4(  10000.0,  40,  10000.0, 1.0),
  vec4(  10000.0,  40, -10000.0, 1.0)
);

out gl_PerVertex {
    vec4 gl_Position;
};

void main() {
  // Translate horizon along with camera, so you can never get to the edge
  // no matter how far you travel:
  world_position =
    mat4(vec4(1.0, 0.0, 0.0, 0.0),
      vec4(0.0, 1.0, 0.0, 0.0),
      vec4(0.0, 0.0, 1.0, 0.0),
      vec4(
        camera_uniforms.camera_position_wspace.x,
        0.0,
        camera_uniforms.camera_position_wspace.z,
        1.0))
    * vertices[gl_VertexIndex];

  gl_Position = camera_uniforms.projection_x_view_matrix * world_position;
}
