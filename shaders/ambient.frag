#version 450

layout(input_attachment_index = 0, set = 0, binding = 0) uniform subpassInput u_diffuse;
layout(input_attachment_index = 1, set = 0, binding = 1) uniform subpassInput u_normal;

layout(push_constant) uniform PushConstants {
    vec4 color;
} push_constants;

layout(location = 0) out vec4 f_light;

void main() {
    f_light = vec4(push_constants.color.rgb, 1.0);
}
