//! Light, Color, Effects, Mixer and Grade panels.

use leptos::html;
use leptos::prelude::*;
use mirror_core::model::{Adjustments, Band, HslShift, Param, Wheel, Zone};
use web_sys::PointerEvent;

use crate::icons::{Icon, I};
use crate::state::{use_editor, Editor};
use crate::ui::slider::{ParamSlider, Slider};
use crate::view::tween;

/// Glide a group of settings back to their defaults as one undo step.
pub fn reset_with(ed: Editor, reset: fn(&mut Adjustments)) {
    let from = ed.adj.get_untracked();
    let mut to = from.clone();
    reset(&mut to);
    if from == to {
        return;
    }
    tween(
        0.0,
        1.0,
        240.0,
        move |t| {
            let mut a = to.clone();
            lerp_scalars(&from, &to, t, &mut a);
            ed.adj.set(a);
        },
        move || ed.commit(),
    );
}

fn lerp_scalars(from: &Adjustments, to: &Adjustments, t: f32, out: &mut Adjustments) {
    for p in Param::ALL {
        let v = p.get(from) + (p.get(to) - p.get(from)) * t;
        p.set(out, v);
    }
    for i in 0..8 {
        let (a, b) = (from.hsl[i], to.hsl[i]);
        out.hsl[i] = HslShift {
            hue: a.hue + (b.hue - a.hue) * t,
            saturation: a.saturation + (b.saturation - a.saturation) * t,
            luminance: a.luminance + (b.luminance - a.luminance) * t,
        };
    }
    for z in Zone::ALL {
        let (a, b) = (*from.grading.zone(z), *to.grading.zone(z));
        let w = out.grading.zone_mut(z);
        w.saturation = a.saturation + (b.saturation - a.saturation) * t;
        w.luminance = a.luminance + (b.luminance - a.luminance) * t;
        w.hue = if b.saturation == 0.0 { a.hue } else { b.hue };
    }
}

#[component]
pub fn Section(
    title: &'static str,
    #[prop(optional)] reset: Option<fn(&mut Adjustments)>,
    #[prop(optional)] changed: Option<fn(&Adjustments) -> bool>,
    children: Children,
) -> impl IntoView {
    let ed = use_editor();
    let dirty = move || changed.map(|f| ed.adj.with(f)).unwrap_or(false);
    view! {
        <section class="section">
            <header class="section-head">
                <h3>{title}</h3>
                {reset.map(|r| view! {
                    <button
                        class="icon-btn small ghost"
                        class:hidden=move || !dirty()
                        title=format!("Reset {}", title.to_lowercase())
                        on:click=move |_| reset_with(ed, r)
                    >
                        <Icon icon=I::Reset size=16 />
                    </button>
                })}
            </header>
            <div class="section-body">{children()}</div>
        </section>
    }
}

fn light_changed(a: &Adjustments) -> bool {
    [Param::Exposure, Param::Contrast, Param::Highlights, Param::Shadows, Param::Whites, Param::Blacks]
        .iter()
        .any(|p| p.get(a) != p.spec().default)
}

fn reset_light(a: &mut Adjustments) {
    for p in [Param::Exposure, Param::Contrast, Param::Highlights, Param::Shadows, Param::Whites, Param::Blacks] {
        p.set(a, p.spec().default);
    }
}

#[component]
pub fn LightPanel() -> impl IntoView {
    let ed = use_editor();
    view! {
        <div class="panel">
            <Section title="Light" reset=reset_light changed=light_changed>
                <button class="chip auto" on:click=move |_| crate::io::auto_light(ed) title="Balance exposure and tones automatically">
                    <Icon icon=I::Wand size=16 /> "Auto"
                </button>
                <ParamSlider param=Param::Exposure />
                <ParamSlider param=Param::Contrast />
                <ParamSlider param=Param::Highlights />
                <ParamSlider param=Param::Shadows />
                <ParamSlider param=Param::Whites />
                <ParamSlider param=Param::Blacks />
            </Section>
        </div>
    }
}

fn wb_changed(a: &Adjustments) -> bool {
    a.temperature != 0.0 || a.tint != 0.0
}
fn reset_wb(a: &mut Adjustments) {
    a.temperature = 0.0;
    a.tint = 0.0;
}
fn presence_changed(a: &Adjustments) -> bool {
    a.vibrance != 0.0 || a.saturation != 0.0
}
fn reset_presence(a: &mut Adjustments) {
    a.vibrance = 0.0;
    a.saturation = 0.0;
}

