#version 140

out vec4 color;

uniform vec2 resolution;
uniform sampler2D tex;

void main() {
    vec2 uv = gl_FragCoord.xy / resolution;
    color = texture(tex, uv);
}
