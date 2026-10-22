//! Viewport (zoom/pan with eased motion), the frame loop and the live histogram.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use leptos::prelude::*;
use mirror_core::geometry;
use mirror_core::histogram::Histogram;
use wasm_bindgen::prelude::*;

use crate::gl::Placement;
use crate::platform::{self, now_ms};
use crate::state::{Editor, Tool};

pub const MAX_ZOOM: f32 = 16.0;

#[derive(Clone, Copy)]
pub struct ViewCtl {
    /// Zoom relative to "fit" (1.0 = whole image visible).
    pub zoom: RwSignal<f32>,
    /// Point of the image (0..1) at the centre of the stage.
    pub center: RwSignal<(f32, f32)>,
    /// Where the eased motion is heading.
    target: StoredValue<(f32, f32, f32)>,
    /// Stage size in CSS pixels.
    pub stage: RwSignal<(f32, f32)>,
}

impl ViewCtl {
    pub fn new() -> Self {
        Self {
            zoom: RwSignal::new(1.0),
            center: RwSignal::new((0.5, 0.5)),
            target: StoredValue::new((1.0, 0.5, 0.5)),
            stage: RwSignal::new((0.0, 0.0)),
        }
    }
}

impl Default for ViewCtl {
    fn default() -> Self {
        Self::new()
    }
}

/// True while the crop tool is showing the whole frame.
pub fn crop_mode(ed: &Editor) -> bool {
    ed.tool.get() == Tool::Crop && (!ed.mobile.get() || ed.sheet_open.get())
}

/// Size (in texture pixels) of what the stage is showing.
pub fn display_size(ed: &Editor) -> (f32, f32) {
    let Some((w, h)) = ed.loaded.with(|l| l.as_ref().map(|l| (l.width as f32, l.height as f32))) else {
        return (0.0, 0.0);
    };
    let g = ed.adj.with(|a| a.geometry);
    let (fw, fh) = geometry::frame_size(&g, w, h);
    if crop_mode(ed) {
        (fw, fh)
    } else {
        (g.crop.w * fw, g.crop.h * fh)
    }
}

fn padding(ed: &Editor) -> f32 {
    match (ed.mobile.get(), crop_mode(ed)) {
        (true, true) => 22.0,
        (true, false) => 10.0,
        (false, true) => 48.0,
        (false, false) => 28.0,
    }
}

/// Scale that fits the image in the stage.
pub fn fit_scale(ed: &Editor) -> f32 {
    let (sw, sh) = ed.view.stage.get();
    let (ow, oh) = display_size(ed);
    if ow <= 0.0 || oh <= 0.0 || sw <= 0.0 {
        return 1.0;
    }
    let pad = padding(ed);
    (((sw - 2.0 * pad) / ow).min((sh - 2.0 * pad) / oh)).max(1e-4)
}

fn clamp_center(c: (f32, f32), dw: f32, dh: f32, sw: f32, sh: f32) -> (f32, f32) {
    let ax = |c: f32, d: f32, s: f32| if d <= s { 0.5 } else { c.clamp(s * 0.5 / d, 1.0 - s * 0.5 / d) };
    (ax(c.0, dw, sw), ax(c.1, dh, sh))
}

/// Image rectangle on the stage, in CSS pixels: `[x, y, w, h]`.
pub fn image_rect(ed: &Editor) -> [f32; 4] {
    let (sw, sh) = ed.view.stage.get();
    let (ow, oh) = display_size(ed);
    if ow <= 0.0 || sw <= 0.0 {
        return [0.0; 4];
    }
    let s = fit_scale(ed) * ed.view.zoom.get();
    let (dw, dh) = (ow * s, oh * s);
    let (cx, cy) = clamp_center(ed.view.center.get(), dw, dh, sw, sh);
    [sw * 0.5 - cx * dw, sh * 0.5 - cy * dh, dw, dh]
}

impl ViewCtl {
    /// Animate back to "fit".
    pub fn fit(&self, ed: &Editor) {
        self.target.set_value((1.0, 0.5, 0.5));
        request_render(*ed);
    }

    pub fn reset_now(&self) {
        self.target.set_value((1.0, 0.5, 0.5));
        self.zoom.set(1.0);
        self.center.set((0.5, 0.5));
    }

