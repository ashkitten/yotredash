#version 140

out vec4 color;

uniform vec2 resolution;
uniform sampler2D animation;

void main() {
    vec2 uv = gl_FragCoord.xy / resolution;
    color = texture(animation, uv);
}
