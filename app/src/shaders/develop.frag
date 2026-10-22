#version 300 es
// MirrorLab develop shader.
//
// A line-by-line port of `mirror_core::pipeline::shade` plus the geometry
// mapping in `mirror_core::geometry`. Keep the two in sync: the Rust tests
// pin the maths, and the app's self-test compares both on the same pixels.
precision highp float;
precision highp int;
precision highp sampler2D;

uniform sampler2D u_src;
uniform sampler2D u_blur;
uniform sampler2D u_lut;

uniform vec2 u_target;      // render target size (px)
uniform vec2 u_offset;      // top-left of this target inside the logical canvas
uniform vec4 u_imgRect;     // image placement in the logical canvas (x, y, w, h)
uniform vec2 u_srcSize;     // texture size (px)
uniform vec2 u_srcFull;     // original size before any downscale (px)

uniform int u_turns;
uniform vec2 u_flip;
uniform float u_straighten; // radians
uniform vec4 u_crop;        // x, y, w, h in view space
uniform float u_split;      // show the original left of this output x; -1 = off
uniform int u_bypass;

uniform vec3 u_gain;
uniform vec4 u_tone;        // highlights, shadows, whites, blacks
uniform float u_contrast;
uniform vec3 u_local;       // clarity, dehaze, sharpen
uniform vec2 u_color;       // saturation, vibrance
uniform vec3 u_hsl[8];
uniform int u_hslOn;
uniform vec4 u_grade[3];
uniform vec2 u_gradeOn;     // enabled, balance
uniform vec2 u_vig;         // amount, feather
uniform vec2 u_grain;       // amount, cell size (source px)
uniform int u_curveOn;
uniform float u_lutSize;

out vec4 o;

const vec3 LUMA = vec3(0.2126, 0.7152, 0.0722);
const float GAMMA = 2.2;
const float CENTERS[9] = float[9](0.0, 30.0, 60.0, 115.0, 175.0, 225.0, 270.0, 315.0, 360.0);

float luma(vec3 c) { return dot(c, LUMA); }

vec2 frameSize() { return (u_turns % 2 == 1) ? u_srcSize.yx : u_srcSize; }

vec2 frameToSource(vec2 f) {
    f = mix(f, 1.0 - f, u_flip);
    if (u_turns == 1) return vec2(f.y, 1.0 - f.x);
    if (u_turns == 2) return 1.0 - f;
    if (u_turns == 3) return vec2(1.0 - f.y, f.x);
    return f;
}

vec2 viewToFrame(vec2 p) {
    if (u_straighten == 0.0) return p;
    vec2 fs = frameSize();
    vec2 d = (p - 0.5) * fs;
    float s = sin(u_straighten);
    float c = cos(u_straighten);
    vec2 r = vec2(d.x * c + d.y * s, -d.x * s + d.y * c);
    return r / fs + 0.5;
}

float hash2(uvec2 p) {
    uint h = (p.x * 0x8da6b343u) ^ (p.y * 0xd8163841u);
    h ^= h >> 16;
    h *= 0x7feb352du;
    h ^= h >> 15;
    h *= 0x846ca68bu;
    h ^= h >> 16;
    return float(h >> 8) / 16777216.0;
}

float valueNoise(vec2 p) {
    vec2 i = floor(p);
    vec2 f = p - i;
    uvec2 c = uvec2(i);
    float a = hash2(c);
    float b = hash2(c + uvec2(1u, 0u));
    float d = hash2(c + uvec2(0u, 1u));
    float e = hash2(c + uvec2(1u, 1u));
    vec2 u = f * f * (3.0 - 2.0 * f);
    return mix(mix(a, b, u.x), mix(d, e, u.x), u.y);
}

float sCurve(float x, float k) {
    return x < 0.5 ? 0.5 * pow(2.0 * x, k) : 1.0 - 0.5 * pow(2.0 - 2.0 * x, k);
}

vec3 rgb2hsv(vec3 c) {
    float mx = max(c.r, max(c.g, c.b));
    float mn = min(c.r, min(c.g, c.b));
    float d = mx - mn;
    float h = 0.0;
    if (d > 1e-6) {
        if (mx == c.r) h = 60.0 * mod((c.g - c.b) / d, 6.0);
        else if (mx == c.g) h = 60.0 * ((c.b - c.r) / d + 2.0);
        else h = 60.0 * ((c.r - c.g) / d + 4.0);
    }
    float s = mx <= 1e-6 ? 0.0 : d / mx;
    return vec3(h, s, mx);
}

