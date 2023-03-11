#version 450

layout(location = 0) in vec3 oColor;

layout(location = 0) out vec4 finalColor;

void main() {
    finalColor = vec4(oColor, 1.0);
}
