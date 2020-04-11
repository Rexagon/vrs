#version 450

layout(input_attachment_index = 0, set = 0, binding = 0) uniform subpassInput u_diffuse;
layout(input_attachment_index = 1, set = 0, binding = 1) uniform subpassInput u_normal;

layout(push_constant) uniform PushConstants {
    vec4 color;
    vec4 direction;
} push_constants;

layout(location = 0) out vec4 f_light;

void main() {
    vec3 normal = normalize(subpassLoad(u_normal).rgb);

    float light = clamp(dot(normal, -normalize(push_constants.direction.xyz)), 0.0, 1.0);

    f_light = vec4(light * push_constants.color.rgb, 1.0);
}
