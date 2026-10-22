#version 300 es
// Luminance of the source, area-averaged down to the blur map size.
precision highp float;
uniform sampler2D u_src;
uniform vec2 u_target;
uniform vec2 u_footprint; // size of one target pixel in source uv
uniform float u_lod;
out vec4 o;
const vec3 LUMA = vec3(0.2126, 0.7152, 0.0722);
void main() {
    vec2 uv = gl_FragCoord.xy / u_target;
    float acc = 0.0;
    for (int y = 0; y < 4; y++) {
        for (int x = 0; x < 4; x++) {
            vec2 off = (vec2(float(x), float(y)) + 0.5) / 4.0 - 0.5;
            acc += dot(textureLod(u_src, uv + off * u_footprint, u_lod).rgb, LUMA);
        }
    }
    o = vec4(vec3(acc / 16.0), 1.0);
}
