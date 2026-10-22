//! Tone curves: monotone cubic (Fritsch–Carlson) interpolation.
//!
//! Monotone splines pass through every control point and never overshoot,
//! so a curve can't invert tones or ring the way a plain cubic does.

use crate::model::{Curves, Point};

/// Minimum horizontal gap kept between neighbouring points.
pub const MIN_GAP: f32 = 0.01;

/// Number of entries in the lookup tables handed to the renderer.
pub const LUT_SIZE: usize = 1024;

/// Points sorted by x, clamped to the unit square, with duplicate x removed.
pub fn normalized(points: &[Point]) -> Vec<Point> {
    let mut pts: Vec<Point> = points
        .iter()
        .filter(|p| p.x.is_finite() && p.y.is_finite())
        .map(|p| Point::new(p.x.clamp(0.0, 1.0), p.y.clamp(0.0, 1.0)))
        .collect();
    pts.sort_by(|a, b| a.x.total_cmp(&b.x));
    pts.dedup_by(|b, a| (b.x - a.x).abs() < 1e-6);
    if pts.is_empty() {
        pts = vec![Point::new(0.0, 0.0), Point::new(1.0, 1.0)];
    } else if pts.len() == 1 {
        let p = pts[0];
        pts = vec![Point::new(0.0, p.y), Point::new(1.0, p.y)];
    }
    pts
}

pub fn is_identity(points: &[Point]) -> bool {
    normalized(points).iter().all(|p| (p.x - p.y).abs() < 1e-4)
        && normalized(points).first().map(|p| p.x) == Some(0.0)
        && normalized(points).last().map(|p| p.x) == Some(1.0)
}

/// A prepared spline that can be evaluated anywhere in `0..=1`.
#[derive(Clone, Debug)]
pub struct Spline {
    xs: Vec<f32>,
    ys: Vec<f32>,
    ms: Vec<f32>,
}

impl Spline {
    pub fn new(points: &[Point]) -> Self {
        let pts = normalized(points);
        let n = pts.len();
        let xs: Vec<f32> = pts.iter().map(|p| p.x).collect();
        let ys: Vec<f32> = pts.iter().map(|p| p.y).collect();

        // Secant slopes.
        let d: Vec<f32> = (0..n - 1).map(|i| (ys[i + 1] - ys[i]) / (xs[i + 1] - xs[i]).max(1e-6)).collect();

        // Initial tangents.
        let mut ms = vec![0.0f32; n];
        ms[0] = d[0];
        ms[n - 1] = d[n - 2];
        for i in 1..n - 1 {
            ms[i] = if d[i - 1] * d[i] <= 0.0 { 0.0 } else { (d[i - 1] + d[i]) * 0.5 };
        }

        // Fritsch–Carlson: limit tangents to keep each segment monotone.
        for i in 0..n - 1 {
            if d[i] == 0.0 {
                ms[i] = 0.0;
                ms[i + 1] = 0.0;
                continue;
            }
            let a = ms[i] / d[i];
            let b = ms[i + 1] / d[i];
            let s = a * a + b * b;
            if s > 9.0 {
                let t = 3.0 / s.sqrt();
                ms[i] = t * a * d[i];
                ms[i + 1] = t * b * d[i];
            }
        }
        Self { xs, ys, ms }
    }

    pub fn eval(&self, x: f32) -> f32 {
        let n = self.xs.len();
        if x <= self.xs[0] {
            return self.ys[0];
        }
        if x >= self.xs[n - 1] {
            return self.ys[n - 1];
        }
        // Few points: linear scan is fastest.
        let mut i = 0;
        while i < n - 2 && x > self.xs[i + 1] {
            i += 1;
        }
        let h = self.xs[i + 1] - self.xs[i];
        let t = (x - self.xs[i]) / h;
        let t2 = t * t;
        let t3 = t2 * t;
        let h00 = 2.0 * t3 - 3.0 * t2 + 1.0;
        let h10 = t3 - 2.0 * t2 + t;
        let h01 = -2.0 * t3 + 3.0 * t2;
        let h11 = t3 - t2;
        (h00 * self.ys[i] + h10 * h * self.ms[i] + h01 * self.ys[i + 1] + h11 * h * self.ms[i + 1]).clamp(0.0, 1.0)
    }
}

