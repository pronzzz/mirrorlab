//! Tool switching and the panel host (desktop sidebar and mobile sheet).

use leptos::prelude::*;
use mirror_core::model::Adjustments;

use crate::icons::{Icon, I};
use crate::io;
use crate::state::{use_editor, Tool};
use crate::ui::crop::CropPanel;
use crate::ui::curve::CurvePanel;
use crate::ui::histogram::HistogramView;
use crate::ui::panels::{reset_with, ColorPanel, EffectsPanel, GradePanel, LightPanel, MixerPanel};
use crate::ui::presets::PresetsPanel;
use crate::view::request_render;

fn reset_all(a: &mut Adjustments) {
    *a = a.with_look(&Adjustments::default());
}

/// Whether the tool's settings differ from default (drives the little dot on its tab).
fn tool_edited(t: Tool, a: &Adjustments) -> bool {
    use mirror_core::model::Param::*;
    let d = Adjustments::default();
    match t {
        Tool::Presets => false,
        Tool::Light => [Exposure, Contrast, Highlights, Shadows, Whites, Blacks].iter().any(|p| p.get(a) != p.get(&d)),
        Tool::Color => [Temperature, Tint, Vibrance, Saturation].iter().any(|p| p.get(a) != p.get(&d)),
        Tool::Curve => !a.curves.is_identity(),
        Tool::Mixer => a.hsl.iter().any(|h| !h.is_zero()),
        Tool::Grade => a.grading != d.grading,
        Tool::Effects => [Clarity, Dehaze, Sharpen, Vignette, Grain].iter().any(|p| p.get(a) != p.get(&d)),
        Tool::Crop => !a.geometry.is_identity(),
    }
}

pub fn panel_for(t: Tool) -> AnyView {
    match t {
        Tool::Presets => view! { <PresetsPanel /> }.into_any(),
        Tool::Light => view! { <LightPanel /> }.into_any(),
        Tool::Color => view! { <ColorPanel /> }.into_any(),
        Tool::Curve => view! { <CurvePanel /> }.into_any(),
        Tool::Mixer => view! { <MixerPanel /> }.into_any(),
        Tool::Grade => view! { <GradePanel /> }.into_any(),
        Tool::Effects => view! { <EffectsPanel /> }.into_any(),
        Tool::Crop => view! { <CropPanel /> }.into_any(),
    }
}

/// Desktop right column: histogram, tool tabs, active panel.
#[component]
pub fn EditPanel() -> impl IntoView {
    let ed = use_editor();
    let index = move || Tool::EDIT.iter().position(|t| *t == ed.tool.get()).unwrap_or(0);
    view! {
        <aside class="side side-right">
            <HistogramView />
            <nav class="tabs" role="tablist" style=move || format!("--i:{};--n:{}", index(), Tool::EDIT.len())>
                {Tool::EDIT
                    .into_iter()
                    .map(|t| {
                        view! {
                            <button
                                class="tab"
                                role="tab"
                                class:selected=move || ed.tool.get() == t
                                class:edited=move || ed.adj.with(|a| tool_edited(t, a))
                                title=t.label()
                                aria-label=t.label()
                                on:click=move |_| {
                                    ed.tool.set(t);
                                    request_render(ed);
                                }
                            >
                                <Icon icon=t.icon() />
                            </button>
                        }
                    })
                    .collect_view()}
                <span class="tab-indicator"></span>
            </nav>
            <div class="panel-scroll">
                {move || {
                    let t = ed.tool.get();
                    let t = if t == Tool::Presets { Tool::Light } else { t };
                    view! { <div class="panel-host" data-tool=t.key()>{panel_for(t)}</div> }
                }}
            </div>
            <footer class="side-foot">
                <button
                    class="btn ghost small"
                    disabled=move || ed.adj.with(|a| a.is_neutral())
                    on:click=move |_| reset_with(ed, reset_all)
                >
                    <Icon icon=I::Reset size=16 />
                    "Reset edits"
                </button>
            </footer>
        </aside>
    }
}

/// Desktop left column: looks.
#[component]
pub fn LooksColumn() -> impl IntoView {
    let ed = use_editor();
    // Re-render thumbnails when the column opens, the photo changes or the crop does
    // (the refresh is skipped when nothing relevant changed).
    Effect::new(move |_| {
        ed.tool.track();
        ed.current.track();
        if ed.presets_open.get() && ed.has_photo() {
            io::refresh_preset_thumbs(ed, false);
        }
    });
    view! {
        <aside class="side side-left">
            <div class="panel-scroll">
                <PresetsPanel />
            </div>
        </aside>
    }
}

/// Mobile: bottom sheet with the active tool, and the tool bar under it.
#[component]
pub fn MobileTools() -> impl IntoView {
    let ed = use_editor();
    Effect::new(move |_| {
        if ed.tool.get() == Tool::Presets && ed.sheet_open.get() && ed.has_photo() {
            io::refresh_preset_thumbs(ed, false);
        }
    });
    view! {
        <div class="sheet" class:open=move || ed.sheet_open.get() && ed.has_photo()>
            <div class="sheet-inner">
                <div class="sheet-grip" on:click=move |_| ed.sheet_open.set(false)></div>
                <div class="panel-scroll">
                    {move || {
                        let t = ed.tool.get();
                        view! { <div class="panel-host" data-tool=t.key()>{panel_for(t)}</div> }
                    }}
                </div>
            </div>
        </div>
        <nav class="toolbar" class:disabled=move || !ed.has_photo()>
            {Tool::MOBILE
                .into_iter()
                .map(|t| {
                    view! {
                        <button
                            class="toolbar-btn"
                            class:selected=move || ed.tool.get() == t && ed.sheet_open.get()
                            class:edited=move || ed.adj.with(|a| tool_edited(t, a))
                            disabled=move || !ed.has_photo()
                            on:click=move |_| {
                                if ed.tool.get_untracked() == t && ed.sheet_open.get_untracked() {
                                    ed.sheet_open.set(false);
                                } else {
                                    ed.tool.set(t);
                                    ed.sheet_open.set(true);
                                }
                                request_render(ed);
                            }
                        >
                            <Icon icon=t.icon() size=22 />
                            <span>{t.label()}</span>
                        </button>
                    }
                })
                .collect_view()}
        </nav>
    }
}
