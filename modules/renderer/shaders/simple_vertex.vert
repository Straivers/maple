#version 450

layout(location = 0) in vec2 inPosition;
layout(location = 1) in vec4 inColor;

layout(location = 0) out vec4 fragColor;

layout (push_constant) uniform PushConstants
{
    vec2 scale;
} constants;

void main() {
    gl_Position = vec4(inPosition * constants.scale + vec2(-1.0, -1.0), 0.0, 1.0);
    fragColor = inColor;
}
