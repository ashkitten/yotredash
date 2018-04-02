#version 140

uniform vec2 resolution;
uniform sampler1D waveform;
uniform sampler2D spectrogram;

out vec4 color;

void main() {
    vec2 uv = gl_FragCoord.xy / resolution.xy;
    float wave = texture(waveform, uv.x).x;

    color = texture(spectrogram, uv);
    color += 1.0 - smoothstep(0.0, 0.01, abs(wave - uv.y));
}
