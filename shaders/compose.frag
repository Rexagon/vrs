#version 450

layout(input_attachment_index = 0, set = 0, binding = 0) uniform subpassInput u_diffuse;
layout(input_attachment_index = 1, set = 0, binding = 1) uniform subpassInput u_light;
layout(input_attachment_index = 2, set = 0, binding = 2) uniform subpassInput u_depth; // TODO: use for discarding fragments

layout(location = 0) out vec4 f_color;

void main() {
    vec3 diffuse = subpassLoad(u_diffuse).rgb;
    vec3 light = subpassLoad(u_light).rgb;

    f_color = vec4(diffuse * light, 1.0);
}
