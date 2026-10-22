//! Looks (presets) with live thumbnails of the current photo.

use leptos::html;
use leptos::prelude::*;
use leptos::task::spawn_local;
use mirror_core::model::Adjustments;
use mirror_core::presets::Preset;

use crate::icons::{Icon, I};
use crate::io;
use crate::state::{use_editor, Editor};
use crate::view::tween;

/// Apply a look with a short glide so the change reads as motion, not a cut.
pub fn apply_look(ed: Editor, look: Adjustments) {
    let from = ed.adj.get_untracked();
    let to = from.with_look(&look);
    if from == to {
        return;
    }
    tween(
        0.0,
        1.0,
        220.0,
        move |t| {
            let mut a = to.clone();
            for p in mirror_core::model::Param::ALL {
                p.set(&mut a, p.get(&from) + (p.get(&to) - p.get(&from)) * t);
            }
            for i in 0..8 {
                let (x, y) = (from.hsl[i], to.hsl[i]);
                a.hsl[i].hue = x.hue + (y.hue - x.hue) * t;
                a.hsl[i].saturation = x.saturation + (y.saturation - x.saturation) * t;
                a.hsl[i].luminance = x.luminance + (y.luminance - x.luminance) * t;
            }
            ed.adj.set(a);
        },
        move || ed.commit(),
    );
}

#[component]
fn LookTile(id: String, name: String, look: Adjustments, custom: bool) -> impl IntoView {
    let ed = use_editor();
    let thumb_id = id.clone();
    let thumb = move || ed.thumbs.with(|t| t.get(&thumb_id).cloned());
    let look2 = look.clone();
    let selected = move || ed.adj.with(|a| *a == a.with_look(&look2));
    let look3 = look.clone();
    let id2 = id.clone();
    let name2 = name.clone();
    let look4 = look.clone();
    view! {
        <div class="look" class:selected=selected>
            <button class="look-btn" on:click=move |_| apply_look(ed, look3.clone()) title=name.clone()>
                <div class="look-thumb">
                    {move || match thumb() {
                        Some(src) => view! { <img src=src alt="" draggable="false" /> }.into_any(),
                        None => view! { <div class="shimmer"></div> }.into_any(),
                    }}
                </div>
                <span class="look-name">{name.clone()}</span>
            </button>
            {custom.then(|| {
                let id3 = id2.clone();
                let name3 = name2.clone();
                let look5 = look4.clone();
                view! {
                    <div class="look-actions">
                        <button
                            class="icon-btn tiny"
                            title="Save as file"
                            on:click=move |_| {
                                let p = Preset::custom(&name3, &look5);
                                spawn_local(io::export_preset(ed, p));
                            }
                        >
                            <Icon icon=I::Download size=14 />
                        </button>
                        <button class="icon-btn tiny" title="Delete look" on:click=move |_| ed.delete_preset(&id3)>
                            <Icon icon=I::Trash size=14 />
                        </button>
                    </div>
                }
            })}
        </div>
    }
}

#[component]
pub fn PresetsPanel() -> impl IntoView {
    let ed = use_editor();
    let naming = RwSignal::new(false);
    let name = RwSignal::new(String::new());
    let file_input = NodeRef::<html::Input>::new();

    let save = move || {
        let n = name.get_untracked();
        let n = n.trim();
        if n.is_empty() {
            return;
        }
        let p = Preset::custom(n, &ed.adj.get_untracked());
        ed.add_preset(p);
        ed.success(format!("Saved look “{n}”"));
        naming.set(false);
        name.set(String::new());
        io::refresh_preset_thumbs(ed, true);
    };

    view! {
        <div class="panel presets-panel">
            <header class="section-head">
                <h3>"Looks"</h3>
                <div class="head-actions">
                    <button class="icon-btn small ghost" title="Import looks (.mlpreset)" on:click=move |_| {
                        if let Some(i) = file_input.get() {
                            i.click();
                        }
                    }>
                        <Icon icon=I::Upload size=16 />
                    </button>
                    <button class="icon-btn small ghost" class:on=move || naming.get() title="Save current look" on:click=move |_| naming.update(|n| *n = !*n)>
                        <Icon icon=I::Plus size=16 />
                    </button>
                </div>
                <input
                    type="file"
                    class="hidden-input"
                    accept=".mlpreset,.json,application/json"
                    multiple=true
                    node_ref=file_input
                    on:change=move |ev| {
                        let input = event_target::<web_sys::HtmlInputElement>(&ev);
                        let files = io::file_list(input.files());
                        input.set_value("");
                        spawn_local(io::import_presets(ed, files));
                    }
                />
            </header>
            <div class="reveal" class:open=move || naming.get()>
                <div class="reveal-inner">
                    <form class="name-form" on:submit=move |ev| {
                        ev.prevent_default();
                        save();
                    }>
                        <input
                            class="text-input"
                            placeholder="Name this look"
                            maxlength="40"
                            prop:value=move || name.get()
                            on:input=move |ev| name.set(event_target_value(&ev))
                            on:keydown=move |ev: web_sys::KeyboardEvent| ev.stop_propagation()
                        />
                        <button class="btn primary small" type="submit" disabled=move || name.with(|n| n.trim().is_empty())>
                            "Save"
                        </button>
                    </form>
                </div>
            </div>
            <div class="looks">
                <LookTile id="original".into() name="Original".into() look=Adjustments::default() custom=false />
                <For
                    each=move || ed.presets.get()
                    key=|p| p.id.clone()
                    children=move |p| view! { <LookTile id=p.id.clone() name=p.name.clone() look=p.look.clone() custom=!p.builtin /> }
                />
            </div>
        </div>
    }
}
