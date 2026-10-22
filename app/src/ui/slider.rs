//! The adjustment slider.
//!
//! * Mouse: click/drag anywhere on the track. Touch: drag relative to where
//!   the finger landed, so scrolling the panel never jumps a value.
//! * Double-click / double-tap animates back to the default.
//! * Click the number to type a value. Arrow keys nudge (Shift = ×10).
//! * One undo step per gesture.

use leptos::html;
use leptos::prelude::*;
use mirror_core::model::{Param, Track};
use web_sys::{KeyboardEvent, PointerEvent};

use crate::platform::now_ms;
use crate::state::use_editor;
use crate::view::tween;

#[derive(Clone, Copy, Default)]
struct Drag {
    active: bool,
    touch: bool,
    moved: bool,
    start_x: f32,
    start_v: f32,
    left: f32,
    width: f32,
    last_tap: f64,
}

pub fn format_value(v: f32, min: f32, decimals: u8) -> String {
    let s = format!("{:.*}", decimals as usize, v.abs());
    let zero = v.abs() < 0.5 * 10f32.powi(-(decimals as i32));
    if min < 0.0 && !zero {
        if v > 0.0 {
            format!("+{s}")
        } else {
            format!("−{s}")
        }
    } else if zero {
        format!("{:.*}", decimals as usize, 0.0)
    } else {
        s
    }
}

pub fn track_css(t: Track) -> Option<String> {
    match t {
        Track::Plain => None,
        Track::Temperature => Some("linear-gradient(90deg,#4a7fd6,#cfcac0 50%,#e0a23b)".into()),
        Track::Tint => Some("linear-gradient(90deg,#43a35a,#cfcac0 50%,#c254bd)".into()),
        Track::Saturation => Some("linear-gradient(90deg,#7b7b7f,#a59a91 45%,#d97a55 70%,#dd4d6a)".into()),
        Track::Hue => Some("linear-gradient(90deg,#e5484d,#ffc53d,#46a758,#12a594,#3e63dd,#8e4ec6,#e5484d)".into()),
    }
}

