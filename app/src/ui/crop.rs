//! Crop & rotate: the panel and the on-image overlay.

use leptos::prelude::*;
use mirror_core::geometry;
use mirror_core::model::{Geometry, Rect};
use web_sys::PointerEvent;

use crate::icons::{Icon, I};
use crate::state::{use_editor, Editor, Tool};
use crate::ui::panels::Section;
use crate::ui::slider::Slider;
use crate::view::{image_rect, request_render};

fn source_size(ed: &Editor) -> (f32, f32) {
    ed.loaded.with_untracked(|l| l.as_ref().map(|l| (l.width as f32, l.height as f32))).unwrap_or((1.0, 1.0))
}

/// Crop aspect (pixel width / height) of the current crop.
fn crop_aspect(g: &Geometry, w: f32, h: f32) -> f32 {
    let (fw, fh) = geometry::frame_size(g, w, h);
    (g.crop.w * fw) / (g.crop.h * fh).max(1e-6)
}

const ASPECTS: [(&str, Option<f32>); 9] = [
    ("Free", None),
    ("Original", Some(-1.0)),
    ("1:1", Some(1.0)),
    ("4:5", Some(0.8)),
    ("5:4", Some(1.25)),
    ("3:2", Some(1.5)),
    ("2:3", Some(2.0 / 3.0)),
    ("16:9", Some(16.0 / 9.0)),
    ("9:16", Some(9.0 / 16.0)),
];