#[component]
pub fn ColorPanel() -> impl IntoView {
    view! {
        <div class="panel">
            <Section title="White balance" reset=reset_wb changed=wb_changed>
                <ParamSlider param=Param::Temperature />
                <ParamSlider param=Param::Tint />
            </Section>
            <Section title="Presence" reset=reset_presence changed=presence_changed>
                <ParamSlider param=Param::Vibrance />
                <ParamSlider param=Param::Saturation />
            </Section>
        </div>
    }
}

fn detail_changed(a: &Adjustments) -> bool {
    a.clarity != 0.0 || a.dehaze != 0.0 || a.sharpen != 0.0
}
fn reset_detail(a: &mut Adjustments) {
    a.clarity = 0.0;
    a.dehaze = 0.0;
    a.sharpen = 0.0;
}
fn vignette_changed(a: &Adjustments) -> bool {
    a.vignette != 0.0 || a.vignette_feather != 50.0
}
fn reset_vignette(a: &mut Adjustments) {
    a.vignette = 0.0;
    a.vignette_feather = 50.0;
}
fn grain_changed(a: &Adjustments) -> bool {
    a.grain != 0.0 || a.grain_size != 25.0
}
fn reset_grain(a: &mut Adjustments) {
    a.grain = 0.0;
    a.grain_size = 25.0;
}

#[component]
pub fn EffectsPanel() -> impl IntoView {
    view! {
        <div class="panel">
            <Section title="Detail" reset=reset_detail changed=detail_changed>
                <ParamSlider param=Param::Clarity />
                <ParamSlider param=Param::Dehaze />
                <ParamSlider param=Param::Sharpen />
            </Section>
            <Section title="Vignette" reset=reset_vignette changed=vignette_changed>
                <ParamSlider param=Param::Vignette />
                <ParamSlider param=Param::VignetteFeather />
            </Section>
            <Section title="Grain" reset=reset_grain changed=grain_changed>
                <ParamSlider param=Param::Grain />
                <ParamSlider param=Param::GrainSize />
            </Section>
        </div>
    }
}

// ---- Mixer -----------------------------------------------------------------

fn hsl_css(h: f32, s: f32, l: f32) -> String {
    format!("hsl({:.0},{:.0}%,{:.0}%)", h.rem_euclid(360.0), s, l)
}

fn mixer_changed(a: &Adjustments) -> bool {
    a.hsl.iter().any(|h| !h.is_zero())
}
fn reset_mixer(a: &mut Adjustments) {
    a.hsl = [HslShift::default(); 8];
}

#[component]
pub fn MixerPanel() -> impl IntoView {
    let ed = use_editor();
    let band = ed.band;
    let field = move |f: fn(&HslShift) -> f32| Signal::derive(move || ed.adj.with(|a| f(&a.hsl[band.get().index()])));
    let setter = move |f: fn(&mut HslShift, f32)| {
        Callback::new(move |v: f32| {
            let i = band.get_untracked().index();
            ed.adj.update(|a| f(&mut a.hsl[i], v));
        })
    };
    let center = move || Band::CENTERS[band.get().index()];
    let hue_track = Signal::derive(move || {
        let c = center();
        format!("linear-gradient(90deg,{},{},{})", hsl_css(c - 30.0, 75.0, 55.0), hsl_css(c, 75.0, 55.0), hsl_css(c + 30.0, 75.0, 55.0))
    });
    let sat_track = Signal::derive(move || format!("linear-gradient(90deg,{},{})", hsl_css(center(), 0.0, 50.0), hsl_css(center(), 85.0, 52.0)));
    let lum_track = Signal::derive(move || {
        format!("linear-gradient(90deg,{},{},{})", hsl_css(center(), 70.0, 14.0), hsl_css(center(), 70.0, 48.0), hsl_css(center(), 70.0, 86.0))
    });

    view! {
        <div class="panel">
            <Section title="Color mixer" reset=reset_mixer changed=mixer_changed>
                <div class="swatches" role="tablist">
                    {Band::ALL
                        .into_iter()
                        .map(|b| {
                            let edited = move || ed.adj.with(|a| !a.hsl[b.index()].is_zero());
                            view! {
                                <button
                                    class="swatch"
                                    role="tab"
                                    class:selected=move || band.get() == b
                                    class:edited=edited
                                    style=format!("--c:{}", b.swatch())
                                    title=b.label()
                                    aria-label=b.label()
                                    on:click=move |_| band.set(b)
                                ></button>
                            }
                        })
                        .collect_view()}
                </div>
                <div class="band-name">{move || band.get().label()}</div>
                <Slider label="Hue" value=field(|h| h.hue) set=setter(|h, v| h.hue = v) min=-100.0 max=100.0 track=hue_track />
                <Slider label="Saturation" value=field(|h| h.saturation) set=setter(|h, v| h.saturation = v) min=-100.0 max=100.0 track=sat_track />
                <Slider label="Luminance" value=field(|h| h.luminance) set=setter(|h, v| h.luminance = v) min=-100.0 max=100.0 track=lum_track />
            </Section>
        </div>
    }
}

