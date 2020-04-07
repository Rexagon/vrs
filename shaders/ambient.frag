#version 450

layout(input_attachment_index = 0, set = 0, binding = 0) uniform subpassInput u_diffuse;

layout(push_constant) uniform PushConstants {
    vec4 color;
} push_constants;

layout(location = 0) out vec4 f_color;

void main() {
    vec3 in_diffuse = subpassLoad(u_diffuse).rgb;

    in_diffuse *= push_constants.color.rgb;

    f_color = vec4(in_diffuse, 1.0);
}
