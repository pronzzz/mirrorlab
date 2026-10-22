//! Live RGB histogram with clipping warnings.

use leptos::html;
use leptos::prelude::*;
use wasm_bindgen::JsCast;

use crate::platform;
use crate::state::use_editor;

#[component]
pub fn HistogramView() -> impl IntoView {
    let ed = use_editor();
    let canvas = NodeRef::<html::Canvas>::new();

    let redraw = move |c: &web_sys::HtmlCanvasElement| {
        if !c.is_connected() {
            return;
        }
        let h = ed.hist.get_untracked();
        let d = platform::dpr() as f64;
        let w = (c.client_width() as f64 * d).max(1.0);
        let hh = (c.client_height() as f64 * d).max(1.0);
        if c.width() != w as u32 || c.height() != hh as u32 {
            c.set_width(w as u32);
            c.set_height(hh as u32);
        }
        let Ok(Some(ctx)) = c.get_context("2d") else { return };
        let Ok(ctx) = ctx.dyn_into::<web_sys::CanvasRenderingContext2d>() else { return };
        ctx.clear_rect(0.0, 0.0, w, hh);
        if h.len() < 1024 {
            return;
        }
        let colors = ["rgba(255,90,95,0.55)", "rgba(80,210,120,0.5)", "rgba(80,130,255,0.6)"];
        let _ = ctx.set_global_composite_operation("screen");
        for (ch, col) in colors.iter().enumerate() {
            ctx.begin_path();
            ctx.move_to(0.0, hh);
            for i in 0..256 {
                let x = i as f64 / 255.0 * w;
                let y = hh - h[ch * 256 + i] as f64 * (hh - 2.0 * d);
                ctx.line_to(x, y);
            }
            ctx.line_to(w, hh);
            ctx.close_path();
            ctx.set_fill_style_str(col);
            ctx.fill();
        }
        let _ = ctx.set_global_composite_operation("source-over");
        ctx.begin_path();
        for i in 0..256 {
            let x = i as f64 / 255.0 * w;
            let y = hh - h[3 * 256 + i] as f64 * (hh - 2.0 * d);
            if i == 0 {
                ctx.move_to(x, y);
            } else {
                ctx.line_to(x, y);
            }
        }
        ctx.set_stroke_style_str("rgba(255,255,255,0.55)");
        ctx.set_line_width(1.0 * d);
        ctx.stroke();
    };

    Effect::new(move |_| {
        ed.hist.track();
        if let Some(c) = canvas.get() {
            redraw(&c);
        }
    });
    // The column animates open, so redraw whenever the canvas changes size.
    Effect::new(move |_| {
        let Some(c) = canvas.get() else { return };
        let el = c.clone();
        let cb = wasm_bindgen::closure::Closure::<dyn FnMut(js_sys::Array)>::new(move |_: js_sys::Array| redraw(&el));
        if let Ok(ro) = web_sys::ResizeObserver::new(cb.as_ref().unchecked_ref()) {
            ro.observe(&c);
        }
        cb.forget();
    });

    let info = move || {
        ed.loaded.with(|l| match l {
            Some(l) => {
                let mp = l.full_width as f64 * l.full_height as f64 / 1e6;
                let size = format!("{} × {}  ·  {:.1} MP", l.full_width, l.full_height, mp);
                match &l.note {
                    Some(n) => format!("{n}  ·  {size}"),
                    None => size,
                }
            }
            None => String::new(),
        })
    };

    view! {
        <div class="histogram">
            <canvas node_ref=canvas></canvas>
            <span class="clip lo" class:on=move || { ed.clip.get().0 > 0.004 } title="Shadows clipped"></span>
            <span class="clip hi" class:on=move || { ed.clip.get().1 > 0.004 } title="Highlights clipped"></span>
        </div>
        <div class="photo-info">{info}</div>
    }
}
