#version 450

#extension GL_ARB_separate_shader_objects : enable

layout(push_constant) uniform PushConsts {
  vec4 color;
} push;

layout(location = 0) out flat vec4 color;

out gl_PerVertex {
    vec4 gl_Position;
};

void main() {
  vec2 position;

  switch(gl_VertexIndex) {
    case 0: position=vec2(-1.0,-1.0); break;
    case 1: position=vec2(-1.0, 1.0); break;
    case 2: position=vec2( 1.0, 1.0); break;
    case 3: position=vec2(-1.0,-1.0); break;
    case 4: position=vec2( 1.0, 1.0); break;
    case 5: position=vec2( 1.0,-1.0); break;
    default: position=vec2(0.0, 0.0); break;
  }

  gl_Position = vec4(position, 1.0, 1.0);
  color = push.color;
}
