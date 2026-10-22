#version 300 es
// Separable Gaussian, 13 taps. Same kernel as mirror_core::pipeline::gaussian_kernel.
precision highp float;
uniform sampler2D u_in;
uniform vec2 u_target;
uniform vec2 u_dir;
uniform float u_w[13];
out vec4 o;
void main() {
    vec2 uv = gl_FragCoord.xy / u_target;
    float acc = 0.0;
    for (int i = 0; i < 13; i++) {
        acc += textureLod(u_in, uv + float(i - 6) * u_dir, 0.0).r * u_w[i];
    }
    o = vec4(vec3(acc), 1.0);
}