vec3 hsv2rgb(float h, float s, float v) {
    h = mod(h, 360.0) / 60.0;
    float c = v * s;
    float x = c * (1.0 - abs(mod(h, 2.0) - 1.0));
    int i = int(h);
    vec3 rgb;
    if (i == 0) rgb = vec3(c, x, 0.0);
    else if (i == 1) rgb = vec3(x, c, 0.0);
    else if (i == 2) rgb = vec3(0.0, c, x);
    else if (i == 3) rgb = vec3(0.0, x, c);
    else if (i == 4) rgb = vec3(x, 0.0, c);
    else rgb = vec3(c, 0.0, x);
    return rgb + (v - c);
}

float lut(float x, float ch) {
    float u = (clamp(x, 0.0, 1.0) * (u_lutSize - 1.0) + 0.5) / u_lutSize;
    vec4 v = textureLod(u_lut, vec2(u, 0.5), 0.0);
    return ch < 0.5 ? v.r : (ch < 1.5 ? v.g : v.b);
}

vec3 shade(vec3 src, float blurL, float detail, float noise, vec2 uv) {
    // 1. White balance and exposure in linear light.
    vec3 c = pow(pow(max(src, 0.0), vec3(GAMMA)) * u_gain, vec3(1.0 / GAMMA));

    // 2. Dehaze.
    if (u_local.y != 0.0) {
        float a = u_local.y * 0.4 * clamp(blurL, 0.0, 1.0);
        c = (c - a) / max(1.0 - a, 0.05);
    }

    // 3. Clarity and sharpening.
    float srcL = luma(src);
    if (u_local.x != 0.0) {
        float l = clamp(luma(c), 0.0, 1.0);
        float mid = 1.0 - (2.0 * l - 1.0) * (2.0 * l - 1.0);
        c += (srcL - blurL) * u_local.x * 1.2 * mid;
    }
    if (u_local.z > 0.0) {
        c += detail * u_local.z * 2.0;
    }

    // 4. Tone.
    float l = luma(c);
    float lc = max(l, 0.0);
    float dl = u_tone.x * 0.32 * smoothstep(0.45, 1.0, lc)
             + u_tone.y * 0.32 * (1.0 - smoothstep(0.0, 0.55, lc))
             + u_tone.z * 0.25 * smoothstep(0.7, 1.1, lc)
             + u_tone.w * 0.18 * (1.0 - smoothstep(0.0, 0.3, lc));
    if (dl != 0.0) {
        float l2 = max(l + dl, 0.0);
        float ratio = l2 / max(l, 1e-4);
        float blend = smoothstep(0.02, 0.2, l);
        c = mix(c + (l2 - l), c * ratio, blend);
    }
    c = clamp(c, 0.0, 1.0);

    // 5. Contrast.
    if (u_contrast != 0.0) {
        float k = exp(u_contrast * 0.85);
        c = vec3(sCurve(c.r, k), sCurve(c.g, k), sCurve(c.b, k));
    }

    // 6. Tone curve.
    if (u_curveOn == 1) {
        c = vec3(lut(c.r, 0.0), lut(c.g, 1.0), lut(c.b, 2.0));
    }

    // 7. Saturation and vibrance.
    float sat = u_color.x + u_local.y * 0.2;
    if (sat != 0.0) {
        float lm = luma(c);
        c = lm + (c - lm) * (1.0 + sat);
    }
    if (u_color.y != 0.0) {
        float lm = luma(c);
        float mx = max(c.r, max(c.g, c.b));
        float mn = min(c.r, min(c.g, c.b));
        float s = clamp(mx - mn, 0.0, 1.0);
        c = lm + (c - lm) * (1.0 + u_color.y * (1.0 - s));
    }
    c = clamp(c, 0.0, 1.0);

    // 8. HSL mixer.
    if (u_hslOn == 1) {
        vec3 hsv = rgb2hsv(c);
        float chroma = max(c.r, max(c.g, c.b)) - min(c.r, min(c.g, c.b));
        float cw = smoothstep(0.0, 0.15, chroma);
        float h = mod(hsv.x, 360.0);
        int a = 0;
        int b = 1;
        float t = 0.0;
        for (int i = 0; i < 8; i++) {
            if (h >= CENTERS[i] && h < CENTERS[i + 1]) {
                a = i;
                b = (i + 1) % 8;
                t = smoothstep(0.0, 1.0, (h - CENTERS[i]) / (CENTERS[i + 1] - CENTERS[i]));
                break;
            }
        }
        vec3 d = mix(u_hsl[a], u_hsl[b], t) * cw;
        c = hsv2rgb(hsv.x + d.x, hsv.y, hsv.z);
        float lm = luma(c);
        c = (lm + (c - lm) * (1.0 + d.y)) * (1.0 + d.z * 0.45);
        c = clamp(c, 0.0, 1.0);
    }

    // 9. Colour grading.
    if (u_gradeOn.x > 0.5) {
        float lm = luma(c);
        float pivot = 0.5 + u_gradeOn.y * 0.25;
        float ws = 1.0 - smoothstep(0.0, pivot, lm);
        float wh = smoothstep(pivot, 1.0, lm);
        float wm = max(1.0 - ws - wh, 0.0);
        c += (u_grade[0].rgb + u_grade[0].a) * ws;
        c += (u_grade[1].rgb + u_grade[1].a) * wm;
        c += (u_grade[2].rgb + u_grade[2].a) * wh;
        c = clamp(c, 0.0, 1.0);
    }

    // 10. Vignette.
    if (u_vig.x != 0.0) {
        vec2 dv = (uv - 0.5) * 2.0;
        float d = length(dv) * 0.70710678;
        float inner = 0.5 * (1.0 - u_vig.y) + 0.1;
        float w = smoothstep(inner, 1.05, d);
        if (u_vig.x < 0.0) c *= 1.0 + u_vig.x * w;
        else c += (1.0 - c) * u_vig.x * w;
    }

    // 11. Grain.
    if (u_grain.x > 0.0) {
        float lm = luma(c);
        float amp = u_grain.x * 0.2 * (1.0 - 0.5 * abs(2.0 * lm - 1.0));
        c += (noise - 0.5) * 2.0 * amp;
    }
    return clamp(c, 0.0, 1.0);
}

