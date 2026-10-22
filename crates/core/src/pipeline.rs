//! The tone and colour pipeline.
//!
//! [`Prepared`] turns user-facing slider values into the constants the
//! per-pixel maths needs. [`shade`] is the per-pixel function. The GPU
//! fragment shader (`app/src/shader.glsl`) is a line-by-line port of
//! [`shade`], so the live preview, GPU export and this CPU path agree.

use crate::curve;
use crate::geometry;
use crate::image::{ImageF32, ImageU8};
use crate::model::{Adjustments, Band, Zone};

pub const LUMA: [f32; 3] = [0.2126, 0.7152, 0.0722];
pub const GAMMA: f32 = 2.2;

/// Long edge of the low-frequency luminance map used by clarity and dehaze.
pub const BLUR_EDGE: u32 = 320;
/// Gaussian sigma (in blur-map pixels).
pub const BLUR_SIGMA: f32 = 2.0;
pub const BLUR_RADIUS: i32 = 6;

#[inline]
pub fn luma(c: [f32; 3]) -> f32 {
    c[0] * LUMA[0] + c[1] * LUMA[1] + c[2] * LUMA[2]
}

#[inline]
pub fn smoothstep(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[inline]
fn mix(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

#[inline]
fn clamp01(c: [f32; 3]) -> [f32; 3] {
    [c[0].clamp(0.0, 1.0), c[1].clamp(0.0, 1.0), c[2].clamp(0.0, 1.0)]
}

/// Film-like S curve around mid grey. `k > 1` adds contrast, `k < 1` flattens.
#[inline]
pub fn s_curve(x: f32, k: f32) -> f32 {
    if x < 0.5 {
        0.5 * (2.0 * x).powf(k)
    } else {
        1.0 - 0.5 * (2.0 - 2.0 * x).powf(k)
    }
}

/// RGB (0..1) to HSV with hue in degrees.
pub fn rgb_to_hsv(c: [f32; 3]) -> (f32, f32, f32) {
    let mx = c[0].max(c[1]).max(c[2]);
    let mn = c[0].min(c[1]).min(c[2]);
    let d = mx - mn;
    let h = if d <= 1e-6 {
        0.0
    } else if mx == c[0] {
        60.0 * ((c[1] - c[2]) / d).rem_euclid(6.0)
    } else if mx == c[1] {
        60.0 * ((c[2] - c[0]) / d + 2.0)
    } else {
        60.0 * ((c[0] - c[1]) / d + 4.0)
    };
    let s = if mx <= 1e-6 { 0.0 } else { d / mx };
    (h, s, mx)
}

pub fn hsv_to_rgb(h: f32, s: f32, v: f32) -> [f32; 3] {
    let h = h.rem_euclid(360.0) / 60.0;
    let c = v * s;
    let x = c * (1.0 - ((h % 2.0) - 1.0).abs());
    let (r, g, b) = match h as i32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = v - c;
    [r + m, g + m, b + m]
}

/// Weights of the two HSL bands a hue falls between. The weights always sum
/// to one, so neighbouring colours blend without banding.
pub fn band_weights(hue: f32) -> (usize, usize, f32) {
    let c = Band::CENTERS;
    let h = hue.rem_euclid(360.0);
    for i in 0..8 {
        let a = c[i];
        let b = if i == 7 { 360.0 } else { c[i + 1] };
        if h >= a && h < b {
            let t = smoothstep(0.0, 1.0, (h - a) / (b - a));
            return (i, (i + 1) % 8, t);
        }
    }
    (0, 1, 0.0)
}

/// Deterministic 2D hash in `0..1`. Matches `hash2` in the shader bit for bit.
#[inline]
pub fn hash2(x: u32, y: u32) -> f32 {
    let mut h = x.wrapping_mul(0x8da6_b343) ^ y.wrapping_mul(0xd816_3841);
    h ^= h >> 16;
    h = h.wrapping_mul(0x7feb_352d);
    h ^= h >> 15;
    h = h.wrapping_mul(0x846c_a68b);
    h ^= h >> 16;
    (h >> 8) as f32 / 16_777_216.0
}

/// Smooth value noise in `0..1`, `p` in grain cells.
pub fn value_noise(px: f32, py: f32) -> f32 {
    let ix = px.floor();
    let iy = py.floor();
    let fx = px - ix;
    let fy = py - iy;
    let (x0, y0) = (ix as i64 as u32, iy as i64 as u32);
    let a = hash2(x0, y0);
    let b = hash2(x0.wrapping_add(1), y0);
    let c = hash2(x0, y0.wrapping_add(1));
    let d = hash2(x0.wrapping_add(1), y0.wrapping_add(1));
    let ux = fx * fx * (3.0 - 2.0 * fx);
    let uy = fy * fy * (3.0 - 2.0 * fy);
    mix(mix(a, b, ux), mix(c, d, ux), uy)
}

/// Pipeline constants derived from [`Adjustments`].
#[derive(Clone, Debug, PartialEq)]
pub struct Prepared {
    /// White balance gains with exposure folded in (linear light).
    pub gain: [f32; 3],
    pub highlights: f32,
    pub shadows: f32,
    pub whites: f32,
    pub blacks: f32,
    pub contrast: f32,
    pub clarity: f32,
    pub dehaze: f32,
    pub sharpen: f32,
    pub saturation: f32,
    pub vibrance: f32,
    /// Per band: hue shift (degrees), saturation (-1..1), luminance (-1..1).
    pub hsl: [[f32; 3]; 8],
    pub hsl_active: bool,
    /// Per zone: tint rgb offset and luminance offset.
    pub grade: [[f32; 4]; 3],
    pub grade_active: bool,
    pub balance: f32,
    pub vignette: f32,
    pub feather: f32,
    pub grain: f32,
    /// Grain cell size in source pixels.
    pub grain_cell: f32,
    /// Composite curve LUT, `curve::LUT_SIZE` RGBA entries.
    pub lut: Vec<f32>,
    pub curves_active: bool,
}

impl Prepared {
    pub fn new(a: &Adjustments) -> Self {
        let t = a.temperature / 100.0;
        let m = a.tint / 100.0;
        let mut gain = [(1.0 + 0.35 * t) * (1.0 + 0.1 * m), 1.0 - 0.3 * m, (1.0 - 0.35 * t) * (1.0 + 0.1 * m)];
        let norm = luma(gain);
        let ev = 2f32.powf(a.exposure);
        for g in &mut gain {
            *g = *g / norm * ev;
        }

        let mut hsl = [[0.0f32; 3]; 8];
        for (i, s) in a.hsl.iter().enumerate() {
            hsl[i] = [s.hue / 100.0 * 30.0, s.saturation / 100.0, s.luminance / 100.0];
        }
        let hsl_active = a.hsl.iter().any(|s| !s.is_zero());

        let mut grade = [[0.0f32; 4]; 3];
        for (i, z) in Zone::ALL.iter().enumerate() {
            let w = a.grading.zone(*z);
            let tc = hsv_to_rgb(w.hue, 1.0, 1.0);
            let tl = luma(tc);
            let k = w.saturation / 100.0 * 0.3;
            grade[i] = [(tc[0] - tl) * k, (tc[1] - tl) * k, (tc[2] - tl) * k, w.luminance / 100.0 * 0.25];
        }

        let curves_active = !a.curves.is_identity();
        Self {
            gain,
            highlights: a.highlights / 100.0,
            shadows: a.shadows / 100.0,
            whites: a.whites / 100.0,
            blacks: a.blacks / 100.0,
            contrast: a.contrast / 100.0,
            clarity: a.clarity / 100.0,
            dehaze: a.dehaze / 100.0,
            sharpen: a.sharpen / 100.0,
            saturation: a.saturation / 100.0,
            vibrance: a.vibrance / 100.0,
            hsl,
            hsl_active,
            grade,
            grade_active: !a.grading.is_neutral(),
            balance: a.grading.balance / 100.0,
            vignette: a.vignette / 100.0,
            feather: a.vignette_feather / 100.0,
            grain: a.grain / 100.0,
            grain_cell: 1.0 + a.grain_size / 100.0 * 3.0,
            lut: if curves_active { curve::composite_lut(&a.curves) } else { Vec::new() },
            curves_active,
        }
    }

    #[inline]
    fn lut(&self, ch: usize, x: f32) -> f32 {
        let n = curve::LUT_SIZE;
        let f = x.clamp(0.0, 1.0) * (n - 1) as f32;
        let i = (f as usize).min(n - 2);
        let t = f - i as f32;
        mix(self.lut[i * 4 + ch], self.lut[(i + 1) * 4 + ch], t)
    }
}

/// Inputs that come from the neighbourhood of a pixel rather than the pixel itself.
#[derive(Clone, Copy, Debug, Default)]
pub struct Local {
    /// Blurred source luminance at this pixel.
    pub blur: f32,
    /// Source luminance minus the average of its 4 neighbours.
    pub detail: f32,
    /// Grain noise in `0..1`.
    pub noise: f32,
    /// Position inside the output frame, `0..1`.
    pub uv: [f32; 2],
}

/// Process one pixel. `src` is display-referred RGB (sRGB-like encoding),
/// which may exceed 1.0 for RAW files with highlight headroom.
pub fn shade(p: &Prepared, src: [f32; 3], loc: Local) -> [f32; 3] {
    // 1. White balance and exposure in linear light.
    let mut c = [0.0f32; 3];
    for i in 0..3 {
        c[i] = (src[i].max(0.0).powf(GAMMA) * p.gain[i]).powf(1.0 / GAMMA);
    }

    // 2. Dehaze: remove (or add) an airlight estimate from the blur map.
    if p.dehaze != 0.0 {
        let a = p.dehaze * 0.4 * loc.blur.clamp(0.0, 1.0);
        let d = (1.0 - a).max(0.05);
        for v in &mut c {
            *v = (*v - a) / d;
        }
    }

    // 3. Local contrast: clarity (large radius) and sharpening (1px).
    let src_l = luma(src);
    if p.clarity != 0.0 {
        let l = luma(c).clamp(0.0, 1.0);
        let mid = 1.0 - (2.0 * l - 1.0) * (2.0 * l - 1.0);
        let d = (src_l - loc.blur) * p.clarity * 1.2 * mid;
        for v in &mut c {
            *v += d;
        }
    }
    if p.sharpen > 0.0 {
        let d = loc.detail * p.sharpen * 2.0;
        for v in &mut c {
            *v += d;
        }
    }

    // 4. Tone: highlights, shadows, whites, blacks act on luminance.
    let l = luma(c);
    let lc = l.max(0.0);
    let dl = p.highlights * 0.32 * smoothstep(0.45, 1.0, lc)
        + p.shadows * 0.32 * (1.0 - smoothstep(0.0, 0.55, lc))
        + p.whites * 0.25 * smoothstep(0.7, 1.1, lc)
        + p.blacks * 0.18 * (1.0 - smoothstep(0.0, 0.3, lc));
    if dl != 0.0 {
        let l2 = (l + dl).max(0.0);
        let ratio = l2 / l.max(1e-4);
        let blend = smoothstep(0.02, 0.2, l);
        for v in &mut c {
            *v = mix(*v + (l2 - l), *v * ratio, blend);
        }
    }
    c = clamp01(c);

    // 5. Contrast.
    if p.contrast != 0.0 {
        let k = (p.contrast * 0.85).exp();
        for v in &mut c {
            *v = s_curve(*v, k);
        }
    }

    // 6. Tone curve.
    if p.curves_active {
        c = [p.lut(0, c[0]), p.lut(1, c[1]), p.lut(2, c[2])];
    }

    // 7. Saturation and vibrance.
    let sat = p.saturation + p.dehaze * 0.2;
    if sat != 0.0 {
        let l = luma(c);
        for v in &mut c {
            *v = l + (*v - l) * (1.0 + sat);
        }
    }
    if p.vibrance != 0.0 {
        let l = luma(c);
        let mx = c[0].max(c[1]).max(c[2]);
        let mn = c[0].min(c[1]).min(c[2]);
        let s = (mx - mn).clamp(0.0, 1.0);
        let f = 1.0 + p.vibrance * (1.0 - s);
        for v in &mut c {
            *v = l + (*v - l) * f;
        }
    }
    c = clamp01(c);

    // 8. HSL mixer.
    if p.hsl_active {
        let (h, s, v) = rgb_to_hsv(c);
        let chroma = c[0].max(c[1]).max(c[2]) - c[0].min(c[1]).min(c[2]);
        let cw = smoothstep(0.0, 0.15, chroma);
        let (a, b, t) = band_weights(h);
        let dh = mix(p.hsl[a][0], p.hsl[b][0], t) * cw;
        let ds = mix(p.hsl[a][1], p.hsl[b][1], t) * cw;
        let dv = mix(p.hsl[a][2], p.hsl[b][2], t) * cw;
        c = hsv_to_rgb(h + dh, s, v);
        let l = luma(c);
        for x in &mut c {
            *x = (l + (*x - l) * (1.0 + ds)) * (1.0 + dv * 0.45);
        }
        c = clamp01(c);
    }

    // 9. Colour grading.
    if p.grade_active {
        let l = luma(c);
        let pivot = 0.5 + p.balance * 0.25;
        let ws = 1.0 - smoothstep(0.0, pivot, l);
        let wh = smoothstep(pivot, 1.0, l);
        let wm = (1.0 - ws - wh).max(0.0);
        for (z, w) in [ws, wm, wh].into_iter().enumerate() {
            let g = p.grade[z];
            for i in 0..3 {
                c[i] += (g[i] + g[3]) * w;
            }
        }
        c = clamp01(c);
    }

    // 10. Vignette, relative to the cropped frame.
    if p.vignette != 0.0 {
        let dx = (loc.uv[0] - 0.5) * 2.0;
        let dy = (loc.uv[1] - 0.5) * 2.0;
        let d = (dx * dx + dy * dy).sqrt() * std::f32::consts::FRAC_1_SQRT_2;
        let inner = 0.5 * (1.0 - p.feather) + 0.1;
        let w = smoothstep(inner, 1.05, d);
        if p.vignette < 0.0 {
            for v in &mut c {
                *v *= 1.0 + p.vignette * w;
            }
        } else {
            for v in &mut c {
                *v += (1.0 - *v) * p.vignette * w;
            }
        }
    }

    // 11. Film grain, strongest in the midtones.
    if p.grain > 0.0 {
        let l = luma(c);
        let amp = p.grain * 0.2 * (1.0 - 0.5 * (2.0 * l - 1.0).abs());
        let n = (loc.noise - 0.5) * 2.0 * amp;
        for v in &mut c {
            *v += n;
        }
    }
    clamp01(c)
}

/// Low-frequency luminance map of a source image.
#[derive(Clone, Debug)]
pub struct BlurMap {
    pub w: usize,
    pub h: usize,
    pub data: Vec<f32>,
}

impl BlurMap {
    /// Area-average the source luminance down to [`BLUR_EDGE`] and blur it.
    pub fn new(src: &ImageF32) -> Self {
        let scale = (BLUR_EDGE as f32 / src.width.max(src.height) as f32).min(1.0);
        let w = ((src.width as f32 * scale).round() as usize).max(1);
        let h = ((src.height as f32 * scale).round() as usize).max(1);
        let mut small = vec![0.0f32; w * h];
        let mut count = vec![0u32; w * h];
        for y in 0..src.height {
            let ty = (y * h / src.height).min(h - 1);
            for x in 0..src.width {
                let tx = (x * w / src.width).min(w - 1);
                small[ty * w + tx] += luma(src.rgb(x, y));
                count[ty * w + tx] += 1;
            }
        }
        for (v, n) in small.iter_mut().zip(&count) {
            *v /= (*n).max(1) as f32;
        }
        let k = gaussian_kernel();
        let mut tmp = vec![0.0f32; w * h];
        for y in 0..h {
            for x in 0..w {
                let mut acc = 0.0;
                for (i, kw) in k.iter().enumerate() {
                    let sx = (x as i32 + i as i32 - BLUR_RADIUS).clamp(0, w as i32 - 1) as usize;
                    acc += small[y * w + sx] * kw;
                }
                tmp[y * w + x] = acc;
            }
        }
        let mut data = vec![0.0f32; w * h];
        for y in 0..h {
            for x in 0..w {
                let mut acc = 0.0;
                for (i, kw) in k.iter().enumerate() {
                    let sy = (y as i32 + i as i32 - BLUR_RADIUS).clamp(0, h as i32 - 1) as usize;
                    acc += tmp[sy * w + x] * kw;
                }
                data[y * w + x] = acc;
            }
        }
        Self { w, h, data }
    }

    /// Bilinear sample at a normalised source coordinate.
    pub fn sample(&self, u: f32, v: f32) -> f32 {
        let fx = (u * self.w as f32 - 0.5).clamp(0.0, (self.w - 1) as f32);
        let fy = (v * self.h as f32 - 0.5).clamp(0.0, (self.h - 1) as f32);
        let x0 = fx as usize;
        let y0 = fy as usize;
        let x1 = (x0 + 1).min(self.w - 1);
        let y1 = (y0 + 1).min(self.h - 1);
        let tx = fx - x0 as f32;
        let ty = fy - y0 as f32;
        let d = &self.data;
        mix(mix(d[y0 * self.w + x0], d[y0 * self.w + x1], tx), mix(d[y1 * self.w + x0], d[y1 * self.w + x1], tx), ty)
    }
}

/// Normalised Gaussian weights, `2 * BLUR_RADIUS + 1` taps.
pub fn gaussian_kernel() -> Vec<f32> {
    let k: Vec<f32> = (-BLUR_RADIUS..=BLUR_RADIUS)
        .map(|i| (-(i * i) as f32 / (2.0 * BLUR_SIGMA * BLUR_SIGMA)).exp())
        .collect();
    let sum: f32 = k.iter().sum();
    k.into_iter().map(|v| v / sum).collect()
}

/// Options for a CPU render.
#[derive(Clone, Copy, Debug, Default)]
pub struct RenderOptions {
    /// Scale the output so its long edge is at most this many pixels.
    pub long_edge: Option<u32>,
}

/// Render the full pipeline on the CPU (multi-threaded with the `parallel` feature).
pub fn render(src: &ImageF32, adj: &Adjustments, opts: RenderOptions) -> ImageU8 {
    let p = Prepared::new(adj);
    let g = adj.geometry;
    let (mut ow, mut oh) = geometry::output_size(&g, src.width as u32, src.height as u32);
    if let Some(le) = opts.long_edge {
        let s = le as f32 / ow.max(oh) as f32;
        if s < 1.0 {
            ow = ((ow as f32 * s).round() as u32).max(1);
            oh = ((oh as f32 * s).round() as u32).max(1);
        }
    }
    // Pre-shrink very large sources so bilinear sampling doesn't alias.
    let (full_w, _) = geometry::output_size(&g, src.width as u32, src.height as u32);
    let shrink = (full_w as f32 / ow as f32).floor() as usize;
    let reduced;
    let source = if shrink >= 2 {
        reduced = src.box_downsample(shrink);
        &reduced
    } else {
        src
    };
    let blur = if p.clarity != 0.0 || p.dehaze != 0.0 { Some(BlurMap::new(source)) } else { None };
    let sw = source.width as f32;
    let sh = source.height as f32;

    let row = |y: usize, out: &mut [u8]| {
        for x in 0..ow as usize {
            let u = (x as f32 + 0.5) / ow as f32;
            let v = (y as f32 + 0.5) / oh as f32;
            let (su, sv) = geometry::output_to_source(&g, sw, sh, u, v);
            let px = &mut out[x * 4..x * 4 + 4];
            if !(0.0..=1.0).contains(&su) || !(0.0..=1.0).contains(&sv) {
                px.copy_from_slice(&[0, 0, 0, 255]);
                continue;
            }
            let c = source.sample(su, sv);
            let detail = if p.sharpen > 0.0 {
                let dx = 1.0 / sw;
                let dy = 1.0 / sh;
                let n = luma(source.sample(su - dx, sv))
                    + luma(source.sample(su + dx, sv))
                    + luma(source.sample(su, sv - dy))
                    + luma(source.sample(su, sv + dy));
                luma(c) - n * 0.25
            } else {
                0.0
            };
            let noise = if p.grain > 0.0 {
                let scale = src.width as f32 / sw;
                value_noise(su * sw * scale / p.grain_cell, sv * sh * scale / p.grain_cell)
            } else {
                0.0
            };
            let loc = Local {
                blur: blur.as_ref().map(|b| b.sample(su, sv)).unwrap_or(0.0),
                detail,
                noise,
                uv: [u, v],
            };
            let o = shade(&p, c, loc);
            px.copy_from_slice(&[to_u8(o[0]), to_u8(o[1]), to_u8(o[2]), 255]);
        }
    };

    let mut out = ImageU8::new(ow, oh);
    let stride = ow as usize * 4;
    #[cfg(feature = "parallel")]
    {
        use rayon::prelude::*;
        out.data.par_chunks_mut(stride).enumerate().for_each(|(y, r)| row(y, r));
    }
    #[cfg(not(feature = "parallel"))]
    {
        out.data.chunks_mut(stride).enumerate().for_each(|(y, r)| row(y, r));
    }
    out
}

#[inline]
pub fn to_u8(v: f32) -> u8 {
    (v.clamp(0.0, 1.0) * 255.0 + 0.5) as u8
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{HslShift, Point};

    fn px(p: &Prepared, c: [f32; 3]) -> [f32; 3] {
        shade(p, c, Local { blur: luma(c), uv: [0.5, 0.5], ..Default::default() })
    }

    #[test]
    fn neutral_is_identity() {
        let p = Prepared::new(&Adjustments::default());
        for c in [[0.0, 0.0, 0.0], [0.2, 0.5, 0.8], [1.0, 1.0, 1.0], [0.9, 0.1, 0.4]] {
            let o = px(&p, c);
            for i in 0..3 {
                assert!((o[i] - c[i]).abs() < 1e-4, "{c:?} -> {o:?}");
            }
        }
    }

    #[test]
    fn exposure_one_stop_doubles_linear_light() {
        let a = Adjustments { exposure: 1.0, ..Default::default() };
        let o = px(&Prepared::new(&a), [0.4, 0.4, 0.4]);
        let expected = (0.4f32.powf(GAMMA) * 2.0).powf(1.0 / GAMMA);
        assert!((o[0] - expected).abs() < 1e-4);
    }

    #[test]
    fn white_balance_keeps_grey_brightness() {
        let a = Adjustments { temperature: 60.0, ..Default::default() };
        let o = px(&Prepared::new(&a), [0.5, 0.5, 0.5]);
        assert!(o[0] > o[2], "warmer means more red than blue");
        let lin = |v: f32| v.powf(GAMMA);
        let l = LUMA[0] * lin(o[0]) + LUMA[1] * lin(o[1]) + LUMA[2] * lin(o[2]);
        assert!((l - lin(0.5)).abs() < 0.01);
    }

    #[test]
    fn contrast_keeps_mid_grey() {
        for c in [-100.0, -40.0, 50.0, 100.0] {
            let a = Adjustments { contrast: c, ..Default::default() };
            let o = px(&Prepared::new(&a), [0.5, 0.5, 0.5]);
            assert!((o[0] - 0.5).abs() < 1e-4);
        }
        let a = Adjustments { contrast: 60.0, ..Default::default() };
        let o = px(&Prepared::new(&a), [0.25, 0.25, 0.25]);
        assert!(o[0] < 0.25);
    }

    #[test]
    fn shadows_lift_dark_tones_more_than_bright() {
        let a = Adjustments { shadows: 80.0, ..Default::default() };
        let p = Prepared::new(&a);
        let dark = px(&p, [0.15, 0.15, 0.15])[0] - 0.15;
        let bright = px(&p, [0.85, 0.85, 0.85])[0] - 0.85;
        assert!(dark > 0.05);
        assert!(bright.abs() < 0.01);
    }

    #[test]
    fn desaturate_fully() {
        let a = Adjustments { saturation: -100.0, ..Default::default() };
        let o = px(&Prepared::new(&a), [0.8, 0.3, 0.1]);
        assert!((o[0] - o[1]).abs() < 1e-5 && (o[1] - o[2]).abs() < 1e-5);
    }

    #[test]
    fn hsl_band_only_touches_its_colours() {
        let mut a = Adjustments::default();
        a.hsl[Band::Blue.index()] = HslShift { hue: 0.0, saturation: -100.0, luminance: 0.0 };
        let p = Prepared::new(&a);
        let blue = px(&p, [0.1, 0.3, 0.9]);
        let (_, s, _) = rgb_to_hsv(blue);
        assert!(s < 0.05, "blue should be grey, got {blue:?}");
        let red = px(&p, [0.9, 0.1, 0.1]);
        assert!((red[0] - 0.9).abs() < 1e-3 && (red[1] - 0.1).abs() < 1e-3);
    }

    #[test]
    fn band_weights_are_a_partition() {
        for h in 0..360 {
            let (a, b, t) = band_weights(h as f32);
            assert!(a < 8 && b < 8 && (0.0..=1.0).contains(&t));
        }
        assert_eq!(band_weights(0.0), (0, 1, 0.0));
        let (a, _, t) = band_weights(225.0);
        assert_eq!((a, t), (Band::Blue.index(), 0.0));
    }

    #[test]
    fn curve_applies() {
        let mut a = Adjustments::default();
        a.curves.master = vec![Point::new(0.0, 0.0), Point::new(0.5, 0.75), Point::new(1.0, 1.0)];
        let o = px(&Prepared::new(&a), [0.5, 0.5, 0.5]);
        assert!((o[0] - 0.75).abs() < 2e-3);
    }

    #[test]
    fn vignette_darkens_corners_not_centre() {
        let a = Adjustments { vignette: -80.0, ..Default::default() };
        let p = Prepared::new(&a);
        let centre = shade(&p, [0.6; 3], Local { blur: 0.6, uv: [0.5, 0.5], ..Default::default() });
        let corner = shade(&p, [0.6; 3], Local { blur: 0.6, uv: [0.0, 0.0], ..Default::default() });
        assert!((centre[0] - 0.6).abs() < 1e-4);
        assert!(corner[0] < 0.3);
    }

    #[test]
    fn hash_is_stable() {
        // Pinned so the GLSL port can be checked against it.
        assert_eq!(hash2(0, 0), 0.0);
        let v = hash2(12, 34);
        assert!((0.0..1.0).contains(&v));
        assert_eq!(v, hash2(12, 34));
        assert_ne!(hash2(12, 34), hash2(34, 12));
    }

    #[test]
    fn cpu_render_crops_and_rotates() {
        let mut src = ImageF32::new(40, 20);
        for y in 0..20 {
            for x in 0..40 {
                src.set(x, y, [x as f32 / 39.0, y as f32 / 19.0, 0.0]);
            }
        }
        let mut a = Adjustments::default();
        crate::geometry::rotate_cw(&mut a.geometry);
        let out = render(&src, &a, RenderOptions::default());
        assert_eq!((out.width, out.height), (20, 40));
        // Top-left of a clockwise-rotated image was the bottom-left of the source: red≈0, green≈1.
        let p = out.pixel(0, 0);
        assert!(p[0] < 10 && p[1] > 240, "{p:?}");
        let small = render(&src, &Adjustments::default(), RenderOptions { long_edge: Some(10) });
        assert_eq!((small.width, small.height), (10, 5));
    }
}
