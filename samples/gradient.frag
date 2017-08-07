#version 130

uniform vec2 iResolution;

out vec4 fragColor;

void main() {
    vec2 position = gl_FragCoord.xy / iResolution;
    fragColor = mix(vec4(1.0, 0.0, 0.0, 1.0), vec4(0.0, 1.0, 0.0, 1.0), position.x);
}