#[component]
pub fn Slider(
    #[prop(into)] label: String,
    #[prop(into)] value: Signal<f32>,
    #[prop(into)] set: Callback<f32>,
    min: f32,
    max: f32,
    #[prop(default = 0.0)] default: f32,
    #[prop(default = 1.0)] step: f32,
    #[prop(default = 0)] decimals: u8,
    #[prop(optional, into)] track: Option<Signal<String>>,
    #[prop(optional)] unit: &'static str,
) -> impl IntoView {
    let ed = use_editor();
    let track_ref = NodeRef::<html::Div>::new();
    let drag = StoredValue::new(Drag::default());
    let dragging = RwSignal::new(false);
    let editing = RwSignal::new(false);
    let range = max - min;

    let snap = move |v: f32| {
        let v = v.clamp(min, max);
        // A gentle detent at the default makes it easy to land on zero.
        if (v - default).abs() < range * 0.012 {
            return default;
        }
        ((v / step).round() * step).clamp(min, max)
    };
    let pct = move |v: f32| ((v - min) / range * 100.0).clamp(0.0, 100.0);
    let zero_pct = pct(default.clamp(min, max));

    let reset = move || {
        let from = value.get_untracked();
        tween(
            from,
            default,
            240.0,
            move |v| {
                // The panel may close mid-animation; then the callback is gone.
                set.try_run(v);
            },
            move || ed.commit(),
        );
    };

    let value_at = move |x: f32| {
        let d = drag.get_value();
        min + ((x - d.left) / d.width.max(1.0)).clamp(0.0, 1.0) * range
    };

    let on_down = move |ev: PointerEvent| {
        if ev.button() != 0 {
            return;
        }
        let Some(el) = track_ref.get() else { return };
        let r = el.get_bounding_client_rect();
        let touch = ev.pointer_type() != "mouse";
        let last_tap = drag.get_value().last_tap;
        drag.set_value(Drag {
            active: true,
            touch,
            moved: false,
            start_x: ev.client_x() as f32,
            start_v: value.get_untracked(),
            left: r.left() as f32,
            width: r.width() as f32,
            last_tap,
        });
        let _ = el.set_pointer_capture(ev.pointer_id());
        if !touch {
            ev.prevent_default();
            let _ = el.focus();
            dragging.set(true);
            set.run(snap(value_at(ev.client_x() as f32)));
        }
    };

    let on_move = move |ev: PointerEvent| {
        let mut d = drag.get_value();
        if !d.active {
            return;
        }
        let x = ev.client_x() as f32;
        if d.touch {
            let dx = x - d.start_x;
            if !d.moved && dx.abs() < 4.0 {
                return;
            }
            d.moved = true;
            drag.set_value(d);
            dragging.set(true);
            set.run(snap(d.start_v + dx / d.width.max(1.0) * range));
        } else {
            d.moved = true;
            drag.set_value(d);
            set.run(snap(value_at(x)));
        }
    };

    let finish = move |_ev: PointerEvent| {
        let mut d = drag.get_value();
        if !d.active {
            return;
        }
        d.active = false;
        dragging.set(false);
        if d.touch && !d.moved {
            // Double-tap resets.
            let now = now_ms();
            if now - d.last_tap < 320.0 {
                d.last_tap = 0.0;
                drag.set_value(d);
                reset();
                return;
            }
            d.last_tap = now;
            drag.set_value(d);
            return;
        }
        drag.set_value(d);
        ed.commit();
    };

    let on_key = move |ev: KeyboardEvent| {
        let base = (range / 200.0).max(step);
        let k = if ev.shift_key() { base * 10.0 } else { base };
        let v = value.get_untracked();
        let nv = match ev.key().as_str() {
            "ArrowRight" | "ArrowUp" => v + k,
            "ArrowLeft" | "ArrowDown" => v - k,
            "Home" => min,
            "End" => max,
            "Backspace" | "Delete" => default,
            _ => return,
        };
        ev.prevent_default();
        ev.stop_propagation();
        set.run(((nv / step).round() * step).clamp(min, max));
    };

    let label2 = label.clone();
    let text = move || format!("{}{unit}", format_value(value.get(), min, decimals));
    let modified = move || (value.get() - default).abs() > step * 0.5;

    let commit_text = move |s: String| {
        let s = s.replace('−', "-").replace('+', "");
        if let Ok(v) = s.trim().parse::<f32>() {
            set.run(v.clamp(min, max));
            ed.commit();
        }
        editing.set(false);
    };

    let rail_style = move || match track {
        Some(t) => format!("background:{}", t.get()),
        None => String::new(),
    };
    let has_track = track.is_some();

    view! {
        <div class="slider" class:active=move || dragging.get() class:modified=modified>
            <div class="slider-head">
                <span class="slider-label" on:dblclick=move |_| reset() title="Double-click to reset">{label.clone()}</span>
                <Show
                    when=move || editing.get()
                    fallback=move || view! {
                        <button class="slider-value" on:click=move |_| editing.set(true) title="Click to type a value">{text}</button>
                    }
                >
                    <input
                        class="slider-input"
                        type="text"
                        inputmode="decimal"
                        prop:value=move || format!("{:.*}", decimals as usize, value.get_untracked())
                        autofocus=true
                        on:keydown=move |ev: KeyboardEvent| {
                            ev.stop_propagation();
                            match ev.key().as_str() {
                                "Enter" => commit_text(event_target_value(&ev)),
                                "Escape" => editing.set(false),
                                _ => {}
                            }
                        }
                        on:blur=move |ev| commit_text(event_target_value(&ev))
                        on:focus=move |ev| {
                            event_target::<web_sys::HtmlInputElement>(&ev).select();
                        }
                    />
                </Show>
            </div>
            <div
                class="slider-track"
                class:gradient=has_track
                node_ref=track_ref
                tabindex="0"
                role="slider"
                aria-label=label2
                aria-valuemin=min
                aria-valuemax=max
                aria-valuenow=move || value.get()
                on:pointerdown=on_down
                on:pointermove=on_move
                on:pointerup=finish
                on:pointercancel=finish
                on:lostpointercapture=finish
                on:dblclick=move |_| reset()
                on:keydown=on_key
                on:keyup=move |ev: KeyboardEvent| {
                    if ev.key().starts_with("Arrow") || ev.key() == "Home" || ev.key() == "End" || ev.key() == "Backspace" || ev.key() == "Delete" {
                        ed.commit();
                    }
                }
            >
                <div class="slider-rail" style=rail_style></div>
                <div
                    class="slider-fill"
                    style=move || {
                        let p = pct(value.get());
                        let (a, b) = if p < zero_pct { (p, zero_pct) } else { (zero_pct, p) };
                        format!("left:{a}%;width:{}%", b - a)
                    }
                ></div>
                <div class="slider-zero" style=format!("left:{zero_pct}%")></div>
                <div class="slider-thumb" style=move || format!("left:{}%", pct(value.get()))></div>
            </div>
        </div>
    }
}

/// A slider wired straight to one of the scalar adjustments.
#[component]
pub fn ParamSlider(param: Param) -> impl IntoView {
    let ed = use_editor();
    let s = param.spec();
    let value = Signal::derive(move || ed.adj.with(|a| param.get(a)));
    let set = Callback::new(move |v: f32| ed.adj.update(|a| param.set(a, v)));
    let track = track_css(s.track).map(|t| Signal::derive(move || t.clone()));
    match track {
        Some(t) => view! {
            <Slider label=s.label value=value set=set min=s.min max=s.max default=s.default step=s.step decimals=s.decimals track=t />
        }
        .into_any(),
        None => view! {
            <Slider label=s.label value=value set=set min=s.min max=s.max default=s.default step=s.step decimals=s.decimals />
        }
        .into_any(),
    }
}

#[cfg(test)]
mod tests {
    use super::format_value;

    #[test]
    fn formatting() {
        assert_eq!(format_value(12.0, -100.0, 0), "+12");
        assert_eq!(format_value(-8.0, -100.0, 0), "−8");
        assert_eq!(format_value(0.0, -100.0, 0), "0");
        assert_eq!(format_value(-0.001, -5.0, 2), "0.00");
        assert_eq!(format_value(0.35, -5.0, 2), "+0.35");
        assert_eq!(format_value(40.0, 0.0, 0), "40");
    }
}
