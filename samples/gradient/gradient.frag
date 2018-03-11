#version 140

out vec4 color;

uniform vec2 resolution;

void main() {
    vec2 position = gl_FragCoord.xy / resolution;
    color = mix(
        vec4(0.137255, 0.101961, 0.14902, 1.0),
        vec4(0.419608, 0.301961, 0.32549, 1.0),
        position.x
    );
}
