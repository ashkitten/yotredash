#version 130

in vec2 texCoords;
out vec4 fragColor;

uniform sampler2D glyphTexture;
uniform vec3 color;

void main() {
    vec4 sampled = vec4(1.0, 1.0, 1.0, texture(glyphTexture, texCoords).r);
    fragColor = vec4(color, 1.0) * sampled;
}
