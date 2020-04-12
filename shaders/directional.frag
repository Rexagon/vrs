#version 450

layout(input_attachment_index = 0, set = 0, binding = 0) uniform subpassInput u_diffuse;
layout(input_attachment_index = 1, set = 0, binding = 1) uniform subpassInput u_normal;

layout(push_constant) uniform LightParameters {
    vec4 color;
    vec4 direction;
} light_parameters;

layout(location = 0) out vec4 f_light;

void main() {
    vec3 color = light_parameters.color.rgb;
    float intensity = light_parameters.color.a;

    vec3 normal = normalize((subpassLoad(u_normal).rgb  * 2.0 - 1.0));

    float light = clamp(dot(normal, -normalize(light_parameters.direction.xyz)), 0.0, 1.0);

    f_light = vec4(light * color * intensity, 1.0);
}
