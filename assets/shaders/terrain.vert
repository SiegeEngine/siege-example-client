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

layout(set = 1, binding = 0) uniform usamplerBuffer heightmap;

layout(location = 0) out vec4 world_position;
layout(location = 1) out vec3 normal;
layout(location = 2) out vec3 tangent;
layout(location = 3) out vec3 bitangent;

out gl_PerVertex {
    vec4 gl_Position;
};

const float yrange = 32; // height ranges 32 meters
const float xz_scale = 1.0; // each point is 1m from it's x/z neighbors
const int width = 513;
const int height = 513;

// This computes the vertex coordinates for triangle strip winding.
// It is the pixel coordinates within the heightmap for the height.
// It is the offset from the anchor in meters for the current vertex.
ivec2 strip_winding() {
  int index = gl_VertexIndex;
  int triangle_row = index / (width * 2 - 1);
  int triangle_offset = index % (width * 2 - 1);
  int t = triangle_row + (triangle_offset % 2); // +1 for every other index
  // the following is branchless code: we add both cases, but one of them
  // is guaranteed to be zero
  int row_mod2 = triangle_row % 2;
  int off_div2 = triangle_offset / 2;
  int s = (1 - row_mod2) * (              off_div2)
    +     (    row_mod2) * ((width - 1) - off_div2);
  return ivec2(s,t);
}

float get_y(ivec2 st) {
  uint i = texelFetch(heightmap, st.s + width * st.t).r;
  return yrange * (1.0 - i / 65536.0);
}

vec3 get_vertex_normal(ivec2 st, float y) {
  /*
  // Compute vertex normal
  vec3 center = vec3(0.0, y, 0.0);
  vec3 above = vec3(0.0, get_y(ivec2(st.s, clamp(st.t-1, 0, height-1))),  1.0);
  vec3 below = vec3(0.0, get_y(ivec2(st.s, clamp(st.t+1, 0, height-1))), -1.0);
  vec3 left = vec3(-1.0, get_y(ivec2(clamp(st.s-1, 0, width-1), st.t)), 0.0);
  vec3 right = vec3(1.0, get_y(ivec2(clamp(st.s+1, 0, width-1), st.t)), 0.0);
  //
  vec3 v_up = above - center;
  vec3 v_left = left - center;
  vec3 v_down = below - center;
  vec3 v_right = right - center;
  //
  vec3 n1 = normalize(cross(v_up, v_left));
  vec3 n2 = normalize(cross(v_left, v_down));
  vec3 n3 = normalize(cross(v_down, v_right));
  vec3 n4 = normalize(cross(v_right, v_up));
  //
  return normalize((n1 + n2 + n3 + n4)/4.0);
  */

  /* This was posted online as a way to compute normals from heightmaps.
     This is cheaper to compute.  Read as "z plus, z minus, ..." */
  float zp = get_y(ivec2(st.s, clamp(st.t-1, 0, height-1)));
  float zm = get_y(ivec2(st.s, clamp(st.t+1, 0, height-1)));
  float xp = get_y(ivec2(clamp(st.s+1, 0, width-1), st.t));
  float xm = get_y(ivec2(clamp(st.s-1, 0, width-1), st.t));
  vec3 t1 = vec3(2, xp - xm, 0);
  vec3 t2 = vec3(0, zp - zm, 2);
  return normalize(cross(t1, t2));
}

void main() {
  ivec2 st = strip_winding();
  float y = get_y(st);

  world_position = vec4( (st.s - (width/2.0)) * xz_scale,
                         y,
                         ((width/2.0) - st.t) * xz_scale,
                         1.0 );

  gl_Position =
    camera_uniforms.projection_x_view_matrix * world_position;

  normal = get_vertex_normal(st, y);
  tangent = vec3(1, 0, 0);
  bitangent = cross(tangent, normal);
  tangent = cross(normal, bitangent);
}
