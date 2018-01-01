#version 140

out vec4 color;

uniform vec2 resolution;
uniform sampler2D tex;

void main() {
    vec2 size = textureSize(tex, 0);
    vec2 uv = gl_FragCoord.xy / resolution;
    uv.x /= size.x / size.y;
    uv.x *= resolution.x / resolution.y;
    uv.x -= ((resolution.x * size.y) / (resolution.y * size.x) - 1.0) / 2;
    color = (uv.x < 0 || uv.x > 1 || uv.y < 0 || uv.y > 1) ? vec4(0.0, 0.0, 0.0, 1.0) : texture(tex, uv);
}
