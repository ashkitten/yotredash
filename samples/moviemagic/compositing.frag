#version 140

// from ferris's Movie Magic (https://www.shadertoy.com/view/4dBcDK)

// Final composite/post pass

// Dedicated to the public domain under CC0 1.0 Universal
//  https://creativecommons.org/publicdomain/zero/1.0/legalcode

uniform float time;
uniform vec2 resolution;
uniform sampler2D vbloom;
uniform sampler2D render;

out vec4 fragColor;

vec3 hableTonemap(vec3 x)
{
    float a = .15;
    float b = .5;
    float c = .1;
    float d = .2;
    float e = .02;
    float f = .3;

    return ((x * (x * a + c * b) + d * e) / (x * (x * a + b) + d * f)) - e / f;
}

vec3 tonemap(vec3 rawColor, float exposure)
{
    float w = 11.2;

    vec3 exposedColor = max(rawColor * exposure, 0.0);

    vec3 linear = pow(exposedColor, vec3(1.0 / 2.2));

    vec3 reinhard = pow(exposedColor / (exposedColor + 1.0), vec3(1.0 / 2.2));

    vec3 x = max(exposedColor - .004, 0.0);
    vec3 hejlBurgessDawson = (x * (x * 6.2 + .5)) / (x * (x * 6.2 + 1.7) + .06);

    float exposureBias = 2.0;
    vec3 curr = hableTonemap(exposedColor * exposureBias);
    float whiteScale = (vec3(1.0) / hableTonemap(vec3(w))).x;
    vec3 color = curr * whiteScale;
    vec3 hable = pow(color, vec3(1.0 / 2.2));

    return
        //linear
        //reinhard
        hejlBurgessDawson
        //hable
        ;
}

void main() {
    vec2 uv = gl_FragCoord.xy / resolution.xy;
    
    float bloomAmount = 0.07;
    vec3 original = texture(render, uv).xyz;
    vec3 bloom = texture(vbloom, uv).xyz;
    vec3 inputColor = original + bloom * bloomAmount;
    
    // Vignette
    float vignetteStrength = 0.9;
    float vignetteSizeBias = 0.5;
    float vignettePower = 1.0;
    float d = clamp(length(uv * 2.0 - 1.0) - vignetteSizeBias, 0.0, 1.0);
  inputColor *= 1.0 - clamp(pow(d, vignettePower) * vignetteStrength, 0.0, 1.0);
    
    // Grain
    float grainAmount = 1.0;
    float grainStrength = 50.0 * grainAmount;

    float x = (uv.x + 4.0) * (uv.y + 4.0) * (time + 10.0) * 10.0;
    float grain = clamp(mod((mod(x, 13.0) + 1.0) * (mod(x, 123.0) + 1.0), 0.01) - 0.005, 0.0, 1.0) * grainStrength;
    
    inputColor *= 1.0 - grain;
    
    // Bring up lows a bit
    inputColor = inputColor * .995 + .005;

    // Some toy grading :)
  inputColor = pow(inputColor, vec3(1.175, 1.05, 1.0));
    
    // Tonemap
    float exposure = 0.0;
    vec3 tonemappedColor = clamp(tonemap(inputColor, pow(2.0, exposure)), 0.0, 1.0);
    
    // Output luma for fxaa
    float luma = sqrt(dot(tonemappedColor, vec3(0.299, 0.587, 0.114)));
    
    fragColor = vec4(tonemappedColor, luma);
}
