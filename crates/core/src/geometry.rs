//! Crop / rotate / flip / straighten.
//!
//! Coordinate spaces, all normalised to `0..=1`:
//! * **source** – the decoded photo, origin top-left.
//! * **frame**  – source after quarter turns and flips.
//! * **view**   – frame after straightening around its centre. The crop
//!   rectangle lives here and is always axis aligned.
//! * **output** – the crop rectangle stretched to `0..=1`.
//!
//! The GPU shader implements exactly the same mapping.

use crate::model::{Geometry, Rect};

/// Size of the frame (source after quarter turns) in pixels.
pub fn frame_size(g: &Geometry, src_w: f32, src_h: f32) -> (f32, f32) {
    if g.quarter_turns % 2 == 1 {
        (src_h, src_w)
    } else {
        (src_w, src_h)
    }
}

/// Map a frame coordinate back to the source image.
pub fn frame_to_source(g: &Geometry, u: f32, v: f32) -> (f32, f32) {
    let u = if g.flip_h { 1.0 - u } else { u };
    let v = if g.flip_v { 1.0 - v } else { v };
    match g.quarter_turns % 4 {
        1 => (v, 1.0 - u),
        2 => (1.0 - u, 1.0 - v),
        3 => (1.0 - v, u),
        _ => (u, v),
    }
}

/// Map a view coordinate (straightened frame) to the frame, undoing the rotation.
pub fn view_to_frame(g: &Geometry, fw: f32, fh: f32, x: f32, y: f32) -> (f32, f32) {
    if g.straighten == 0.0 {
        return (x, y);
    }
    let (s, c) = g.straighten.to_radians().sin_cos();
    let dx = (x - 0.5) * fw;
    let dy = (y - 0.5) * fh;
    // Inverse of a clockwise (screen space) rotation.
    let rx = dx * c + dy * s;
    let ry = -dx * s + dy * c;
    (rx / fw + 0.5, ry / fh + 0.5)
}

/// Map an output coordinate (inside the crop) to the source image.
pub fn output_to_source(g: &Geometry, src_w: f32, src_h: f32, u: f32, v: f32) -> (f32, f32) {
    let (fw, fh) = frame_size(g, src_w, src_h);
    let x = g.crop.x + u * g.crop.w;
    let y = g.crop.y + v * g.crop.h;
    let (fx, fy) = view_to_frame(g, fw, fh, x, y);
    frame_to_source(g, fx, fy)
}

/// Pixel size of the rendered output.
pub fn output_size(g: &Geometry, src_w: u32, src_h: u32) -> (u32, u32) {
    let (fw, fh) = frame_size(g, src_w as f32, src_h as f32);
    (((g.crop.w * fw).round() as u32).max(1), ((g.crop.h * fh).round() as u32).max(1))
}

fn rect_inside(g: &Geometry, fw: f32, fh: f32, r: &Rect) -> bool {
    let eps = 0.5 / fw.max(fh).max(1.0);
    let corners = [(r.x, r.y), (r.x + r.w, r.y), (r.x, r.y + r.h), (r.x + r.w, r.y + r.h)];
    corners.iter().all(|&(x, y)| {
        let (u, v) = view_to_frame(g, fw, fh, x, y);
        u >= -eps && u <= 1.0 + eps && v >= -eps && v <= 1.0 + eps
    })
}

/// True if every corner of the crop lands on real image pixels.
pub fn crop_fits(g: &Geometry, src_w: f32, src_h: f32) -> bool {
    let (fw, fh) = frame_size(g, src_w, src_h);
    rect_inside(g, fw, fh, &g.crop)
}

/// Pull the crop back inside the image: clamp to the view, then shrink
/// around its centre (keeping its aspect) until no corner shows empty space.
pub fn constrain(g: &mut Geometry, src_w: f32, src_h: f32) {
    let (fw, fh) = frame_size(g, src_w, src_h);
    let mut r = g.crop;
    r.w = r.w.clamp(0.02, 1.0);
    r.h = r.h.clamp(0.02, 1.0);
    r.x = r.x.clamp(0.0, 1.0 - r.w);
    r.y = r.y.clamp(0.0, 1.0 - r.h);
    if !rect_inside(g, fw, fh, &r) {
        // First try sliding the rectangle towards the centre.
        let (cx, cy) = r.center();
        let mut moved = r;
        let mut lo = 0.0f32;
        let mut hi = 1.0f32;
        for _ in 0..24 {
            let t = (lo + hi) * 0.5;
            moved.x = r.x + (0.5 - cx) * t;
            moved.y = r.y + (0.5 - cy) * t;
            if rect_inside(g, fw, fh, &moved) {
                hi = t;
            } else {
                lo = t;
            }
        }
        moved.x = r.x + (0.5 - cx) * hi;
        moved.y = r.y + (0.5 - cy) * hi;
        if rect_inside(g, fw, fh, &moved) && hi < 1.0 {
            r = moved;
        } else {
            // Then shrink around the centre of the view.
            r.x = 0.5 - r.w * 0.5;
            r.y = 0.5 - r.h * 0.5;
            let base = r;
            let (mut lo, mut hi) = (0.0f32, 1.0f32);
            for _ in 0..30 {
                let s = (lo + hi) * 0.5;
                let t = scaled(&base, s);
                if rect_inside(g, fw, fh, &t) {
                    lo = s;
                } else {
                    hi = s;
                }
            }
            r = scaled(&base, lo);
        }
    }
    g.crop = r;
}

