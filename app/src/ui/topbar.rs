//! Top bar.

use leptos::prelude::*;

use crate::icons::{Icon, I};
use crate::platform;
use crate::state::use_editor;
use crate::view::request_render;

#[component]
pub fn TopBar(#[prop(into)] on_open: Callback<()>) -> impl IntoView {
    let ed = use_editor();
    let mod_key = if platform::is_apple() { "⌘" } else { "Ctrl+" };
    let title = move || ed.photo_name().unwrap_or_default();

    view! {
        <header class="topbar">
            <div class="topbar-left">
                <div class="brand" title="MirrorLab">
                    <Icon icon=I::Logo size=22 />
                    <span class="wordmark">"MirrorLab"</span>
                </div>
                <button
                    class="icon-btn ghost desktop-only"
                    class:on=move || ed.presets_open.get()
                    title="Show looks"
                    on:click=move |_| ed.presets_open.update(|v| *v = !*v)
                >
                    <Icon icon=I::Sidebar />
                </button>
            </div>
            <div class="topbar-title">
                <span class="file-name">{title}</span>
            </div>
            <div class="topbar-right">
                <div class="group">
                    <button
                        class="icon-btn ghost"
                        title=format!("Undo ({mod_key}Z)")
                        disabled=move || !ed.can_undo.get()
                        on:click=move |_| ed.undo()
                    >
                        <Icon icon=I::Undo />
                    </button>
                    <button
                        class="icon-btn ghost"
                        title=format!("Redo ({mod_key}⇧Z)")
                        disabled=move || !ed.can_redo.get()
                        on:click=move |_| ed.redo()
                    >
                        <Icon icon=I::Redo />
                    </button>
                </div>
                <button
                    class="icon-btn ghost"
                    class:on=move || ed.compare.get()
                    title="Before / after (hold \\ for original)"
                    disabled=move || !ed.has_photo()
                    on:click=move |_| {
                        ed.compare.update(|c| *c = !*c);
                        request_render(ed);
                    }
                >
                    <Icon icon=I::Compare />
                </button>
                <button class="icon-btn ghost" title=format!("Open photos ({mod_key}O)") on:click=move |_| on_open.run(())>
                    <Icon icon=I::Open />
                </button>
                <button
                    class="btn primary export-btn"
                    title=format!("Export ({mod_key}E)")
                    disabled=move || !ed.has_photo()
                    on:click=move |_| ed.export_open.set(true)
                >
                    <Icon icon=I::Export size=18 />
                    <span class="label">"Export"</span>
                </button>
            </div>
        </header>
    }
}
