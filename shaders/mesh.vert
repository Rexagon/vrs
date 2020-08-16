#version 450

layout(location = 0) in vec3 in_position;
layout(location = 1) in vec3 in_normal;

layout(set = 0, binding = 0) uniform WorldData {
    mat4 u_view;
    mat4 u_projection;
};

layout(location = 0) out vec3 out_normal;

void main() {
    gl_Position = u_projection * u_view * vec4(in_position, 1.0);
    out_normal = in_normal;
}
