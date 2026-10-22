//! MirrorLab's icon set: 24px grid, 1.6px round strokes, drawn for this app.

use leptos::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum I {
    Logo,
    Open,
    Export,
    Undo,
    Redo,
    Compare,
    Presets,
    Light,
    Color,
    Curve,
    Mixer,
    Grade,
    Effects,
    Crop,
    RotateLeft,
    RotateRight,
    FlipH,
    FlipV,
    Reset,
    Close,
    Check,
    Plus,
    Trash,
    Download,
    Upload,
    Image,
    Sidebar,
    Wand,
    Copy,
    Paste,
    Spinner,
}

impl I {
    fn paths(self) -> &'static str {
        match self {
            I::Logo => r#"<circle cx="12" cy="12" r="8.2"/><path d="M12 3.8a8.2 8.2 0 0 0 0 16.4z" fill="currentColor" stroke="none"/>"#,
            I::Open => r#"<rect x="3.5" y="5" width="13" height="13" rx="3"/><path d="M3.8 15.2l3.6-3.6a1.6 1.6 0 0 1 2.3 0l3.3 3.3"/><path d="M19.5 3.5v5M17 6h5"/>"#,
            I::Export => r#"<path d="M12 14.5V3.8M7.8 8 12 3.8 16.2 8"/><path d="M5 12.5v5a2.5 2.5 0 0 0 2.5 2.5h9a2.5 2.5 0 0 0 2.5-2.5v-5"/>"#,
            I::Undo => r#"<path d="M8.5 13.5 4 9l4.5-4.5"/><path d="M4.5 9H14a5.5 5.5 0 0 1 0 11h-3"/>"#,
            I::Redo => r#"<path d="M15.5 13.5 20 9l-4.5-4.5"/><path d="M19.5 9H10a5.5 5.5 0 0 0 0 11h3"/>"#,
            I::Compare => r#"<rect x="3.5" y="4.5" width="17" height="15" rx="3"/><path d="M12 2.5v19"/><path d="M12 4.5H6.5a3 3 0 0 0-3 3v9a3 3 0 0 0 3 3H12z" fill="currentColor" stroke="none" opacity=".28"/>"#,
            I::Presets => r#"<rect x="4" y="4" width="7" height="7" rx="2"/><rect x="13" y="4" width="7" height="7" rx="2"/><rect x="4" y="13" width="7" height="7" rx="2"/><rect x="13" y="13" width="7" height="7" rx="2"/>"#,
            I::Light => r#"<circle cx="12" cy="12" r="3.8"/><path d="M12 2.8v2M12 19.2v2M2.8 12h2M19.2 12h2M5.5 5.5l1.4 1.4M17.1 17.1l1.4 1.4M5.5 18.5l1.4-1.4M17.1 6.9l1.4-1.4"/>"#,
            I::Color => r#"<path d="M12 3.5s6.2 6.3 6.2 10.6a6.2 6.2 0 0 1-12.4 0C5.8 9.8 12 3.5 12 3.5z"/><path d="M9 14.5a3 3 0 0 0 3 3" opacity=".6"/>"#,
            I::Curve => r#"<path d="M4 4v16h16" opacity=".45"/><path d="M4.5 19C11 19 9.5 5 19.5 5"/><circle cx="11.8" cy="12" r="1.4" fill="currentColor"/>"#,
            I::Mixer => r#"<circle cx="9" cy="9.5" r="5"/><circle cx="15" cy="9.5" r="5"/><circle cx="12" cy="14.8" r="5"/>"#,
            I::Grade => r#"<circle cx="12" cy="12" r="8.5"/><circle cx="14.6" cy="9.4" r="2.2"/><path d="M12 12l1-1"/>"#,
            I::Effects => r#"<path d="M11 3.5l1.7 4.6 4.6 1.7-4.6 1.7L11 16.1l-1.7-4.6-4.6-1.7 4.6-1.7z"/><path d="M18 14.5l.8 2 2 .8-2 .8-.8 2-.8-2-2-.8 2-.8z"/>"#,
            I::Crop => r#"<path d="M7 2.5V15a2 2 0 0 0 2 2h12.5"/><path d="M2.5 7H15a2 2 0 0 1 2 2v12.5"/>"#,
            I::RotateLeft => r#"<path d="M4.5 4v5h5"/><path d="M4.9 9a7.5 7.5 0 1 1 .6 6.6"/>"#,
            I::RotateRight => r#"<path d="M19.5 4v5h-5"/><path d="M19.1 9a7.5 7.5 0 1 0-.6 6.6"/>"#,
            I::FlipH => r#"<path d="M12 3v18" stroke-dasharray="2 2.4"/><path d="M8.5 7.5 4 16.5h4.5z"/><path d="M15.5 7.5l4.5 9h-4.5z"/>"#,
            I::FlipV => r#"<path d="M3 12h18" stroke-dasharray="2 2.4"/><path d="M7.5 8.5 16.5 4v4.5z"/><path d="M7.5 15.5l9 4.5v-4.5z"/>"#,
            I::Reset => r#"<path d="M4.5 12a7.5 7.5 0 1 0 2.2-5.3L4.5 9"/><path d="M4.5 4.5V9H9"/>"#,
            I::Close => r#"<path d="M6.5 6.5l11 11M17.5 6.5l-11 11"/>"#,
            I::Check => r#"<path d="M5 12.5l4.5 4.5L19 7.5"/>"#,
            I::Plus => r#"<path d="M12 5v14M5 12h14"/>"#,
            I::Trash => r#"<path d="M4.5 7h15M10 4h4"/><path d="M6.5 7l.8 11.2A2 2 0 0 0 9.3 20h5.4a2 2 0 0 0 2-1.8L17.5 7"/>"#,
            I::Download => r#"<path d="M12 4v10.5M7.8 10.3 12 14.5l4.2-4.2"/><path d="M5 19.5h14"/>"#,
            I::Upload => r#"<path d="M12 15V4.5M7.8 8.7 12 4.5l4.2 4.2"/><path d="M5 19.5h14"/>"#,
            I::Image => r#"<rect x="3.5" y="4.5" width="17" height="15" rx="3"/><path d="M4 16.5l4.4-4.4a1.8 1.8 0 0 1 2.5 0l5.6 5.6"/><path d="M14 14.5l1.4-1.4a1.8 1.8 0 0 1 2.5 0l2.3 2.3"/><circle cx="15.5" cy="9" r="1.5"/>"#,
            I::Sidebar => r#"<rect x="3.5" y="4.5" width="17" height="15" rx="3"/><path d="M9.5 4.5v15"/>"#,
            I::Wand => r#"<path d="M4.5 19.5 15 9"/><path d="M14 4v2.5M19.5 9.5H17M17.3 5.7l-1.6 1.6M20 4.5l-.01.01"/><path d="M13.5 10.5l-1-1"/>"#,
            I::Copy => r#"<rect x="8.5" y="8.5" width="11" height="11" rx="2.5"/><path d="M15.5 5.5v-.5a1.5 1.5 0 0 0-1.5-1.5H6A2.5 2.5 0 0 0 3.5 6v8A1.5 1.5 0 0 0 5 15.5h.5"/>"#,
            I::Paste => r#"<rect x="5" y="5.5" width="14" height="15" rx="2.5"/><path d="M9 5.5V4.8A1.3 1.3 0 0 1 10.3 3.5h3.4A1.3 1.3 0 0 1 15 4.8v.7"/><path d="M9 12h6M9 15.5h4"/>"#,
            I::Spinner => r#"<path d="M12 3.5a8.5 8.5 0 1 1-8.5 8.5"/>"#,
        }
    }
}

/// An inline SVG icon that inherits `currentColor`.
#[component]
pub fn Icon(icon: I, #[prop(optional)] size: Option<u32>, #[prop(optional)] class: &'static str) -> impl IntoView {
    let s = size.unwrap_or(20);
    view! {
        <svg
            class=format!("icon {class}")
            width=s
            height=s
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="1.6"
            stroke-linecap="round"
            stroke-linejoin="round"
            aria-hidden="true"
            inner_html=icon.paths()
        ></svg>
    }
}