fn scaled(r: &Rect, s: f32) -> Rect {
    let (cx, cy) = r.center();
    Rect { x: cx - r.w * s * 0.5, y: cy - r.h * s * 0.5, w: r.w * s, h: r.h * s }
}

/// Largest centred crop with the given pixel aspect ratio (width / height).
pub fn set_aspect(g: &mut Geometry, src_w: f32, src_h: f32, aspect: Option<f32>) {
    g.aspect = aspect;
    if let Some(a) = aspect {
        let (fw, fh) = frame_size(g, src_w, src_h);
        let frame_aspect = fw / fh;
        let (w, h) = if a > frame_aspect { (1.0, frame_aspect / a) } else { (a / frame_aspect, 1.0) };
        let (cx, cy) = g.crop.center();
        g.crop = Rect { x: (cx - w * 0.5).clamp(0.0, 1.0 - w), y: (cy - h * 0.5).clamp(0.0, 1.0 - h), w, h };
    }
    constrain(g, src_w, src_h);
}

/// Rotate the whole picture 90° clockwise, carrying the crop with it.
pub fn rotate_cw(g: &mut Geometry) {
    g.quarter_turns = (g.quarter_turns + 1) % 4;
    std::mem::swap(&mut g.flip_h, &mut g.flip_v);
    let r = g.crop;
    g.crop = Rect { x: 1.0 - (r.y + r.h), y: r.x, w: r.h, h: r.w };
    g.aspect = g.aspect.map(|a| 1.0 / a);
}

/// Rotate 90° counter-clockwise.
pub fn rotate_ccw(g: &mut Geometry) {
    for _ in 0..3 {
        rotate_cw(g);
    }
}

/// Mirror the picture left/right as seen on screen.
pub fn flip_horizontal(g: &mut Geometry) {
    g.flip_h = !g.flip_h;
    g.straighten = -g.straighten;
    g.crop.x = 1.0 - g.crop.x - g.crop.w;
}

/// Mirror the picture top/bottom as seen on screen.
pub fn flip_vertical(g: &mut Geometry) {
    g.flip_v = !g.flip_v;
    g.straighten = -g.straighten;
    g.crop.y = 1.0 - g.crop.y - g.crop.h;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: (f32, f32), b: (f32, f32)) -> bool {
        (a.0 - b.0).abs() < 1e-4 && (a.1 - b.1).abs() < 1e-4
    }

    #[test]
    fn identity_maps_through() {
        let g = Geometry::default();
        assert!(approx(output_to_source(&g, 400.0, 300.0, 0.25, 0.75), (0.25, 0.75)));
        assert_eq!(output_size(&g, 400, 300), (400, 300));
    }

    #[test]
    fn quarter_turns() {
        let mut g = Geometry::default();
        rotate_cw(&mut g);
        // After a clockwise turn the top-left of the output is the bottom-left of the source.
        assert!(approx(output_to_source(&g, 400.0, 300.0, 0.0, 0.0), (0.0, 1.0)));
        assert_eq!(output_size(&g, 400, 300), (300, 400));
        rotate_cw(&mut g);
        assert!(approx(output_to_source(&g, 400.0, 300.0, 0.0, 0.0), (1.0, 1.0)));
        rotate_ccw(&mut g);
        rotate_ccw(&mut g);
        assert_eq!(g, Geometry::default());
    }

    #[test]
    fn rotation_and_flip_compose() {
        // Flip then rotate must look the same as the equivalent model state.
        let mut g = Geometry::default();
        flip_horizontal(&mut g);
        rotate_cw(&mut g);
        // Visual: mirror, then turn clockwise. Output top-left = source top-right... turned.
        let p = output_to_source(&g, 400.0, 300.0, 0.0, 0.0);
        // Mirror moves source (1,0) to view top-left; turning cw brings view bottom-left to top-left,
        // which was source (1,1).
        assert!(approx(p, (1.0, 1.0)), "{p:?}");
    }

    #[test]
    fn straighten_shrinks_crop_to_fit() {
        let mut g = Geometry { straighten: 10.0, ..Default::default() };
        assert!(!crop_fits(&g, 600.0, 400.0));
        constrain(&mut g, 600.0, 400.0);
        assert!(crop_fits(&g, 600.0, 400.0));
        assert!(g.crop.w < 1.0 && g.crop.w > 0.6);
        // Aspect is preserved while shrinking.
        assert!((g.crop.w - g.crop.h).abs() < 1e-3);
    }

    #[test]
    fn aspect_crop() {
        let mut g = Geometry::default();
        set_aspect(&mut g, 600.0, 400.0, Some(1.0));
        let (w, h) = output_size(&g, 600, 400);
        assert_eq!(w, h);
        assert_eq!(h, 400);
        set_aspect(&mut g, 600.0, 400.0, Some(16.0 / 9.0));
        let (w, h) = output_size(&g, 600, 400);
        assert!(((w as f32 / h as f32) - 16.0 / 9.0).abs() < 0.01);
    }

    #[test]
    fn flips_mirror_crop() {
        let mut g = Geometry { crop: Rect { x: 0.1, y: 0.2, w: 0.3, h: 0.4 }, ..Default::default() };
        flip_horizontal(&mut g);
        assert!((g.crop.x - 0.6).abs() < 1e-6);
        flip_vertical(&mut g);
        assert!((g.crop.y - 0.4).abs() < 1e-6);
    }
}