    /// Zoom so `anchor` (CSS px on the stage) stays over the same image point.
    pub fn zoom_at(&self, ed: &Editor, zoom: f32, anchor: Option<(f32, f32)>, animate: bool) {
        let zoom = zoom.clamp(1.0, MAX_ZOOM);
        let (sw, sh) = self.stage.get_untracked();
        let r = untrack(|| image_rect(ed));
        let (ax, ay) = anchor.unwrap_or((sw * 0.5, sh * 0.5));
        let u = ((ax - r[0]) / r[2]).clamp(0.0, 1.0);
        let v = ((ay - r[1]) / r[3]).clamp(0.0, 1.0);
        let ratio = zoom / self.zoom.get_untracked();
        let (dw, dh) = (r[2] * ratio, r[3] * ratio);
        let nx = ax - u * dw;
        let ny = ay - v * dh;
        let c = clamp_center(((sw * 0.5 - nx) / dw, (sh * 0.5 - ny) / dh), dw, dh, sw, sh);
        self.target.set_value((zoom, c.0, c.1));
        if !animate || platform::prefers_reduced_motion() {
            self.zoom.set(zoom);
            self.center.set(c);
        }
        request_render(*ed);
    }

    /// Zoom where one image pixel covers one device pixel.
    pub fn one_to_one_zoom(&self, ed: &Editor) -> f32 {
        let fit = untrack(|| fit_scale(ed));
        ((1.0 / platform::dpr()) / fit).clamp(1.0, MAX_ZOOM)
    }

    pub fn pan_by(&self, ed: &Editor, dx: f32, dy: f32) {
        let r = untrack(|| image_rect(ed));
        if r[2] <= 0.0 {
            return;
        }
        let (sw, sh) = self.stage.get_untracked();
        let (cx, cy) = self.center.get_untracked();
        let c = clamp_center((cx - dx / r[2], cy - dy / r[3]), r[2], r[3], sw, sh);
        let z = self.zoom.get_untracked();
        self.center.set(c);
        self.target.set_value((z, c.0, c.1));
        request_render(*ed);
    }

    /// Advance the eased motion. Returns true while still moving.
    fn step(&self, dt: f64) -> bool {
        let (tz, tx, ty) = self.target.get_value();
        let z = self.zoom.get_untracked();
        let (cx, cy) = self.center.get_untracked();
        let close = (tz - z).abs() < 1e-3 * tz && (tx - cx).abs() < 1e-4 && (ty - cy).abs() < 1e-4;
        if close {
            if z != tz || (cx, cy) != (tx, ty) {
                self.zoom.set(tz);
                self.center.set((tx, ty));
            }
            return false;
        }
        // Critically damped-ish exponential approach (~110ms to settle).
        let k = (1.0 - (-dt / 55.0).exp()) as f32;
        // Zoom eases in log space so in and out feel symmetrical.
        let lz = z.ln() + (tz.ln() - z.ln()) * k;
        self.zoom.set(lz.exp());
        self.center.set((cx + (tx - cx) * k, cy + (ty - cy) * k));
        true
    }
}

// ---- frame loop --------------------------------------------------------

thread_local! {
    static PENDING: Cell<bool> = const { Cell::new(false) };
    static LAST_FRAME: Cell<f64> = const { Cell::new(0.0) };
    static GEN: Cell<u64> = const { Cell::new(0) };
    static HIST_GEN: Cell<u64> = const { Cell::new(u64::MAX) };
    static HIST_AT: Cell<f64> = const { Cell::new(0.0) };
    static HIST_TIMER: Cell<bool> = const { Cell::new(false) };
}

/// The adjustments or photo changed: the histogram is stale.
pub fn bump_generation() {
    GEN.with(|g| g.set(g.get().wrapping_add(1)));
}

/// Ask for a redraw on the next animation frame (coalesced).
pub fn request_render(ed: Editor) {
    if PENDING.with(|p| p.replace(true)) {
        return;
    }
    request_animation_frame(move || {
        PENDING.with(|p| p.set(false));
        let now = now_ms();
        let last = LAST_FRAME.with(|l| l.replace(now));
        let dt = if last == 0.0 { 16.0 } else { (now - last).clamp(1.0, 64.0) };
        let moving = ed.view.step(dt);
        draw(&ed);
        if moving {
            request_render(ed);
        } else {
            LAST_FRAME.with(|l| l.set(0.0));
        }
        schedule_histogram(ed);
    });
}

