#version 450

layout(push_constant) uniform LightParameters {
    vec4 color;
} light_parameters;

layout(location = 0) out vec4 f_light;

void main() {
    f_light = vec4(light_parameters.color.rgb, 1.0);
}
