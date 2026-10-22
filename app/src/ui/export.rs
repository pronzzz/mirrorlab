//! Export dialog.

use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::icons::{Icon, I};
use crate::io::{self, ExportOptions, Format};
use crate::platform;
use crate::state::use_editor;
use crate::ui::slider::Slider;

const SIZES: [(&str, Option<u32>); 4] = [("Full", None), ("4096", Some(4096)), ("2048", Some(2048)), ("1080", Some(1080))];

#[component]
pub fn ExportDialog() -> impl IntoView {
    let ed = use_editor();
    let formats = Format::available();
    let format = RwSignal::new(Format::Jpeg);
    let quality = RwSignal::new(92.0f32);
    let size = RwSignal::new(None::<u32>);
    let all = RwSignal::new(false);
    let working = RwSignal::new(false);
    let name = RwSignal::new(String::new());

    // Suggest a file name whenever the dialog opens.
    Effect::new(move |_| {
        if ed.export_open.get() {
            let base = ed.photo_name().unwrap_or_else(|| "photo".into());
            let stem = base.rsplit_once('.').map(|(s, _)| s.to_string()).unwrap_or(base);
            name.set(format!("{stem}-edit"));
            all.set(false);
        }
    });

    let dims = move || {
        let adj = ed.adj.get();
        let (w, h) = io::export_size(&ed, &adj, size.get());
        format!("{w} × {h}")
    };
    let count = move || ed.photos.with(|p| p.len());

    let close = move || {
        if !working.get_untracked() {
            ed.export_open.set(false);
        }
    };

    let run = move |_| {
        if working.get_untracked() {
            return;
        }
        working.set(true);
        let opts = ExportOptions {
            format: format.get_untracked(),
            quality: quality.get_untracked().round() as u8,
            long_edge: size.get_untracked(),
            name: name.get_untracked(),
        };
        let every = all.get_untracked() && count() > 1;
        spawn_local(async move {
            let res = if every {
                io::export_all(ed, ExportOptions { name: "-edit".into(), ..opts }).await
            } else {
                io::export_current(ed, opts).await
            };
            ed.busy.set(None);
            working.set(false);
            match res {
                Ok(msg) => {
                    ed.export_open.set(false);
                    ed.success(msg);
                }
                Err(e) => ed.error(format!("Export failed: {e}")),
            }
        });
    };

    let q_value = Signal::derive(move || quality.get());
    let q_set = Callback::new(move |v: f32| quality.set(v));

    view! {
        <div class="modal-backdrop" class:open=move || ed.export_open.get() on:click=move |_| close()>
            <div class="modal" role="dialog" aria-label="Export" on:click=|ev| ev.stop_propagation()>
                <header class="modal-head">
                    <h2>"Export"</h2>
                    <button class="icon-btn ghost" title="Close" on:click=move |_| close()>
                        <Icon icon=I::Close size=18 />
                    </button>
                </header>
                <div class="modal-body">
                    <Show when=move || { count() > 1 }>
                        <div class="segmented">
                            <button class="seg" class:selected=move || !all.get() on:click=move |_| all.set(false)>"This photo"</button>
                            <button class="seg" class:selected=move || all.get() on:click=move |_| all.set(true)>
                                {move || format!("All {} photos", count())}
                            </button>
                        </div>
                    </Show>
                    <label class="field" class:muted=move || all.get()>
                        <span>"File name"</span>
                        <input
                            class="text-input"
                            prop:value=move || name.get()
                            disabled=move || all.get()
                            on:input=move |ev| name.set(event_target_value(&ev))
                            on:keydown=move |ev: web_sys::KeyboardEvent| ev.stop_propagation()
                        />
                    </label>
                    <div class="field">
                        <span>"Format"</span>
                        <div class="segmented">
                            {formats
                                .iter()
                                .map(|f| {
                                    let f = *f;
                                    view! { <button class="seg" class:selected=move || format.get() == f on:click=move |_| format.set(f)>{f.label()}</button> }
                                })
                                .collect_view()}
                        </div>
                    </div>
                    <div class="reveal" class:open=move || format.get().lossy()>
                        <div class="reveal-inner">
                            <Slider label="Quality" value=q_value set=q_set min=40.0 max=100.0 default=92.0 />
                        </div>
                    </div>
                    <div class="field">
                        <span>"Size · long edge"</span>
                        <div class="segmented">
                            {SIZES
                                .into_iter()
                                .map(|(l, s)| view! { <button class="seg" class:selected=move || size.get() == s on:click=move |_| size.set(s)>{l}</button> })
                                .collect_view()}
                        </div>
                    </div>
                    <p class="export-summary">
                        {move || if all.get() { format!("{} photos · {}", count(), format.get().label()) } else { format!("{} · {}", dims(), format.get().label()) }}
                        {move || if platform::is_mobile_native() { " · saved to Photos" } else { "" }}
                    </p>
                </div>
                <footer class="modal-foot">
                    <button class="btn ghost" on:click=move |_| close()>"Cancel"</button>
                    <button class="btn primary" class:busy=move || working.get() on:click=run>
                        {move || if working.get() {
                            view! { <Icon icon=I::Spinner size=16 class="spin" /> }.into_any()
                        } else {
                            view! { <Icon icon=I::Export size=16 /> }.into_any()
                        }}
                        {move || if working.get() { "Exporting…" } else { "Export" }}
                    </button>
                </footer>
            </div>
        </div>
    }
}
