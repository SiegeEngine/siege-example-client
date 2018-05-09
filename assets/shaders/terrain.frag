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

layout(set = 1, binding = 1) uniform sampler2D albedomap;
layout(set = 1, binding = 2) uniform sampler2D normalmap;
layout(set = 1, binding = 3) uniform sampler2D aomap;
layout(set = 1, binding = 4) uniform sampler2D roughnessmap;
layout(set = 1, binding = 5) uniform sampler2D cavitymap;

layout(location = 0) in vec4 in_position;
layout(location = 1) in vec3 normal;
layout(location = 2) in vec3 tangent;
layout(location = 3) in vec3 bitangent;

layout(location = 0) out vec4 diffuse;
layout(location = 1) out vec4 normals;
layout(location = 2) out vec4 material;

vec4 triplanar(vec3 world_position, sampler2D map, float tilesize, vec3 normal) {
  return texture(map, fract(world_position.zy / tilesize)) * normal.x * normal.x +
    texture(map, fract(world_position.zx / tilesize)) * normal.y * normal.y +
    texture(map, fract(world_position.xy / tilesize)) * normal.z * normal.z;
}

vec3 decode_normal(vec3 n) {
  return (n - 0.5) * 2;
}
vec4 encode_normal(vec4 n) {
  return vec4((n.xyz / 2.0) + 0.5, 0.0);
}

void main() {
  float tilesize = 6.0;

  diffuse = triplanar(in_position.xyz, albedomap, tilesize, normal);

  vec3 pointnormal = decode_normal(triplanar(in_position.xyz, normalmap, tilesize, normal).xyz);
  // must be encoded and in view space
  normals = encode_normal(camera_uniforms.view_matrix *
    vec4(mat3(tangent, bitangent, normal) * pointnormal, 0.0));

  // roughness
  material.x = triplanar(in_position.xyz, roughnessmap, tilesize, normal).r;
  // metallicity
  material.y = 0.04;
  // ambient occlusion
  material.z = triplanar(in_position.xyz, aomap, tilesize, normal).r;
  // cavity
  material.a = triplanar(in_position.xyz, cavitymap, tilesize, normal).r;
}
