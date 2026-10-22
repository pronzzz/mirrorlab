//! One-tap "Auto" for the Light panel, driven by the histogram of the unedited photo.

use crate::model::Adjustments;

/// Suggested Light settings.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AutoLight {
    pub exposure: f32,
    pub contrast: f32,
    pub highlights: f32,
    pub shadows: f32,
    pub whites: f32,
    pub blacks: f32,
}

impl AutoLight {
    pub fn apply(&self, a: &mut Adjustments) {
        a.exposure = self.exposure;
        a.contrast = self.contrast;
        a.highlights = self.highlights;
        a.shadows = self.shadows;
        a.whites = self.whites;
        a.blacks = self.blacks;
    }
}

fn percentile(bins: &[u32; 256], total: u64, q: f64) -> f32 {
    let target = (total as f64 * q).max(1.0);
    let mut acc = 0u64;
    for (i, n) in bins.iter().enumerate() {
        acc += *n as u64;
        if acc as f64 >= target {
            return i as f32 / 255.0;
        }
    }
    1.0
}

/// `lum` is a 256-bin luminance histogram of the photo before any edits.
pub fn auto_light(lum: &[u32; 256]) -> AutoLight {
    let total: u64 = lum.iter().map(|v| *v as u64).sum();
    if total == 0 {
        return AutoLight::default();
    }
    let lo = percentile(lum, total, 0.005);
    let hi = percentile(lum, total, 0.995);
    let median = percentile(lum, total, 0.5).max(0.01);

    let lin = |v: f32| v.max(1e-4).powf(2.2);
    let enc = |v: f32| v.max(0.0).powf(1.0 / 2.2);
    let ev = ((lin(0.46) / lin(median)).log2() * 0.6).clamp(-1.5, 1.5);
    let gain = 2f32.powf(ev);
    let hi2 = enc(lin(hi) * gain);
    let lo2 = enc(lin(lo) * gain);
    let med2 = enc(lin(median) * gain);

    let mut r = AutoLight { exposure: (ev * 100.0).round() / 100.0, ..Default::default() };
    if hi2 > 0.97 {
        r.highlights = -((hi2 - 0.9) * 350.0).min(60.0);
    } else if hi2 < 0.92 {
        r.whites = ((0.96 - hi2) * 220.0).min(40.0);
    }
    if lo2 > 0.04 {
        r.blacks = -(lo2 * 300.0).min(40.0);
    }
    if med2 < 0.38 {
        r.shadows = ((0.42 - med2) * 160.0).min(40.0);
    }
    let range = hi - lo;
    if range < 0.7 {
        r.contrast = ((0.75 - range) * 60.0).min(25.0);
    }
    for v in [&mut r.contrast, &mut r.highlights, &mut r.shadows, &mut r.whites, &mut r.blacks] {
        *v = v.round();
    }
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gaussian(center: f32, width: f32) -> [u32; 256] {
        let mut b = [0u32; 256];
        for (i, v) in b.iter_mut().enumerate() {
            let x = (i as f32 / 255.0 - center) / width;
            *v = (10_000.0 * (-x * x).exp()) as u32;
        }
        b
    }

    #[test]
    fn dark_photo_gets_brighter() {
        let r = auto_light(&gaussian(0.2, 0.08));
        assert!(r.exposure > 0.3, "{r:?}");
        assert!(r.contrast > 0.0);
    }

    #[test]
    fn bright_photo_gets_darker_and_recovers_highlights() {
        let r = auto_light(&gaussian(0.85, 0.12));
        assert!(r.exposure < 0.0, "{r:?}");
    }

    #[test]
    fn well_exposed_photo_barely_changes() {
        let r = auto_light(&gaussian(0.46, 0.25));
        assert!(r.exposure.abs() < 0.15, "{r:?}");
        assert_eq!(auto_light(&[0; 256]), AutoLight::default());
    }
}