void main() {
    vec2 p = vec2(gl_FragCoord.x, u_target.y - gl_FragCoord.y) + u_offset;
    vec2 img = (p - u_imgRect.xy) / u_imgRect.zw;
    vec2 view = u_crop.xy + img * u_crop.zw;
    vec2 fr = viewToFrame(view);
    vec2 suv = frameToSource(fr);
    // Gradients are taken before any branching so mip selection stays defined.
    vec2 gx = dFdx(suv);
    vec2 gy = dFdy(suv);

    if (any(lessThan(img, vec2(0.0))) || any(greaterThan(img, vec2(1.0))) ||
        any(lessThan(fr, vec2(0.0))) || any(greaterThan(fr, vec2(1.0)))) {
        o = vec4(0.0);
        return;
    }

    vec3 src = textureGrad(u_src, suv, gx, gy).rgb;
    if (u_bypass == 1 || img.x < u_split) {
        o = vec4(clamp(src, 0.0, 1.0), 1.0);
        return;
    }

    float blurL = textureLod(u_blur, suv, 0.0).r;
    float detail = 0.0;
    if (u_local.z > 0.0) {
        vec2 t = 1.0 / u_srcSize;
        float n = luma(textureGrad(u_src, suv - vec2(t.x, 0.0), gx, gy).rgb)
                + luma(textureGrad(u_src, suv + vec2(t.x, 0.0), gx, gy).rgb)
                + luma(textureGrad(u_src, suv - vec2(0.0, t.y), gx, gy).rgb)
                + luma(textureGrad(u_src, suv + vec2(0.0, t.y), gx, gy).rgb);
        detail = luma(src) - n * 0.25;
    }
    float noise = 0.0;
    if (u_grain.x > 0.0) {
        noise = valueNoise(suv * u_srcFull / u_grain.y);
    }
    o = vec4(shade(src, blurL, detail, noise, img), 1.0);
}
