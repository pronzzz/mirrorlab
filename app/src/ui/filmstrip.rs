//! Strip of open photos, with copy/paste of looks between them.

use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::icons::{Icon, I};
use crate::io;
use crate::state::use_editor;

#[component]
pub fn Filmstrip(#[prop(into)] on_add: Callback<()>) -> impl IntoView {
    let ed = use_editor();
    let has_clip = RwSignal::new(false);

    let copy = move |_| {
        ed.clipboard.set_value(Some(ed.adj.get_untracked()));
        has_clip.set(true);
        ed.info("Look copied");
    };
    let paste = move |_| {
        if let (Some(look), Some(id)) = (ed.clipboard.get_value(), ed.current.get_untracked()) {
            ed.set_look_of(id, &look);
            ed.info("Look pasted");
        }
    };
    let paste_all = move |_| {
        let Some(look) = ed.clipboard.get_value() else { return };
        let ids: Vec<u32> = ed.photos.with_untracked(|p| p.iter().map(|p| p.id).collect());
        for id in &ids {
            ed.set_look_of(*id, &look);
        }
        ed.info(format!("Look pasted to {} photos", ids.len()));
    };

    view! {
        <div class="filmstrip">
            <div class="film-actions">
                <button class="icon-btn small ghost" title="Copy look" on:click=copy>
                    <Icon icon=I::Copy size=17 />
                </button>
                <button class="icon-btn small ghost" title="Paste look" disabled=move || !has_clip.get() on:click=paste>
                    <Icon icon=I::Paste size=17 />
                </button>
                <button class="chip small" title="Paste look to every photo" disabled=move || !has_clip.get() on:click=paste_all>
                    "Paste to all"
                </button>
            </div>
            <div class="film-scroll">
                <For
                    each=move || ed.photos.get()
                    key=|p| (p.id, p.thumb.clone(), p.edited)
                    children=move |p| {
                        let id = p.id;
                        view! {
                            <div class="film" class:selected=move || ed.current.get() == Some(id) title=p.name.clone()>
                                <button class="film-btn" on:click=move |_| spawn_local(io::load_photo(ed, id))>
                                    {match p.thumb.clone() {
                                        Some(src) => view! { <img src=src alt="" draggable="false" /> }.into_any(),
                                        None => view! { <div class="film-ph"><Icon icon=I::Image size=18 /></div> }.into_any(),
                                    }}
                                    {p.edited.then(|| view! { <span class="film-dot"></span> })}
                                </button>
                                <button
                                    class="film-x"
                                    title="Remove from strip"
                                    on:click=move |_| {
                                        if ed.current.get_untracked() != Some(id) {
                                            ed.remove_photo(id);
                                        } else if let Some(next) = ed.photos.with_untracked(|ps| ps.iter().map(|p| p.id).find(|x| *x != id)) {
                                            spawn_local(async move {
                                                io::load_photo(ed, next).await;
                                                ed.remove_photo(id);
                                            });
                                        }
                                    }
                                >
                                    <Icon icon=I::Close size=12 />
                                </button>
                            </div>
                        }
                    }
                />
                <button class="film add" title="Add photos" on:click=move |_| on_add.run(())>
                    <Icon icon=I::Plus size=20 />
                </button>
            </div>
        </div>
    }
}
