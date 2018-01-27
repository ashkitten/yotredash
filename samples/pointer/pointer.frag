#version 140

out vec4 color;

uniform vec4 info_pointer;

void main() {
    color = mix(
        vec4(1.0),
        vec4(0.0),
        min(1.0, exp(distance(gl_FragCoord.xy, info_pointer.xy) / 100.0) * (info_pointer.z != 0.0 ? 1.0 : 0.4))
    );
}
