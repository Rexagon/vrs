#version 450

layout(location = 0) out vec4 f_color;
layout(location = 1) out vec4 f_normal;

void main() {
    f_color = vec4(1.0, 0.0, 0.0, 1.0);
    f_normal = vec4(1.0, 0.0, 1.0, 1.0);
}
