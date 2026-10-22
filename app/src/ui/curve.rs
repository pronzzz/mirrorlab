//! Tone curve editor.

use leptos::prelude::*;
use mirror_core::curve::{self, Spline};
use mirror_core::model::{Adjustments, CurveChannel, Curves, Point};
use web_sys::PointerEvent;

use crate::state::use_editor;
use crate::ui::panels::Section;

fn curves_changed(a: &Adjustments) -> bool {
    !a.curves.is_identity()
}
fn reset_curves(a: &mut Adjustments) {
    a.curves = Curves::default();
}

fn channel_color(c: CurveChannel) -> &'static str {
    match c {
        CurveChannel::Master => "#f2f2f2",
        CurveChannel::Red => "#ff6369",
        CurveChannel::Green => "#5bd47a",
        CurveChannel::Blue => "#5b8cff",
    }
}

#[component]
pub fn CurvePanel() -> impl IntoView {
    let ed = use_editor();
    let channel = ed.curve_channel;
    let svg = NodeRef::<leptos::svg::Svg>::new();
    let active = RwSignal::new(None::<usize>);
    let dragging = StoredValue::new(false);

    let points = move || ed.adj.with(|a| a.curves.get(channel.get()).clone());

    let to_norm = move |ev: &PointerEvent| -> Option<(f32, f32, f32)> {
        let el = svg.get()?;
        let r = el.get_bounding_client_rect();
        let x = ((ev.client_x() as f64 - r.left()) / r.width()) as f32;
        let y = (1.0 - (ev.client_y() as f64 - r.top()) / r.height()) as f32;
        Some((x, y, r.width() as f32))
    };

    let on_down = move |ev: PointerEvent| {
        let Some((x, y, w)) = to_norm(&ev) else { return };
        ev.prevent_default();
        if let Some(el) = svg.get() {
            let _ = el.set_pointer_capture(ev.pointer_id());
        }
        let ch = channel.get_untracked();
        let pts = ed.adj.with_untracked(|a| a.curves.get(ch).clone());
        let hit = 16.0 / w.max(1.0);
        let nearest = pts
            .iter()
            .enumerate()
            .map(|(i, p)| (i, ((p.x - x).powi(2) + (p.y - y).powi(2)).sqrt()))
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .filter(|(_, d)| *d < hit * 1.6)
            .map(|(i, _)| i);
        let idx = match nearest {
            Some(i) => i,
            None => {
                let mut idx = 0;
                // New points land on the curve; dragging then shapes it.
                let on_curve = Spline::new(&pts).eval(x.clamp(0.0, 1.0));
                ed.adj.update(|a| idx = curve::insert_point(a.curves.get_mut(ch), Point::new(x, on_curve)));
                idx
            }
        };
        active.set(Some(idx));
        dragging.set_value(true);
    };

    let on_move = move |ev: PointerEvent| {
        if !dragging.get_value() {
            return;
        }
        let (Some(i), Some((x, y, _))) = (active.get_untracked(), to_norm(&ev)) else { return };
        let ch = channel.get_untracked();
        ed.adj.update(|a| curve::move_point(a.curves.get_mut(ch), i, Point::new(x, y)));
        // Dragging an inner point well outside the graph removes it.
        let outside = !(-0.12..=1.12).contains(&y);
        let len = ed.adj.with_untracked(|a| a.curves.get(ch).len());
        if outside && i > 0 && i + 1 < len {
            ed.adj.update(|a| {
                curve::remove_point(a.curves.get_mut(ch), i);
            });
            active.set(None);
            dragging.set_value(false);
            ed.commit();
        }
    };

    let on_up = move |_ev: PointerEvent| {
        if dragging.get_value() {
            dragging.set_value(false);
            ed.commit();
        }
    };

    let path = move || {
        let s = Spline::new(&points());
        let mut d = String::with_capacity(2048);
        for i in 0..=128 {
            let x = i as f32 / 128.0;
            let y = s.eval(x);
            d.push_str(&format!("{}{:.2},{:.2}", if i == 0 { "M" } else { "L" }, x * 100.0, (1.0 - y) * 100.0));
        }
        d
    };

    let hist_path = move || {
        let h = ed.hist.get();
        if h.len() < 1024 {
            return String::new();
        }
        let c = match channel.get() {
            CurveChannel::Master => 3,
            CurveChannel::Red => 0,
            CurveChannel::Green => 1,
            CurveChannel::Blue => 2,
        };
        let mut d = String::from("M0,100");
        for i in (0..256).step_by(2) {
            d.push_str(&format!(" L{:.2},{:.2}", i as f32 / 255.0 * 100.0, 100.0 - h[c * 256 + i] * 92.0));
        }
        d.push_str(" L100,100 Z");
        d
    };

    let readout = move || {
        let pts = points();
        match active.get().and_then(|i| pts.get(i).copied()) {
            Some(p) => format!("In {:.0}  ·  Out {:.0}", p.x * 255.0, p.y * 255.0),
            None => "Tap the curve to add a point".into(),
        }
    };

    view! {
        <div class="panel">
            <Section title="Tone curve" reset=reset_curves changed=curves_changed>
                <div class="segmented compact" role="tablist">
                    {CurveChannel::ALL
                        .into_iter()
                        .map(|c| {
                            let edited = move || ed.adj.with(|a| !curve::is_identity(a.curves.get(c)));
                            view! {
                                <button
                                    class="seg"
                                    class:selected=move || channel.get() == c
                                    on:click=move |_| {
                                        channel.set(c);
                                        active.set(None);
                                    }
                                >
                                    <span class="seg-dot" class:on=edited style=format!("background:{}", channel_color(c))></span>
                                    {c.label()}
                                </button>
                            }
                        })
                        .collect_view()}
                </div>
                <div class="curve-box">
                    <svg
                        class="curve"
                        node_ref=svg
                        viewBox="-2 -2 104 104"
                        preserveAspectRatio="none"
                        on:pointerdown=on_down
                        on:pointermove=on_move
                        on:pointerup=on_up
                        on:pointercancel=on_up
                        on:dblclick=move |_| {
                            if let Some(i) = active.get_untracked() {
                                let ch = channel.get_untracked();
                                ed.apply(|a| {
                                    curve::remove_point(a.curves.get_mut(ch), i);
                                });
                                active.set(None);
                            }
                        }
                    >
                        <path class="curve-hist" d=hist_path></path>
                        <path class="curve-grid" d="M25,0V100M50,0V100M75,0V100M0,25H100M0,50H100M0,75H100"></path>
                        <rect class="curve-frame" x="0" y="0" width="100" height="100" rx="1.5"></rect>
                        <path class="curve-diag" d="M0,100L100,0"></path>
                        <path class="curve-line" d=path style=move || format!("stroke:{}", channel_color(channel.get()))></path>
                        {move || {
                            let a = active.get();
                            points()
                                .into_iter()
                                .enumerate()
                                .map(|(i, p)| {
                                    view! {
                                        <circle
                                            class="curve-pt"
                                            class:on=a == Some(i)
                                            cx=p.x * 100.0
                                            cy=(1.0 - p.y) * 100.0
                                            r=if a == Some(i) { 2.6 } else { 2.0 }
                                        ></circle>
                                    }
                                })
                                .collect_view()
                        }}
                    </svg>
                </div>
                <div class="curve-readout">
                    <span>{readout}</span>
                    <button
                        class="link"
                        on:click=move |_| {
                            let ch = channel.get_untracked();
                            ed.apply(|a| *a.curves.get_mut(ch) = Curves::default().master);
                            active.set(None);
                        }
                    >
                        "Reset channel"
                    </button>
                </div>
            </Section>
        </div>
    }
}
