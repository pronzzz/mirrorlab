//! Minimal image buffers used by the engine.

/// Display-referred RGB, 3 floats per pixel. Values may exceed 1.0 (RAW headroom).
#[derive(Clone, Debug, PartialEq)]
pub struct ImageF32 {
    pub width: usize,
    pub height: usize,
    pub data: Vec<f32>,
}

impl ImageF32 {
    pub fn new(width: usize, height: usize) -> Self {
        Self { width, height, data: vec![0.0; width * height * 3] }
    }

    pub fn from_rgba8(width: usize, height: usize, rgba: &[u8]) -> Self {
        let data = rgba.chunks_exact(4).flat_map(|p| [p[0] as f32 / 255.0, p[1] as f32 / 255.0, p[2] as f32 / 255.0]).collect();
        Self { width, height, data }
    }

    pub fn from_rgb16(width: usize, height: usize, rgb: &[u16]) -> Self {
        Self { width, height, data: rgb.iter().map(|v| *v as f32 / 65535.0).collect() }
    }

    #[inline]
    pub fn rgb(&self, x: usize, y: usize) -> [f32; 3] {
        let i = (y * self.width + x) * 3;
        [self.data[i], self.data[i + 1], self.data[i + 2]]
    }

    #[inline]
    pub fn set(&mut self, x: usize, y: usize, c: [f32; 3]) {
        let i = (y * self.width + x) * 3;
        self.data[i..i + 3].copy_from_slice(&c);
    }

    /// Bilinear sample at a normalised coordinate (pixel centres at `(i + 0.5) / n`).
    #[inline]
    pub fn sample(&self, u: f32, v: f32) -> [f32; 3] {
        let fx = (u * self.width as f32 - 0.5).clamp(0.0, (self.width - 1) as f32);
        let fy = (v * self.height as f32 - 0.5).clamp(0.0, (self.height - 1) as f32);
        let x0 = fx as usize;
        let y0 = fy as usize;
        let x1 = (x0 + 1).min(self.width - 1);
        let y1 = (y0 + 1).min(self.height - 1);
        let tx = fx - x0 as f32;
        let ty = fy - y0 as f32;
        let a = self.rgb(x0, y0);
        let b = self.rgb(x1, y0);
        let c = self.rgb(x0, y1);
        let d = self.rgb(x1, y1);
        let mut o = [0.0; 3];
        for i in 0..3 {
            let top = a[i] + (b[i] - a[i]) * tx;
            let bot = c[i] + (d[i] - c[i]) * tx;
            o[i] = top + (bot - top) * ty;
        }
        o
    }

    /// Average `f`×`f` blocks.
    pub fn box_downsample(&self, f: usize) -> ImageF32 {
        let w = (self.width / f).max(1);
        let h = (self.height / f).max(1);
        let mut out = ImageF32::new(w, h);
        for y in 0..h {
            for x in 0..w {
                let mut acc = [0.0f32; 3];
                let mut n = 0.0;
                for sy in y * f..((y + 1) * f).min(self.height) {
                    for sx in x * f..((x + 1) * f).min(self.width) {
                        let c = self.rgb(sx, sy);
                        acc[0] += c[0];
                        acc[1] += c[1];
                        acc[2] += c[2];
                        n += 1.0;
                    }
                }
                out.set(x, y, [acc[0] / n, acc[1] / n, acc[2] / n]);
            }
        }
        out
    }

    /// Apply an EXIF-style orientation: optional flips first, then an optional transpose.
    pub fn oriented(self, transpose: bool, flip_h: bool, flip_v: bool) -> ImageF32 {
        if !transpose && !flip_h && !flip_v {
            return self;
        }
        let (w, h) = (self.width, self.height);
        let mut out = if transpose { ImageF32::new(h, w) } else { ImageF32::new(w, h) };
        for y in 0..h {
            for x in 0..w {
                let x1 = if flip_h { w - 1 - x } else { x };
                let y1 = if flip_v { h - 1 - y } else { y };
                let (x2, y2) = if transpose { (y1, x1) } else { (x1, y1) };
                out.set(x2, y2, self.rgb(x, y));
            }
        }
        out
    }

    /// Pack into RGBA half floats (bit patterns), for GPU upload.
    pub fn to_rgba_f16_bits(&self) -> Vec<u16> {
        let one = half::f16::from_f32(1.0).to_bits();
        let mut out = Vec::with_capacity(self.width * self.height * 4);
        for p in self.data.chunks_exact(3) {
            out.push(half::f16::from_f32(p[0]).to_bits());
            out.push(half::f16::from_f32(p[1]).to_bits());
            out.push(half::f16::from_f32(p[2]).to_bits());
            out.push(one);
        }
        out
    }

    /// Shrink (never enlarge) so the long edge fits `max_edge`.
    pub fn fit(self, max_edge: usize) -> ImageF32 {
        let long = self.width.max(self.height);
        if long <= max_edge {
            return self;
        }
        let f = long.div_ceil(max_edge);
        self.box_downsample(f)
    }
}

/// 8-bit RGBA output.
#[derive(Clone, Debug, PartialEq)]
pub struct ImageU8 {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

impl ImageU8 {
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height, data: vec![0; width as usize * height as usize * 4] }
    }

    pub fn pixel(&self, x: u32, y: u32) -> [u8; 4] {
        let i = ((y * self.width + x) * 4) as usize;
        [self.data[i], self.data[i + 1], self.data[i + 2], self.data[i + 3]]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn orientation_rotate90() {
        // EXIF 6 (rotate 90 cw) is transpose + vertical flip in rawler's convention.
        let mut img = ImageF32::new(3, 2);
        img.set(0, 0, [1.0, 0.0, 0.0]); // top-left marker
        let out = img.oriented(true, false, true);
        assert_eq!((out.width, out.height), (2, 3));
        // After a clockwise turn the old top-left sits at the top-right.
        assert_eq!(out.rgb(1, 0), [1.0, 0.0, 0.0]);
    }

    #[test]
    fn downsample_and_fit() {
        let img = ImageF32::from_rgba8(4, 4, &[255; 64]);
        let small = img.clone().fit(2);
        assert_eq!((small.width, small.height), (2, 2));
        assert_eq!(small.rgb(1, 1), [1.0, 1.0, 1.0]);
        assert_eq!(img.sample(0.5, 0.5), [1.0, 1.0, 1.0]);
    }
}
