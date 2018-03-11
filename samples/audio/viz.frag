#version 140

uniform float info_time;
uniform vec2 info_resolution;
uniform sampler1D audio_spectrum;

out vec4 color;

void main() {
    vec2 uv = gl_FragCoord.xy / info_resolution.xy;
    color = vec4(texture(audio_spectrum, uv.x).x, 0.0, 0.0, 1.0);
}
