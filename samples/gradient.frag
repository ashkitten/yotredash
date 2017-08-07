#version 130

out vec4 color;

uniform vec2 resolution;

void main() {
    vec2 position = gl_FragCoord.xy / resolution;
    color = mix(vec4(1.0, 0.0, 0.0, 1.0), vec4(0.0, 1.0, 0.0, 1.0), position.x);
}
