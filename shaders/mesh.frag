#version 450

layout(location = 0) in vec3 in_normal;

layout(location = 0) out vec4 out_color;

void main() {
    out_color = vec4((in_normal + 1.0) / 2.0, 1.0);
}
