#version 140

out vec4 color;

uniform vec2 resolution;
uniform sampler2D texture0;

void main() {
    vec2 uv = gl_FragCoord.xy / resolution;
    color = texture(texture0, uv);
}