// ---- Grade -----------------------------------------------------------------

fn grade_changed(a: &Adjustments) -> bool {
    !a.grading.is_neutral() || a.grading.balance != 0.0
}
fn reset_grade(a: &mut Adjustments) {
    a.grading = Default::default();
}

#[component]
fn ColorWheel(zone: RwSignal<Zone>) -> impl IntoView {
    let ed = use_editor();
    let el = NodeRef::<html::Div>::new();
    let active = RwSignal::new(false);
    let wheel = move || ed.adj.with(|a| *a.grading.zone(zone.get()));

    let set_from = move |x: f64, y: f64| {
        let Some(e) = el.get() else { return };
        let r = e.get_bounding_client_rect();
        let rad = r.width() / 2.0;
        let dx = x - (r.left() + rad);
        let dy = y - (r.top() + rad);
        let mut hue = (-dy).atan2(dx).to_degrees() as f32;
        if hue < 0.0 {
            hue += 360.0;
        }
        let mut sat = ((dx * dx + dy * dy).sqrt() / rad * 100.0).min(100.0) as f32;
        if sat < 3.0 {
            sat = 0.0;
        }
        let z = zone.get_untracked();
        ed.adj.update(|a| {
            let w = a.grading.zone_mut(z);
            w.hue = hue.round();
            w.saturation = sat.round();
        });
    };

    let puck = move || {
        let w: Wheel = wheel();
        let a = w.hue.to_radians();
        let r = w.saturation / 100.0 * 50.0;
        format!("left:{}%;top:{}%;--c:{}", 50.0 + r * a.cos(), 50.0 - r * a.sin(), hsl_css(w.hue, w.saturation * 0.9, 55.0))
    };

    view! {
        <div class="wheel-wrap">
            <div
                class="wheel"
                class:active=move || active.get()
                node_ref=el
                on:pointerdown=move |ev: PointerEvent| {
                    if let Some(e) = el.get() {
                        let _ = e.set_pointer_capture(ev.pointer_id());
                    }
                    active.set(true);
                    set_from(ev.client_x() as f64, ev.client_y() as f64);
                }
                on:pointermove=move |ev: PointerEvent| {
                    if active.get_untracked() {
                        set_from(ev.client_x() as f64, ev.client_y() as f64);
                    }
                }
                on:pointerup=move |_| {
                    active.set(false);
                    ed.commit();
                }
                on:pointercancel=move |_| {
                    active.set(false);
                    ed.commit();
                }
                on:dblclick=move |_| {
                    let z = zone.get_untracked();
                    ed.apply(|a| a.grading.zone_mut(z).saturation = 0.0);
                }
            >
                <div class="wheel-cross"></div>
                <div class="wheel-puck" style=puck></div>
            </div>
            <div class="wheel-readout">
                {move || {
                    let w = wheel();
                    if w.saturation == 0.0 { "Neutral".to_string() } else { format!("{:.0}° · {:.0}%", w.hue, w.saturation) }
                }}
            </div>
        </div>
    }
}

#[component]
pub fn GradePanel() -> impl IntoView {
    let ed = use_editor();
    let zone = ed.zone;
    let lum = Signal::derive(move || ed.adj.with(|a| a.grading.zone(zone.get()).luminance));
    let set_lum = Callback::new(move |v: f32| {
        let z = zone.get_untracked();
        ed.adj.update(|a| a.grading.zone_mut(z).luminance = v);
    });
    view! {
        <div class="panel">
            <Section title="Color grading" reset=reset_grade changed=grade_changed>
                <div class="segmented" role="tablist">
                    {Zone::ALL
                        .into_iter()
                        .map(|z| {
                            let tint = move || {
                                let w = ed.adj.with(|a| *a.grading.zone(z));
                                if w.saturation > 0.0 { hsl_css(w.hue, 70.0, 60.0) } else { "transparent".into() }
                            };
                            view! {
                                <button class="seg" class:selected=move || zone.get() == z on:click=move |_| zone.set(z)>
                                    <span class="seg-dot" style=move || format!("background:{}", tint())></span>
                                    {z.label()}
                                </button>
                            }
                        })
                        .collect_view()}
                </div>
                <ColorWheel zone=zone />
                <Slider label="Luminance" value=lum set=set_lum min=-100.0 max=100.0 />
                <ParamSlider param=Param::GradingBalance />
            </Section>
        </div>
    }
}
