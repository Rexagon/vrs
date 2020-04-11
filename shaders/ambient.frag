#version 450

layout(push_constant) uniform LightParameters {
    vec4 color;
} light_parameters;

layout(location = 0) out vec4 f_light;

void main() {
    vec3 color = light_parameters.color.rgb;
    float intensity = light_parameters.color.a;

    f_light = vec4(color * intensity, 1.0);
}
