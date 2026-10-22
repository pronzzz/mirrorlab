//! Root component: layout, global shortcuts, drag & drop, paste.

use leptos::ev;
use leptos::html;
use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen::JsCast;
use web_sys::KeyboardEvent;

use crate::icons::{Icon, I};
use crate::io;
use crate::platform;
use crate::state::{Editor, Tool};
use crate::ui::export::ExportDialog;
use crate::ui::filmstrip::Filmstrip;
use crate::ui::stage::Stage;
use crate::ui::toast::Toasts;
use crate::ui::tools::{EditPanel, LooksColumn, MobileTools};
use crate::ui::topbar::TopBar;
use crate::view::{bump_generation, request_render};

const MOBILE_BREAKPOINT: f64 = 820.0;

fn typing_in_field(ev: &web_sys::Event) -> bool {
    ev.target()
        .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
        .map(|el| matches!(el.tag_name().as_str(), "INPUT" | "TEXTAREA" | "SELECT") || el.closest("[contenteditable]").ok().flatten().is_some())
        .unwrap_or(false)
}

fn files_from_transfer(dt: Option<web_sys::DataTransfer>) -> Vec<web_sys::File> {
    io::file_list(dt.and_then(|d| d.files()))
}

#[component]
pub fn App() -> impl IntoView {
    let ed = Editor::new();
    provide_context(ed);
    #[cfg(debug_assertions)]
    crate::selftest::register(ed);

    let file_input = NodeRef::<html::Input>::new();
    let open = Callback::new(move |_: ()| {
        if let Some(i) = file_input.get_untracked() {
            i.click();
        }
    });

    // Layout mode.
    let update_mobile = move || {
        let w = platform::window().inner_width().ok().and_then(|v| v.as_f64()).unwrap_or(1200.0);
        let m = w < MOBILE_BREAKPOINT;
        if ed.mobile.get_untracked() != m {
            ed.mobile.set(m);
            if !m && ed.tool.get_untracked() == Tool::Presets {
                ed.tool.set(Tool::Light);
            }
        }
        if w < 1180.0 && !m {
            // Narrow desktop windows start with the looks column tucked away.
            ed.presets_open.set(false);
        }
    };
    update_mobile();
    let _resize = window_event_listener(ev::resize, move |_| update_mobile());

    // Redraw whenever anything the picture depends on changes.
    Effect::new(move |_| {
        ed.adj.track();
        ed.loaded.track();
        bump_generation();
        request_render(ed);
    });
    Effect::new(move |_| {
        ed.compare.track();
        ed.split.track();
        ed.hold_original.track();
        ed.tool.track();
        ed.sheet_open.track();
        ed.mobile.track();
        ed.view.zoom.track();
        ed.view.center.track();
        request_render(ed);
    });
    // Keep the filmstrip thumbnail of the open photo in step with its edits.
    let thumb_timer = StoredValue::new(None::<i32>);
    Effect::new(move |_| {
        ed.adj.track();
        let Some(id) = ed.current.get_untracked() else { return };
        if let Some(t) = thumb_timer.get_value() {
            platform::window().clear_timeout_with_handle(t);
        }
        let cb = wasm_bindgen::closure::Closure::once_into_js(move || {
            let adj = ed.adj.get_untracked();
            if ed.current.get_untracked() == Some(id) {
                if let Some(url) = io::render_thumb(&ed, &adj, 200) {
                    ed.set_thumb(id, url);
                }
            }
        });
        let handle = platform::window().set_timeout_with_callback_and_timeout_and_arguments_0(cb.unchecked_ref(), 700).ok();
        thumb_timer.set_value(handle);
    });
    // Entering or leaving the crop tool changes what's framed: re-fit smoothly.
    Effect::new(move |prev: Option<bool>| {
        let crop = crate::view::crop_mode(&ed);
        if prev.is_some_and(|p| p != crop) {
            ed.view.reset_now();
        }
        crop
    });

    // Keyboard shortcuts.
    let _keys = window_event_listener(ev::keydown, move |e: KeyboardEvent| {
        if typing_in_field(&e) {
            return;
        }
        let m = e.meta_key() || e.ctrl_key();
        let key = e.key();
        let handled = match (m, e.shift_key(), key.as_str()) {
            (true, _, "o") => {
                open.run(());
                true
            }
            (true, _, "e") if ed.has_photo() => {
                ed.export_open.set(true);
                true
            }
            (true, false, "z") => {
                ed.undo();
                true
            }
            (true, true, "z") | (true, true, "Z") | (true, false, "y") => {
                ed.redo();
                true
            }
            (true, false, "c") if ed.has_photo() => {
                ed.clipboard.set_value(Some(ed.adj.get_untracked()));
                ed.info("Look copied");
                true
            }
            (true, false, "v") if ed.clipboard.with_value(|c| c.is_some()) => {
                if let (Some(look), Some(id)) = (ed.clipboard.get_value(), ed.current.get_untracked()) {
                    ed.set_look_of(id, &look);
                    ed.info("Look pasted");
                }
                true
            }
            (false, _, "\\") => {
                ed.hold_original.set(true);
                true
            }
            (false, _, "c") | (false, _, "C") if ed.has_photo() => {
                let t = if ed.tool.get_untracked() == Tool::Crop { Tool::Light } else { Tool::Crop };
                ed.tool.set(t);
                ed.sheet_open.set(true);
                true
            }
            (false, _, "0") => {
                ed.view.fit(&ed);
                true
            }
            (false, _, "1") if ed.has_photo() => {
                let z = ed.view.one_to_one_zoom(&ed);
                ed.view.zoom_at(&ed, z, None, true);
                true
            }
            (false, _, "=") | (false, _, "+") if ed.has_photo() => {
                let z = ed.view.zoom.get_untracked() * 1.5;
                ed.view.zoom_at(&ed, z, None, true);
                true
            }
            (false, _, "-") if ed.has_photo() => {
                let z = ed.view.zoom.get_untracked() / 1.5;
                ed.view.zoom_at(&ed, z, None, true);
                true
            }
            (false, _, "ArrowLeft") | (false, _, "ArrowRight") => {
                let dir: i32 = if key == "ArrowLeft" { -1 } else { 1 };
                let ids: Vec<u32> = ed.photos.with_untracked(|p| p.iter().map(|p| p.id).collect());
                if let Some(pos) = ed.current.get_untracked().and_then(|c| ids.iter().position(|x| *x == c)) {
                    let next = pos as i32 + dir;
                    if next >= 0 && (next as usize) < ids.len() {
                        spawn_local(io::load_photo(ed, ids[next as usize]));
                    }
                }
                ids.len() > 1
            }
            (false, _, "Escape") => {
                if ed.export_open.get_untracked() {
                    ed.export_open.set(false);
                } else if ed.tool.get_untracked() == Tool::Crop {
                    ed.tool.set(Tool::Light);
                } else if ed.compare.get_untracked() {
                    ed.compare.set(false);
                } else {
                    ed.view.fit(&ed);
                }
                true
            }
            _ => false,
        };
        if handled {
            e.prevent_default();
        }
    });
    let _keyup = window_event_listener(ev::keyup, move |e: KeyboardEvent| {
        if e.key() == "\\" {
            ed.hold_original.set(false);
        }
    });

    // Drag & drop anywhere.
    let has_files = |e: &web_sys::DragEvent| {
        e.data_transfer()
            .map(|d| {
                let types = d.types();
                (0..types.length()).any(|i| types.get(i).as_string().as_deref() == Some("Files"))
            })
            .unwrap_or(false)
    };
    let _dragover = window_event_listener(ev::dragover, move |e| {
        if has_files(&e) {
            e.prevent_default();
            ed.dragging_file.set(true);
        }
    });
    let _dragleave = window_event_listener(ev::dragleave, move |e| {
        if e.related_target().is_none() {
            ed.dragging_file.set(false);
        }
    });
    let _drop = window_event_listener(ev::drop, move |e| {
        e.prevent_default();
        ed.dragging_file.set(false);
        let files = files_from_transfer(e.data_transfer());
        if !files.is_empty() {
            io::open_files(ed, files);
        }
    });
    // Paste an image from the clipboard.
    let _paste = window_event_listener(ev::paste, move |e: web_sys::ClipboardEvent| {
        if typing_in_field(&e) {
            return;
        }
        let files = files_from_transfer(e.clipboard_data());
        if !files.is_empty() {
            e.prevent_default();
            io::open_files(ed, files);
        }
    });

    let show_strip = move || ed.photos.with(|p| p.len() > 1) || (ed.has_photo() && !ed.mobile.get());

    view! {
        <div
            class="app"
            class:mobile=move || ed.mobile.get()
            class:looks-open=move || ed.presets_open.get() && !ed.mobile.get()
            class:sheet-open=move || ed.sheet_open.get() && ed.mobile.get() && ed.has_photo()
            class:empty-state=move || !ed.has_photo()
        >
            <TopBar on_open=open />
            <Show when=move || !ed.mobile.get()>
                <LooksColumn />
            </Show>
            <main class="main">
                <Stage on_open=open />
                <div class="strip-wrap" class:show=show_strip>
                    <Filmstrip on_add=open />
                </div>
            </main>
            <Show when=move || !ed.mobile.get() fallback=move || view! { <MobileTools /> }>
                <EditPanel />
            </Show>
            <ExportDialog />
            <Toasts />
            <div class="drop-overlay" class:show=move || ed.dragging_file.get()>
                <div class="drop-card">
                    <Icon icon=I::Image size=32 />
                    <span>"Drop to open"</span>
                </div>
            </div>
            <input
                type="file"
                class="hidden-input"
                multiple=true
                accept=io::ACCEPT
                node_ref=file_input
                on:change=move |ev| {
                    let input = event_target::<web_sys::HtmlInputElement>(&ev);
                    let files = io::file_list(input.files());
                    input.set_value("");
                    io::open_files(ed, files);
                }
            />
        </div>
    }
}
