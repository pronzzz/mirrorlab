//! Transient status messages.

use leptos::prelude::*;

use crate::icons::{Icon, I};
use crate::state::{use_editor, ToastKind};

#[component]
pub fn Toasts() -> impl IntoView {
    let ed = use_editor();
    view! {
        <div class="toasts" aria-live="polite">
            <For
                each=move || ed.toasts.get()
                key=|t| t.id
                children=move |t| {
                    let icon = match t.kind {
                        ToastKind::Success => Some(I::Check),
                        ToastKind::Error => Some(I::Close),
                        ToastKind::Info => None,
                    };
                    let id = t.id;
                    view! {
                        <div class="toast" class:error=t.kind == ToastKind::Error on:click=move |_| ed.toasts.update(|v| v.retain(|x| x.id != id))>
                            {icon.map(|i| view! { <Icon icon=i size=16 /> })}
                            <span>{t.text.clone()}</span>
                        </div>
                    }
                }
            />
        </div>
    }
}
