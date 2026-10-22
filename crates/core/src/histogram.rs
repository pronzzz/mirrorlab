//! RGB + luminance histograms.

/// 256-bin counts per channel: red, green, blue, luminance.
#[derive(Clone, Debug, PartialEq)]
pub struct Histogram {
    pub bins: [[u32; 256]; 4],
    pub total: u32,
}

impl Histogram {
    /// Build from RGBA8 pixels. Fully transparent pixels (outside the photo) are skipped.
    pub fn from_rgba(data: &[u8]) -> Self {
        let mut bins = [[0u32; 256]; 4];
        let mut total = 0;
        for p in data.chunks_exact(4) {
            if p[3] == 0 {
                continue;
            }
            bins[0][p[0] as usize] += 1;
            bins[1][p[1] as usize] += 1;
            bins[2][p[2] as usize] += 1;
            let l = (p[0] as u32 * 54 + p[1] as u32 * 183 + p[2] as u32 * 19) >> 8;
            bins[3][l.min(255) as usize] += 1;
            total += 1;
        }
        Self { bins, total }
    }

    /// Heights in `0..=1` for drawing. Square-root scaled so small peaks stay visible,
    /// and normalised to the tallest interior bin so clipped ends don't flatten the rest.
    pub fn display(&self) -> [[f32; 256]; 4] {
        let mut out = [[0.0f32; 256]; 4];
        let peak = self
            .bins
            .iter()
            .take(3)
            .flat_map(|b| b[1..255].iter())
            .copied()
            .max()
            .unwrap_or(0)
            .max(1) as f32;
        for (c, bins) in self.bins.iter().enumerate() {
            for (i, v) in bins.iter().enumerate() {
                out[c][i] = ((*v as f32 / peak).sqrt()).min(1.0);
            }
        }
        out
    }

    /// Fraction of pixels clipped to pure black / pure white on any channel.
    pub fn clipping(&self) -> (f32, f32) {
        if self.total == 0 {
            return (0.0, 0.0);
        }
        let t = self.total as f32;
        let lo = (0..3).map(|c| self.bins[c][0]).max().unwrap_or(0) as f32 / t;
        let hi = (0..3).map(|c| self.bins[c][255]).max().unwrap_or(0) as f32 / t;
        (lo, hi)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_and_skips_transparent() {
        let px = [255, 0, 0, 255, 0, 0, 0, 0, 128, 128, 128, 255];
        let h = Histogram::from_rgba(&px);
        assert_eq!(h.total, 2);
        assert_eq!(h.bins[0][255], 1);
        assert_eq!(h.bins[3][128], 1);
        let (lo, hi) = h.clipping();
        assert_eq!((lo, hi), (0.5, 0.5));
        assert!(h.display()[1][128] <= 1.0);
    }
}