#[component]
pub fn CropPanel() -> impl IntoView {
    let ed = use_editor();
    let selected = move |a: Option<f32>| {
        ed.adj.with(|adj| match (adj.geometry.aspect, a) {
            (None, None) => true,
            (Some(x), Some(-1.0)) => {
                let (w, h) = ed.loaded.with(|l| l.as_ref().map(|l| (l.width as f32, l.height as f32)).unwrap_or((1.0, 1.0)));
                let (fw, fh) = geometry::frame_size(&adj.geometry, w, h);
                (x - fw / fh).abs() < 1e-3 && (x - 1.0).abs() > 1e-3
            }
            (Some(x), Some(y)) => (x - y).abs() < 1e-3,
            _ => false,
        })
    };
    let set_aspect = move |a: Option<f32>| {
        let (w, h) = source_size(&ed);
        ed.apply(|adj| {
            let g = &mut adj.geometry;
            let a = match a {
                Some(v) if v < 0.0 => {
                    let (fw, fh) = geometry::frame_size(g, w, h);
                    Some(fw / fh)
                }
                other => other,
            };
            match a {
                Some(_) => geometry::set_aspect(g, w, h, a),
                None => g.aspect = None,
            }
        });
    };

    let straighten = Signal::derive(move || ed.adj.with(|a| a.geometry.straighten));
    let set_straighten = Callback::new(move |v: f32| {
        let (w, h) = source_size(&ed);
        ed.adj.update(|a| {
            let g = &mut a.geometry;
            let aspect = g.aspect;
            let keep = crop_aspect(g, w, h);
            g.straighten = v;
            // Largest crop of the same shape that still fits the rotated photo.
            geometry::set_aspect(g, w, h, Some(keep));
            g.aspect = aspect;
        });
    });

    view! {
        <div class="panel">
            <Section title="Aspect ratio">
                <div class="chips">
                    {ASPECTS
                        .into_iter()
                        .map(|(label, a)| {
                            view! {
                                <button class="chip" class:selected=move || selected(a) on:click=move |_| set_aspect(a)>
                                    {label}
                                </button>
                            }
                        })
                        .collect_view()}
                </div>
            </Section>
            <Section title="Rotate & flip">
                <div class="button-row">
                    <button class="tool-btn" title="Rotate left" on:click=move |_| ed.apply(|a| geometry::rotate_ccw(&mut a.geometry))>
                        <Icon icon=I::RotateLeft />
                        <span>"Left"</span>
                    </button>
                    <button class="tool-btn" title="Rotate right" on:click=move |_| ed.apply(|a| geometry::rotate_cw(&mut a.geometry))>
                        <Icon icon=I::RotateRight />
                        <span>"Right"</span>
                    </button>
                    <button class="tool-btn" title="Flip horizontal" on:click=move |_| ed.apply(|a| geometry::flip_horizontal(&mut a.geometry))>
                        <Icon icon=I::FlipH />
                        <span>"Flip H"</span>
                    </button>
                    <button class="tool-btn" title="Flip vertical" on:click=move |_| ed.apply(|a| geometry::flip_vertical(&mut a.geometry))>
                        <Icon icon=I::FlipV />
                        <span>"Flip V"</span>
                    </button>
                </div>
                <Slider label="Straighten" value=straighten set=set_straighten min=-45.0 max=45.0 step=0.1 decimals=1 unit="°" />
            </Section>
            <div class="panel-actions">
                <button
                    class="btn ghost"
                    disabled=move || ed.adj.with(|a| a.geometry.is_identity())
                    on:click=move |_| ed.apply(|a| a.geometry = Geometry::default())
                >
                    <Icon icon=I::Reset size=16 />
                    "Reset crop"
                </button>
                <button
                    class="btn primary"
                    on:click=move |_| {
                        if ed.mobile.get_untracked() {
                            ed.sheet_open.set(false);
                        } else {
                            ed.tool.set(Tool::Light);
                        }
                        ed.view.fit(&ed);
                    }
                >
                    <Icon icon=I::Check size=16 />
                    "Done"
                </button>
            </div>
        </div>
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Handle {
    Move,
    N,
    S,
    E,
    W,
    NE,
    NW,
    SE,
    SW,
}

#[derive(Clone, Copy)]
struct CropDrag {
    handle: Handle,
    start: (f32, f32),
    rect: Rect,
    /// Pixel size of the view (straightened frame) on screen.
    view_px: (f32, f32),
}

fn adjust(r: Rect, h: Handle, dx: f32, dy: f32, aspect: Option<f32>, frame: (f32, f32)) -> Rect {
    let min = 0.04;
    let (mut x0, mut y0, mut x1, mut y1) = (r.x, r.y, r.x + r.w, r.y + r.h);
    match h {
        Handle::Move => {
            let w = r.w;
            let hh = r.h;
            x0 = (r.x + dx).clamp(0.0, 1.0 - w);
            y0 = (r.y + dy).clamp(0.0, 1.0 - hh);
            return Rect { x: x0, y: y0, w, h: hh };
        }
        _ => {
            if matches!(h, Handle::W | Handle::NW | Handle::SW) {
                x0 = (x0 + dx).clamp(0.0, x1 - min);
            }
            if matches!(h, Handle::E | Handle::NE | Handle::SE) {
                x1 = (x1 + dx).clamp(x0 + min, 1.0);
            }
            if matches!(h, Handle::N | Handle::NW | Handle::NE) {
                y0 = (y0 + dy).clamp(0.0, y1 - min);
            }
            if matches!(h, Handle::S | Handle::SW | Handle::SE) {
                y1 = (y1 + dy).clamp(y0 + min, 1.0);
            }
        }
    }
    let mut out = Rect { x: x0, y: y0, w: x1 - x0, h: y1 - y0 };
    if let Some(a) = aspect {
        // Convert the locked pixel aspect to normalised units.
        let na = a * frame.1 / frame.0;
        let (cx, cy) = out.center();
        match h {
            Handle::N | Handle::S => {
                let w = (out.h * na).min(1.0);
                out.w = w;
                out.h = w / na;
                out.x = (cx - w * 0.5).clamp(0.0, 1.0 - w);
            }
            Handle::E | Handle::W => {
                let hh = (out.w / na).min(1.0);
                out.h = hh;
                out.w = hh * na;
                out.y = (cy - hh * 0.5).clamp(0.0, 1.0 - hh);
            }
            _ => {
                // Corners: follow the larger change, anchored at the opposite corner.
                let w = out.w.max(out.h * na).min(1.0).min(na);
                let hh = w / na;
                let (ax, ay) = match h {
                    Handle::NW => (r.x + r.w, r.y + r.h),
                    Handle::NE => (r.x, r.y + r.h),
                    Handle::SW => (r.x + r.w, r.y),
                    _ => (r.x, r.y),
                };
                let x = if matches!(h, Handle::NW | Handle::SW) { ax - w } else { ax };
                let y = if matches!(h, Handle::NW | Handle::NE) { ay - hh } else { ay };
                out = Rect { x, y, w, h: hh };
                if out.x < 0.0 || out.y < 0.0 || out.x + out.w > 1.0 || out.y + out.h > 1.0 {
                    return r;
                }
            }
        }
    }
    out
}

/// Crop frame drawn over the photo while the crop tool is open.
#[component]
pub fn CropOverlay() -> impl IntoView {
    let ed = use_editor();
    let drag = StoredValue::new(None::<CropDrag>);
    let active = RwSignal::new(false);
    let rect = Memo::new(move |_| image_rect(&ed));

    let begin = move |h: Handle, ev: PointerEvent| {
        ev.stop_propagation();
        ev.prevent_default();
        if let Some(t) = ev.current_target() {
            use wasm_bindgen::JsCast;
            if let Ok(el) = t.dyn_into::<web_sys::Element>() {
                let _ = el.set_pointer_capture(ev.pointer_id());
            }
        }
        let r = rect.get_untracked();
        drag.set_value(Some(CropDrag {
            handle: h,
            start: (ev.client_x() as f32, ev.client_y() as f32),
            rect: ed.adj.with_untracked(|a| a.geometry.crop),
            view_px: (r[2], r[3]),
        }));
        active.set(true);
    };

    let on_move = move |ev: PointerEvent| {
        let Some(d) = drag.get_value() else { return };
        let dx = (ev.client_x() as f32 - d.start.0) / d.view_px.0.max(1.0);
        let dy = (ev.client_y() as f32 - d.start.1) / d.view_px.1.max(1.0);
        let (w, h) = source_size(&ed);
        let mut g = ed.adj.with_untracked(|a| a.geometry);
        let frame = geometry::frame_size(&g, w, h);
        // Largest fraction of the drag that keeps the crop on real pixels.
        let try_t = |t: f32, g: &mut Geometry| {
            g.crop = adjust(d.rect, d.handle, dx * t, dy * t, g.aspect, frame);
            geometry::crop_fits(g, w, h)
        };
        if !try_t(1.0, &mut g) {
            let (mut lo, mut hi) = (0.0f32, 1.0f32);
            for _ in 0..16 {
                let m = (lo + hi) * 0.5;
                if try_t(m, &mut g) {
                    lo = m;
                } else {
                    hi = m;
                }
            }
            try_t(lo, &mut g);
        }
        ed.adj.update(|a| a.geometry.crop = g.crop);
        request_render(ed);
    };

    let end = move |_ev: PointerEvent| {
        if drag.get_value().is_some() {
            drag.set_value(None);
            active.set(false);
            ed.commit();
        }
    };

    let crop_style = move || {
        let c = ed.adj.with(|a| a.geometry.crop);
        format!("left:{}%;top:{}%;width:{}%;height:{}%", c.x * 100.0, c.y * 100.0, c.w * 100.0, c.h * 100.0)
    };
    let frame_style = move || {
        let r = rect.get();
        format!("left:{}px;top:{}px;width:{}px;height:{}px", r[0], r[1], r[2], r[3])
    };

    let handle = move |h: Handle, class: &'static str| {
        view! { <div class=format!("crop-handle {class}") on:pointerdown=move |ev| begin(h, ev)></div> }
    };

    view! {
        <div class="crop-layer" style=frame_style on:pointermove=on_move on:pointerup=end on:pointercancel=end>
            <div class="crop-box" class:active=move || active.get() style=crop_style on:pointerdown=move |ev| begin(Handle::Move, ev)>
                <div class="crop-grid"></div>
                {handle(Handle::N, "n")}
                {handle(Handle::S, "s")}
                {handle(Handle::E, "e")}
                {handle(Handle::W, "w")}
                {handle(Handle::NW, "nw corner")}
                {handle(Handle::NE, "ne corner")}
                {handle(Handle::SW, "sw corner")}
                {handle(Handle::SE, "se corner")}
            </div>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn corner_drag_keeps_aspect() {
        let r = Rect { x: 0.1, y: 0.1, w: 0.5, h: 0.5 };
        let out = adjust(r, Handle::SE, 0.2, 0.05, Some(1.0), (100.0, 100.0));
        assert!((out.w - out.h).abs() < 1e-5);
        assert_eq!((out.x, out.y), (0.1, 0.1));
    }

    #[test]
    fn move_stays_inside() {
        let r = Rect { x: 0.5, y: 0.5, w: 0.4, h: 0.4 };
        let out = adjust(r, Handle::Move, 0.5, 0.5, None, (1.0, 1.0));
        assert!((out.x + out.w) <= 1.0 + 1e-6 && (out.y + out.h) <= 1.0 + 1e-6);
    }
}
