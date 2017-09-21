#version 140

out vec4 color;

uniform vec4 pointer;

void main() {
    color = mix(
        vec4(0.509804, 0.67451, 0.878431, 1.0),
        vec4(0.137255, 0.101961, 0.14902, 1.0),
        min(1.0, exp(distance(gl_FragCoord.xy, pointer.xy) / 100.0)
                * (pointer.z != 0.0 ? 1.0 : 0.5))
    );
}