/// Draw immediately (used right after a canvas resize to avoid a blank frame).
pub fn draw(ed: &Editor) {
    let place = untrack(|| {
        let r = image_rect(ed);
        let d = platform::dpr();
        let crop = crop_mode(ed);
        let split = if ed.compare.get() && !crop { ed.split.get() } else { -1.0 };
        Placement {
            rect: [r[0] * d, r[1] * d, r[2] * d, r[3] * d],
            split,
            bypass: ed.hold_original.get(),
            uncropped: crop,
        }
    });
    let adj = ed.adj.get_untracked();
    ed.gl.update_value(|g| {
        if let Some(r) = g.as_mut() {
            r.render(&adj, &place);
        }
    });
}

fn schedule_histogram(ed: Editor) {
    let gen = GEN.with(|g| g.get());
    if HIST_GEN.with(|h| h.get()) == gen || HIST_TIMER.with(|t| t.get()) {
        return;
    }
    let wait = (90.0 - (now_ms() - HIST_AT.with(|h| h.get()))).max(0.0);
    HIST_TIMER.with(|t| t.set(true));
    set_timeout(
        move || {
            HIST_TIMER.with(|t| t.set(false));
            update_histogram(&ed);
        },
        std::time::Duration::from_millis(wait as u64),
    );
}

fn update_histogram(ed: &Editor) {
    let gen = GEN.with(|g| g.get());
    HIST_GEN.with(|h| h.set(gen));
    HIST_AT.with(|h| h.set(now_ms()));
    let adj = ed.adj.get_untracked();
    let hist = ed.gl.try_update_value(|g| {
        let r = g.as_mut()?;
        let (w, h) = r.output_size(&adj);
        if w == 0 {
            return None;
        }
        let s = (220.0 / w.max(h) as f32).min(1.0);
        let (tw, th) = (((w as f32 * s) as u32).max(1), ((h as f32 * s) as u32).max(1));
        Some(Histogram::from_rgba(&r.render_pixels(&adj, tw, th)))
    });
    if let Some(Some(h)) = hist {
        ed.hist.set(h.display().iter().flatten().copied().collect());
        ed.clip.set(h.clipping());
    }
}

// ---- small animation helper ---------------------------------------------

type FrameSlot = Rc<RefCell<Option<Closure<dyn FnMut(f64)>>>>;

fn ease_out_cubic(t: f64) -> f64 {
    1.0 - (1.0 - t).powi(3)
}

/// Animate a value from `from` to `to`, calling `on` every frame, then `done`.
pub fn tween(from: f32, to: f32, ms: f64, on: impl Fn(f32) + 'static, done: impl FnOnce() + 'static) {
    if platform::prefers_reduced_motion() || (from - to).abs() < f32::EPSILON {
        on(to);
        done();
        return;
    }
    let start = now_ms();
    let done = Rc::new(RefCell::new(Some(done)));
    let cb: FrameSlot = Rc::new(RefCell::new(None));
    let cb2 = cb.clone();
    *cb.borrow_mut() = Some(Closure::new(move |_t: f64| {
        let t = ((now_ms() - start) / ms).clamp(0.0, 1.0);
        on(from + (to - from) * ease_out_cubic(t) as f32);
        if t < 1.0 {
            if let Some(c) = cb2.borrow().as_ref() {
                let _ = platform::window().request_animation_frame(c.as_ref().unchecked_ref());
            }
        } else {
            if let Some(d) = done.borrow_mut().take() {
                d();
            }
            // Break the Rc cycle once finished.
            let cb3 = cb2.clone();
            set_timeout(move || drop(cb3.borrow_mut().take()), std::time::Duration::from_millis(0));
        }
    }));
    let first = cb.borrow();
    if let Some(c) = first.as_ref() {
        let _ = platform::window().request_animation_frame(c.as_ref().unchecked_ref());
    }
    drop(first);
}