/// Sample a curve into `n` evenly spaced values.
pub fn sample(points: &[Point], n: usize) -> Vec<f32> {
    let s = Spline::new(points);
    (0..n).map(|i| s.eval(i as f32 / (n - 1) as f32)).collect()
}

/// Composite lookup table: master curve followed by each channel's curve.
/// Returns `LUT_SIZE` RGBA entries (alpha = 1), ready for a float texture.
pub fn composite_lut(c: &Curves) -> Vec<f32> {
    let m = Spline::new(&c.master);
    let r = Spline::new(&c.red);
    let g = Spline::new(&c.green);
    let b = Spline::new(&c.blue);
    let mut out = Vec::with_capacity(LUT_SIZE * 4);
    for i in 0..LUT_SIZE {
        let x = i as f32 / (LUT_SIZE - 1) as f32;
        let v = m.eval(x);
        out.extend_from_slice(&[r.eval(v), g.eval(v), b.eval(v), 1.0]);
    }
    out
}

/// Insert a point, returning its index in the sorted list.
pub fn insert_point(points: &mut Vec<Point>, p: Point) -> usize {
    let p = Point::new(p.x.clamp(0.0, 1.0), p.y.clamp(0.0, 1.0));
    let idx = points.iter().position(|q| q.x > p.x).unwrap_or(points.len());
    points.insert(idx, p);
    idx
}

/// Move point `i`, keeping it between its neighbours so the order never flips.
pub fn move_point(points: &mut [Point], i: usize, p: Point) {
    let lo = if i == 0 { 0.0 } else { points[i - 1].x + MIN_GAP };
    let hi = if i + 1 >= points.len() { 1.0 } else { points[i + 1].x - MIN_GAP };
    let x = if lo <= hi { p.x.clamp(lo, hi) } else { points[i].x };
    points[i] = Point::new(x, p.y.clamp(0.0, 1.0));
}

/// Remove an interior point. End points are kept so the curve stays defined.
pub fn remove_point(points: &mut Vec<Point>, i: usize) -> bool {
    if points.len() > 2 && i > 0 && i + 1 < points.len() {
        points.remove(i);
        true
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_lut() {
        let c = Curves::default();
        let lut = composite_lut(&c);
        for i in 0..LUT_SIZE {
            let x = i as f32 / (LUT_SIZE - 1) as f32;
            assert!((lut[i * 4] - x).abs() < 1e-5);
        }
        assert!(c.is_identity());
    }

    #[test]
    fn passes_through_points_and_is_monotone() {
        let pts = vec![Point::new(0.0, 0.0), Point::new(0.25, 0.15), Point::new(0.75, 0.9), Point::new(1.0, 1.0)];
        let s = Spline::new(&pts);
        for p in &pts {
            assert!((s.eval(p.x) - p.y).abs() < 1e-5);
        }
        let v = sample(&pts, 512);
        assert!(v.windows(2).all(|w| w[1] >= w[0] - 1e-6), "curve must not dip");
    }

    #[test]
    fn no_overshoot_on_steep_step() {
        let pts = vec![Point::new(0.0, 0.0), Point::new(0.45, 0.02), Point::new(0.55, 0.98), Point::new(1.0, 1.0)];
        let v = sample(&pts, 1024);
        assert!(v.iter().all(|y| (0.0..=1.0).contains(y)));
        assert!(v.windows(2).all(|w| w[1] >= w[0] - 1e-6));
    }

    #[test]
    fn flat_ends_outside_points() {
        let pts = vec![Point::new(0.2, 0.1), Point::new(0.8, 0.9)];
        let s = Spline::new(&pts);
        assert_eq!(s.eval(0.0), 0.1);
        assert_eq!(s.eval(1.0), 0.9);
    }

    #[test]
    fn editing_keeps_order() {
        let mut pts = Curves::default().master;
        let i = insert_point(&mut pts, Point::new(0.5, 0.7));
        assert_eq!(i, 1);
        move_point(&mut pts, 1, Point::new(1.5, 0.2));
        assert!(pts[1].x < pts[2].x);
        assert!(!remove_point(&mut pts, 0));
        assert!(remove_point(&mut pts, 1));
        assert_eq!(pts.len(), 2);
    }
}
