//! The photo stage: GPU canvas, gestures and overlays.

use std::collections::HashMap;

use leptos::html;
use leptos::prelude::*;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{PointerEvent, WheelEvent};

use crate::gl::Renderer;
use crate::icons::{Icon, I};
use crate::platform::{self, now_ms};
use crate::state::use_editor;
use crate::ui::crop::CropOverlay;
use crate::view::{crop_mode, draw, fit_scale, image_rect, request_render};

#[derive(Clone, Default)]
struct Gesture {
    pointers: HashMap<i32, (f32, f32)>,
    start: (f32, f32),
    moved: bool,
    pinch: Option<(f32, f32, (f32, f32))>,
    hold_timer: Option<i32>,
    last_tap: (f64, f32, f32),
}

fn dist(a: (f32, f32), b: (f32, f32)) -> f32 {
    ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt()
}

#[component]
pub fn Stage(#[prop(into)] on_open: Callback<()>) -> impl IntoView {
    let ed = use_editor();
    let stage = NodeRef::<html::Div>::new();
    let canvas = NodeRef::<html::Canvas>::new();
    let gesture = StoredValue::new_local(Gesture::default());

    // Create the renderer once the canvas exists and keep the canvas sized to the stage.
    Effect::new(move |_| {
        let (Some(c), Some(s)) = (canvas.get(), stage.get()) else { return };
        if ed.gl.with_value(|g| g.is_none()) {
            match Renderer::new(&c) {
                Ok(r) => ed.gl.set_value(Some(r)),
                Err(e) => ed.gl_error.set(Some(e)),
            }
        }
        let c2 = c.clone();
        let stage_el = s.clone();
        let on_resize = Closure::<dyn FnMut(js_sys::Array)>::new(move |_entries: js_sys::Array| {
            let s = &stage_el;
            let (w, h) = (s.client_width() as f32, s.client_height() as f32);
            let d = platform::dpr();
            let (pw, ph) = ((w * d).round() as u32, (h * d).round() as u32);
            if c2.width() != pw || c2.height() != ph {
                c2.set_width(pw.max(1));
                c2.set_height(ph.max(1));
            }
            ed.view.stage.set((w, h));
            // Draw synchronously so a resize never shows an empty frame.
            draw(&ed);
        });
        if let Ok(ro) = web_sys::ResizeObserver::new(on_resize.as_ref().unchecked_ref()) {
            ro.observe(&s);
        }
        on_resize.forget();

        // Recover if the GPU context is lost (driver reset, tab in background on mobile).
        let lost = Closure::<dyn FnMut(web_sys::Event)>::new(move |e: web_sys::Event| e.prevent_default());
        let _ = c.add_event_listener_with_callback("webglcontextlost", lost.as_ref().unchecked_ref());
        lost.forget();
        let c3 = c.clone();
        let restored = Closure::<dyn FnMut()>::new(move || {
            if let Ok(r) = Renderer::new(&c3) {
                ed.gl.set_value(Some(r));
                if let Some(id) = ed.current.get_untracked() {
                    ed.loaded.set(None);
                    ed.current.set(None);
                    leptos::task::spawn_local(crate::io::load_photo(ed, id));
                }
            }
        });
        let _ = c.add_event_listener_with_callback("webglcontextrestored", restored.as_ref().unchecked_ref());
        restored.forget();
    });

    let local = move |ev: &PointerEvent| -> (f32, f32) {
        let r = stage.get_untracked().map(|s| s.get_bounding_client_rect());
        let (l, t) = r.map(|r| (r.left() as f32, r.top() as f32)).unwrap_or((0.0, 0.0));
        (ev.client_x() as f32 - l, ev.client_y() as f32 - t)
    };

    let clear_hold = move || {
        gesture.update_value(|g| {
            if let Some(id) = g.hold_timer.take() {
                platform::window().clear_timeout_with_handle(id);
            }
        });
    };

    let on_down = move |ev: PointerEvent| {
        if !ed.has_photo() {
            return;
        }
        let p = local(&ev);
        if let Some(s) = stage.get_untracked() {
            let _ = s.set_pointer_capture(ev.pointer_id());
        }
        let touch = ev.pointer_type() == "touch";
        gesture.update_value(|g| {
            g.pointers.insert(ev.pointer_id(), p);
            g.moved = false;
            g.start = p;
            if g.pointers.len() == 2 {
                let pts: Vec<(f32, f32)> = g.pointers.values().copied().collect();
                let mid = ((pts[0].0 + pts[1].0) * 0.5, (pts[0].1 + pts[1].1) * 0.5);
                g.pinch = Some((dist(pts[0], pts[1]), ed.view.zoom.get_untracked(), mid));
            }
        });
        clear_hold();
        // Long-press on touch shows the original photo.
        if touch && gesture.with_value(|g| g.pointers.len() == 1) && !crop_mode(&ed) {
            let cb = Closure::once_into_js(move || {
                let still = gesture.with_value(|g| !g.moved && g.pointers.len() == 1);
                if still {
                    ed.hold_original.set(true);
                }
            });
            let id = platform::window()
                .set_timeout_with_callback_and_timeout_and_arguments_0(cb.unchecked_ref(), 420)
                .ok();
            gesture.update_value(|g| g.hold_timer = id);
        }
    };

    let on_move = move |ev: PointerEvent| {
        let p = local(&ev);
        let mut pan = None;
        let mut pinch = None;
        gesture.update_value(|g| {
            let Some(prev) = g.pointers.get(&ev.pointer_id()).copied() else { return };
            g.pointers.insert(ev.pointer_id(), p);
            if dist(p, g.start) > 6.0 {
                g.moved = true;
            }
            if g.pointers.len() >= 2 {
                if let Some((d0, z0, m0)) = g.pinch {
                    let pts: Vec<(f32, f32)> = g.pointers.values().copied().collect();
                    let mid = ((pts[0].0 + pts[1].0) * 0.5, (pts[0].1 + pts[1].1) * 0.5);
                    pinch = Some((z0 * dist(pts[0], pts[1]) / d0.max(1.0), mid, (mid.0 - m0.0, mid.1 - m0.1)));
                    g.pinch = Some((d0, z0, mid));
                }
            } else if g.moved {
                pan = Some((p.0 - prev.0, p.1 - prev.1));
            }
        });
        if gesture.with_value(|g| g.moved) {
            clear_hold();
        }
        if let Some((z, mid, dm)) = pinch {
            ed.view.pan_by(&ed, dm.0, dm.1);
            ed.view.zoom_at(&ed, z, Some(mid), false);
        } else if let Some((dx, dy)) = pan {
            if ed.view.zoom.get_untracked() > 1.001 && !crop_mode(&ed) {
                ed.view.pan_by(&ed, dx, dy);
            }
        }
    };

    let toggle_zoom = move |p: (f32, f32)| {
        if crop_mode(&ed) {
            return;
        }
        if ed.view.zoom.get_untracked() > 1.05 {
            ed.view.fit(&ed);
        } else {
            let z = ed.view.one_to_one_zoom(&ed).max(2.5);
            ed.view.zoom_at(&ed, z, Some(p), true);
        }
    };

    let on_up = move |ev: PointerEvent| {
        let p = local(&ev);
        let touch = ev.pointer_type() == "touch";
        clear_hold();
        if ed.hold_original.get_untracked() {
            ed.hold_original.set(false);
        }
        let mut double = false;
        gesture.update_value(|g| {
            g.pointers.remove(&ev.pointer_id());
            if g.pointers.len() < 2 {
                g.pinch = None;
            }
            if touch && !g.moved && g.pointers.is_empty() {
                let now = now_ms();
                let (t, x, y) = g.last_tap;
                if now - t < 300.0 && dist((x, y), p) < 30.0 {
                    double = true;
                    g.last_tap = (0.0, 0.0, 0.0);
                } else {
                    g.last_tap = (now, p.0, p.1);
                }
            }
        });
        if double {
            toggle_zoom(p);
        }
    };

    let on_wheel = move |ev: WheelEvent| {
        if !ed.has_photo() {
            return;
        }
        ev.prevent_default();
        let r = stage.get_untracked().map(|s| s.get_bounding_client_rect());
        let (l, t) = r.map(|r| (r.left() as f32, r.top() as f32)).unwrap_or((0.0, 0.0));
        let p = (ev.client_x() as f32 - l, ev.client_y() as f32 - t);
        let mut dy = ev.delta_y() as f32;
        if ev.delta_mode() == 1 {
            dy *= 16.0;
        }
        let k = if ev.ctrl_key() { 0.012 } else { 0.0025 };
        let z = ed.view.zoom.get_untracked() * (-dy * k).exp();
        ed.view.zoom_at(&ed, z, Some(p), false);
    };

    // Before / after divider.
    let split_drag = StoredValue::new(false);
    let divider_style = move || {
        let r = image_rect(&ed);
        format!("left:{}px;top:{}px;height:{}px", r[0] + r[2] * ed.split.get(), r[1], r[3])
    };
    let move_split = move |ev: PointerEvent| {
        if !split_drag.get_value() {
            return;
        }
        let r = untrack(|| image_rect(&ed));
        let x = local(&ev).0;
        ed.split.set(((x - r[0]) / r[2].max(1.0)).clamp(0.0, 1.0));
        request_render(ed);
    };

    let zoom_label = move || {
        let pct = fit_scale(&ed) * ed.view.zoom.get() * platform::dpr() * 100.0;
        format!("{:.0}%", pct)
    };

    view! {
        <div
            class="stage"
            node_ref=stage
            class:has-photo=move || ed.has_photo()
            class:zoomed=move || { ed.view.zoom.get() > 1.001 }
            on:pointerdown=on_down
            on:pointermove=on_move
            on:pointerup=on_up
            on:pointercancel=on_up
            on:wheel=on_wheel
            on:dblclick=move |ev: web_sys::MouseEvent| {
                let r = stage.get_untracked().map(|s| s.get_bounding_client_rect());
                let (l, t) = r.map(|r| (r.left() as f32, r.top() as f32)).unwrap_or((0.0, 0.0));
                toggle_zoom((ev.client_x() as f32 - l, ev.client_y() as f32 - t));
            }
            on:contextmenu=move |ev| {
                if platform::is_touch() {
                    ev.prevent_default();
                }
            }
        >
            <canvas class="gl" node_ref=canvas class:visible=move || ed.has_photo()></canvas>

            <Show when=move || ed.has_photo() && crop_mode(&ed)>
                <CropOverlay />
            </Show>

            <Show when=move || ed.has_photo() && ed.compare.get() && !crop_mode(&ed)>
                <div
                    class="divider"
                    style=divider_style
                    on:pointerdown=move |ev: PointerEvent| {
                        ev.stop_propagation();
                        if let Some(t) = ev.current_target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()) {
                            let _ = t.set_pointer_capture(ev.pointer_id());
                        }
                        split_drag.set_value(true);
                    }
                    on:pointermove=move_split
                    on:pointerup=move |_| split_drag.set_value(false)
                    on:pointercancel=move |_| split_drag.set_value(false)
                >
                    <span class="divider-knob"><Icon icon=I::Compare size=16 /></span>
                    <span class="divider-tag left">"Before"</span>
                    <span class="divider-tag right">"After"</span>
                </div>
            </Show>

            <div class="badge original" class:show=move || ed.hold_original.get()>"Original"</div>
            <button
                class="badge zoom"
                class:show=move || { ed.has_photo() && ed.view.zoom.get() > 1.001 }
                on:click=move |_| ed.view.fit(&ed)
                on:pointerdown=|ev| ev.stop_propagation()
                title="Fit to screen"
            >
                {zoom_label}
            </button>

            <Show when=move || !ed.has_photo()>
                <div class="empty">
                    {move || match ed.gl_error.get() {
                        Some(e) => view! {
                            <div class="empty-card">
                                <div class="empty-mark"><Icon icon=I::Logo size=44 /></div>
                                <h1>"This browser can't run MirrorLab"</h1>
                                <p class="muted">"MirrorLab needs WebGL2 for real-time editing. Try a current Chrome, Edge, Firefox or Safari."</p>
                                <p class="hint">{e}</p>
                            </div>
                        }.into_any(),
                        None => view! {
                            <div class="empty-card">
                                <div class="empty-mark"><Icon icon=I::Logo size=44 /></div>
                                <h1>"Edit a photo"</h1>
                                <p class="muted">{if platform::is_touch() { "Pick photos from your device" } else { "Drop photos anywhere, or" }}</p>
                                <button class="btn primary large" on:click=move |_| on_open.run(())>
                                    <Icon icon=I::Open size=18 />
                                    "Open photos"
                                </button>
                                <p class="hint">"JPEG · PNG · WebP · HEIC · TIFF · camera RAW"</p>
                                <p class="hint">"Photos never leave your device."</p>
                            </div>
                        }.into_any(),
                    }}
                </div>
            </Show>

            <div class="busy" class:show=move || ed.busy.with(|b| b.is_some())>
                <Icon icon=I::Spinner size=16 class="spin" />
                <span>{move || ed.busy.get().unwrap_or_default()}</span>
            </div>
        </div>
    }
}
