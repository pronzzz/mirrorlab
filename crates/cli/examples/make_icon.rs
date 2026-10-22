//! Renders the MirrorLab app icon (1024×1024 PNG) with signed distance fields.
//! `cargo run -p mirrorlab-cli --example make_icon -- out.png [--full-bleed]`

fn coverage(d: f32) -> f32 {
    (0.5 - d).clamp(0.0, 1.0)
}

fn rounded_rect(px: f32, py: f32, cx: f32, cy: f32, half: f32, r: f32) -> f32 {
    let qx = (px - cx).abs() - (half - r);
    let qy = (py - cy).abs() - (half - r);
    let ox = qx.max(0.0);
    let oy = qy.max(0.0);
    (ox * ox + oy * oy).sqrt() + qx.max(qy).min(0.0) - r
}

fn over(dst: [f32; 4], src: [f32; 3], a: f32) -> [f32; 4] {
    let oa = a + dst[3] * (1.0 - a);
    if oa <= 0.0 {
        return [0.0; 4];
    }
    let mut o = [0.0; 4];
    for i in 0..3 {
        o[i] = (src[i] * a + dst[i] * dst[3] * (1.0 - a)) / oa;
    }
    o[3] = oa;
    o
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let out = args.get(1).cloned().unwrap_or_else(|| "icon.png".into());
    let full = args.iter().any(|a| a == "--full-bleed");
    let n = 1024u32;
    let c = n as f32 / 2.0;
    let (half, radius) = if full { (512.0, 0.0) } else { (412.0, 92.0) };
    let ink = [0.945, 0.937, 0.914];
    let mut img = image::RgbaImage::new(n, n);
    for y in 0..n {
        for x in 0..n {
            let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
            let mut col = [0.0f32; 4];
            // Tile with a soft vertical gradient.
            let t = py / n as f32;
            let bg = [0.115 - 0.07 * t, 0.115 - 0.07 * t, 0.125 - 0.075 * t];
            col = over(col, bg, coverage(rounded_rect(px, py, c, c, half, radius)));
            // Ring.
            let r = ((px - c).powi(2) + (py - c).powi(2)).sqrt();
            let ring = (r - 236.0).abs() - 22.0;
            col = over(col, ink, coverage(ring));
            // Left half disc: the "mirror".
            let disc = (r - 236.0).max(px - c);
            col = over(col, ink, coverage(disc));
            let p = image::Rgba([
                (col[0] * 255.0).round() as u8,
                (col[1] * 255.0).round() as u8,
                (col[2] * 255.0).round() as u8,
                (col[3] * 255.0).round() as u8,
            ]);
            img.put_pixel(x, y, p);
        }
    }
    img.save(&out).expect("write icon");
    println!("wrote {out}");
}
