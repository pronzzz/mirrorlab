//! Debug-only check that the GPU shader matches the Rust reference pipeline.
//! Run `await mirrorlabSelfTest()` in the browser console of a debug build.

use mirror_core::geometry;
use mirror_core::model::{Adjustments, Band, Point, Wheel};
use mirror_core::{render, ImageF32, RenderOptions};
use leptos::prelude::*;
use wasm_bindgen::prelude::*;

use crate::state::Editor;

thread_local! {
    static EDITOR: std::cell::Cell<Option<Editor>> = const { std::cell::Cell::new(None) };
}

/// Called by the app so the console hook can reach the renderer.
pub fn register(ed: Editor) {
    EDITOR.with(|e| e.set(Some(ed)));
}

fn test_image(w: usize, h: usize) -> ImageF32 {
    let mut img = ImageF32::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let hue = x as f32 / w as f32 * 360.0;
            let v = y as f32 / (h - 1) as f32;
            let mut c = mirror_core::pipeline::hsv_to_rgb(hue, 0.15 + 0.85 * ((x * 7 + y * 3) % 17) as f32 / 16.0, v);
            // A little texture so clarity and sharpening have something to bite on.
            if (x / 6 + y / 6) % 2 == 0 {
                for ch in &mut c {
                    *ch = (*ch * 0.92).min(1.0);
                }
            }
            img.set(x, y, c);
        }
    }
    img
}

fn cases() -> Vec<(&'static str, Adjustments)> {
    let d = Adjustments::default;
    let mut curve = d();
    curve.curves.master = vec![Point::new(0.0, 0.05), Point::new(0.3, 0.22), Point::new(0.7, 0.8), Point::new(1.0, 0.95)];
    curve.curves.blue = vec![Point::new(0.0, 0.1), Point::new(1.0, 0.9)];
    let mut hsl = d();
    hsl.hsl[Band::Blue.index()].hue = -60.0;
    hsl.hsl[Band::Green.index()].saturation = -80.0;
    hsl.hsl[Band::Orange.index()].luminance = 50.0;
    let mut grade = d();
    grade.grading.shadows = Wheel { hue: 200.0, saturation: 40.0, luminance: -10.0 };
    grade.grading.highlights = Wheel { hue: 40.0, saturation: 30.0, luminance: 10.0 };
    grade.grading.balance = 20.0;
    let mut geo = d();
    geometry::rotate_cw(&mut geo.geometry);
    geometry::flip_horizontal(&mut geo.geometry);
    geo.geometry.straighten = 7.0;
    geometry::constrain(&mut geo.geometry, 256.0, 192.0);
    vec![
        ("neutral", d()),
        ("exposure", Adjustments { exposure: 1.2, ..d() }),
        ("contrast", Adjustments { contrast: 55.0, ..d() }),
        ("tone", Adjustments { highlights: -70.0, shadows: 60.0, whites: 30.0, blacks: -40.0, ..d() }),
        ("white balance", Adjustments { temperature: 45.0, tint: -25.0, ..d() }),
        ("presence", Adjustments { vibrance: 60.0, saturation: -30.0, ..d() }),
        ("curve", curve),
        ("hsl", hsl),
        ("grading", grade),
        ("clarity+dehaze", Adjustments { clarity: 70.0, dehaze: 40.0, ..d() }),
        ("sharpen", Adjustments { sharpen: 80.0, ..d() }),
        ("vignette", Adjustments { vignette: -60.0, vignette_feather: 30.0, ..d() }),
        ("grain", Adjustments { grain: 60.0, grain_size: 40.0, ..d() }),
        ("geometry", geo),
    ]
}

#[wasm_bindgen(js_name = mirrorlabSelfTest)]
pub fn self_test() -> JsValue {
    let ed = EDITOR.with(|e| e.get());
    let mut out = Vec::new();
    let (w, h) = (256usize, 192usize);
    let src = test_image(w, h);
    // Quantise exactly as the upload does so both sides start from identical pixels.
    let bits = src.to_rgba_f16_bits();
    let quant: Vec<f32> = bits
        .chunks_exact(4)
        .flat_map(|p| [p[0], p[1], p[2]].map(|b| half::f16::from_bits(b).to_f32()))
        .collect();
    let src = ImageF32 { width: w, height: h, data: quant };

    let Some(ed) = ed else { return JsValue::from_str("app not mounted") };
    for (name, adj) in cases() {
        let cpu = render(&src, &adj, RenderOptions::default());
        let gpu = ed.gl.try_update_value(|g| {
            let r = g.as_mut()?;
            r.set_source_f16(w as u32, h as u32, &bits, (w as u32, h as u32)).ok()?;
            Some(r.render_pixels(&adj, cpu.width, cpu.height))
        });
        let Some(Some(gpu)) = gpu else {
            out.push(format!("{name}: GPU unavailable"));
            continue;
        };
        let mut sum = 0u64;
        let mut max = 0u8;
        let mut n = 0u64;
        for (a, b) in cpu.data.chunks_exact(4).zip(gpu.chunks_exact(4)) {
            if a[3] == 0 || b[3] == 0 {
                continue;
            }
            for i in 0..3 {
                let d = a[i].abs_diff(b[i]);
                sum += d as u64;
                max = max.max(d);
                n += 1;
            }
        }
        let mean = sum as f64 / n.max(1) as f64;
        out.push(format!("{name:<15} mean {mean:.3}  max {max:>3}  {}", if mean < 1.0 { "ok" } else { "MISMATCH" }));
    }
    JsValue::from_str(&out.join("\n"))
}
