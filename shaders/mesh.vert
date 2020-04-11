#version 450

layout(location = 0) in vec3 position;
layout(location = 1) in vec3 normal;

layout(location = 0) out vec3 v_normal;

layout(set = 0, binding = 0) uniform WorldData {
    mat4 view;
    mat4 projection;
} world_data;

layout(push_constant) uniform MeshData {
    mat4 transform;
} mesh_data;

void main() {
    mat4 view_transform = world_data.view * mesh_data.transform;
    v_normal = transpose(inverse(mat3(view_transform))) * normal;
    gl_Position = world_data.projection * view_transform * vec4(position, 1.0);
}
