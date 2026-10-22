//! Application state. Everything is a `Copy` handle so closures can capture it freely.

use std::collections::HashMap;

use leptos::prelude::*;
use mirror_core::model::{Adjustments, Band, CurveChannel, Zone};
use mirror_core::presets::{self, Preset};

use crate::gl::Renderer;
use crate::icons::I;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Tool {
    Presets,
    Light,
    Color,
    Curve,
    Mixer,
    Grade,
    Effects,
    Crop,
}

impl Tool {
    /// Tools shown in the editing panel (presets live in their own column on desktop).
    pub const EDIT: [Tool; 7] = [Tool::Light, Tool::Color, Tool::Curve, Tool::Mixer, Tool::Grade, Tool::Effects, Tool::Crop];
    pub const MOBILE: [Tool; 8] =
        [Tool::Presets, Tool::Light, Tool::Color, Tool::Curve, Tool::Mixer, Tool::Grade, Tool::Effects, Tool::Crop];

    pub fn label(self) -> &'static str {
        match self {
            Tool::Presets => "Looks",
            Tool::Light => "Light",
            Tool::Color => "Color",
            Tool::Curve => "Curve",
            Tool::Mixer => "Mixer",
            Tool::Grade => "Grade",
            Tool::Effects => "Effects",
            Tool::Crop => "Crop",
        }
    }

    pub fn icon(self) -> I {
        match self {
            Tool::Presets => I::Presets,
            Tool::Light => I::Light,
            Tool::Color => I::Color,
            Tool::Curve => I::Curve,
            Tool::Mixer => I::Mixer,
            Tool::Grade => I::Grade,
            Tool::Effects => I::Effects,
            Tool::Crop => I::Crop,
        }
    }

    pub fn key(self) -> &'static str {
        self.label()
    }
}

/// Undo stack. One entry per finished gesture, not per slider tick.
#[derive(Clone, Debug)]
pub struct History {
    stack: Vec<Adjustments>,
    idx: usize,
}

const HISTORY_LIMIT: usize = 250;

impl History {
    pub fn new(start: Adjustments) -> Self {
        Self { stack: vec![start], idx: 0 }
    }

    pub fn current(&self) -> &Adjustments {
        &self.stack[self.idx]
    }

    /// Record `a` if it differs from the current entry.
    pub fn commit(&mut self, a: &Adjustments) -> bool {
        if self.current() == a {
            return false;
        }
        self.stack.truncate(self.idx + 1);
        self.stack.push(a.clone());
        if self.stack.len() > HISTORY_LIMIT {
            self.stack.remove(0);
        }
        self.idx = self.stack.len() - 1;
        true
    }

    pub fn undo(&mut self) -> Option<Adjustments> {
        (self.idx > 0).then(|| {
            self.idx -= 1;
            self.current().clone()
        })
    }

    pub fn redo(&mut self) -> Option<Adjustments> {
        (self.idx + 1 < self.stack.len()).then(|| {
            self.idx += 1;
            self.current().clone()
        })
    }

    pub fn can_undo(&self) -> bool {
        self.idx > 0
    }

    pub fn can_redo(&self) -> bool {
        self.idx + 1 < self.stack.len()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PhotoMeta {
    pub id: u32,
    pub name: String,
    pub thumb: Option<String>,
    pub edited: bool,
}

#[derive(Clone, Debug)]
struct PhotoEdit {
    adj: Adjustments,
    history: History,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToastKind {
    Info,
    Success,
    Error,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Toast {
    pub id: u32,
    pub text: String,
    pub kind: ToastKind,
}

/// Information about the decoded photo currently on the GPU.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Loaded {
    /// Size of the uploaded texture.
    pub width: u32,
    pub height: u32,
    /// Size of the original file.
    pub full_width: u32,
    pub full_height: u32,
    /// "RAW", "RAW preview", "TIFF"…
    pub note: Option<String>,
}

#[derive(Clone, Copy)]
pub struct Editor {
    pub adj: RwSignal<Adjustments>,
    history: StoredValue<History>,
    pub can_undo: RwSignal<bool>,
    pub can_redo: RwSignal<bool>,

    pub photos: RwSignal<Vec<PhotoMeta>>,
    pub current: RwSignal<Option<u32>>,
    pub loaded: RwSignal<Option<Loaded>>,
    edits: StoredValue<HashMap<u32, PhotoEdit>>,
    pub files: StoredValue<HashMap<u32, web_sys::File>, LocalStorage>,
    next_id: StoredValue<u32>,

    pub tool: RwSignal<Tool>,
    pub sheet_open: RwSignal<bool>,
    pub presets_open: RwSignal<bool>,
    pub compare: RwSignal<bool>,
    pub split: RwSignal<f32>,
    pub hold_original: RwSignal<bool>,
    pub export_open: RwSignal<bool>,
    pub busy: RwSignal<Option<String>>,
    pub toasts: RwSignal<Vec<Toast>>,
    pub dragging_file: RwSignal<bool>,

    pub presets: RwSignal<Vec<Preset>>,
    pub thumbs: RwSignal<HashMap<String, String>>,
    pub thumbs_key: StoredValue<String>,
    pub hist: RwSignal<Vec<f32>>,
    pub clip: RwSignal<(f32, f32)>,

    pub curve_channel: RwSignal<CurveChannel>,
    pub band: RwSignal<Band>,
    pub zone: RwSignal<Zone>,
    pub clipboard: StoredValue<Option<Adjustments>>,

    pub view: crate::view::ViewCtl,
    pub gl: StoredValue<Option<Renderer>, LocalStorage>,
    pub gl_error: RwSignal<Option<String>>,
    pub mobile: RwSignal<bool>,
}

const PRESETS_KEY: &str = "mirrorlab.presets.v2";

impl Editor {
    pub fn new() -> Self {
        let mut all = presets::builtin();
        all.extend(load_custom_presets());
        Self {
            adj: RwSignal::new(Adjustments::default()),
            history: StoredValue::new(History::new(Adjustments::default())),
            can_undo: RwSignal::new(false),
            can_redo: RwSignal::new(false),
            photos: RwSignal::new(Vec::new()),
            current: RwSignal::new(None),
            loaded: RwSignal::new(None),
            edits: StoredValue::new(HashMap::new()),
            files: StoredValue::new_local(HashMap::new()),
            next_id: StoredValue::new(1),
            tool: RwSignal::new(Tool::Light),
            sheet_open: RwSignal::new(false),
            presets_open: RwSignal::new(true),
            compare: RwSignal::new(false),
            split: RwSignal::new(0.5),
            hold_original: RwSignal::new(false),
            export_open: RwSignal::new(false),
            busy: RwSignal::new(None),
            toasts: RwSignal::new(Vec::new()),
            dragging_file: RwSignal::new(false),
            presets: RwSignal::new(all),
            thumbs: RwSignal::new(HashMap::new()),
            thumbs_key: StoredValue::new(String::new()),
            hist: RwSignal::new(Vec::new()),
            clip: RwSignal::new((0.0, 0.0)),
            curve_channel: RwSignal::new(CurveChannel::Master),
            band: RwSignal::new(Band::Red),
            zone: RwSignal::new(Zone::Shadows),
            clipboard: StoredValue::new(None),
            view: crate::view::ViewCtl::new(),
            gl: StoredValue::new_local(None),
            gl_error: RwSignal::new(None),
            mobile: RwSignal::new(false),
        }
    }

    pub fn has_photo(&self) -> bool {
        self.loaded.with(|l| l.is_some())
    }

    // ---- history -------------------------------------------------------

    fn sync_history_flags(&self) {
        let (u, r) = self.history.with_value(|h| (h.can_undo(), h.can_redo()));
        self.can_undo.set(u);
        self.can_redo.set(r);
    }

    /// Record the current adjustments as one undo step.
    pub fn commit(&self) {
        let a = self.adj.get_untracked();
        if self.history.try_update_value(|h| h.commit(&a)).unwrap_or(false) {
            self.sync_history_flags();
            self.mark_edited();
        }
    }

    /// Change the adjustments and record the change as one step.
    pub fn apply(&self, f: impl FnOnce(&mut Adjustments)) {
        self.adj.update(f);
        self.commit();
    }

    pub fn undo(&self) {
        if let Some(a) = self.history.try_update_value(|h| h.undo()).flatten() {
            self.adj.set(a);
            self.sync_history_flags();
            self.mark_edited();
        }
    }

    pub fn redo(&self) {
        if let Some(a) = self.history.try_update_value(|h| h.redo()).flatten() {
            self.adj.set(a);
            self.sync_history_flags();
            self.mark_edited();
        }
    }

    fn mark_edited(&self) {
        let Some(id) = self.current.get_untracked() else { return };
        let edited = !self.adj.with_untracked(|a| a.is_neutral() && a.geometry.is_identity());
        self.photos.update(|ps| {
            if let Some(p) = ps.iter_mut().find(|p| p.id == id) {
                if p.edited != edited {
                    p.edited = edited;
                }
            }
        });
    }

    // ---- photos --------------------------------------------------------

    pub fn add_photo(&self, file: web_sys::File) -> u32 {
        let id = self.next_id.get_value();
        self.next_id.set_value(id + 1);
        let name = file.name();
        self.files.update_value(|m| {
            m.insert(id, file);
        });
        self.photos.update(|ps| ps.push(PhotoMeta { id, name, thumb: None, edited: false }));
        id
    }

    pub fn file(&self, id: u32) -> Option<web_sys::File> {
        self.files.with_value(|m| m.get(&id).cloned())
    }

    pub fn photo_name(&self) -> Option<String> {
        let id = self.current.get()?;
        self.photos.with(|ps| ps.iter().find(|p| p.id == id).map(|p| p.name.clone()))
    }

    /// Park the current photo's edits so they come back when it's reopened.
    pub fn stash_current(&self) {
        if let Some(id) = self.current.get_untracked() {
            let adj = self.adj.get_untracked();
            let history = self.history.get_value();
            self.edits.update_value(|m| {
                m.insert(id, PhotoEdit { adj, history });
            });
        }
    }

    /// Bring back a photo's edits (or start fresh).
    pub fn restore(&self, id: u32) {
        let e = self.edits.with_value(|m| m.get(&id).cloned());
        let (adj, history) = match e {
            Some(e) => (e.adj, e.history),
            None => (Adjustments::default(), History::new(Adjustments::default())),
        };
        self.history.set_value(history);
        self.adj.set(adj);
        self.sync_history_flags();
    }


    /// Give another photo a look (its own geometry is kept) as one undo step.
    pub fn set_look_of(&self, id: u32, look: &Adjustments) {
        if Some(id) == self.current.get_untracked() {
            self.apply(|a| *a = a.with_look(look));
            return;
        }
        self.edits.update_value(|m| {
            let e = m.entry(id).or_insert_with(|| PhotoEdit {
                adj: Adjustments::default(),
                history: History::new(Adjustments::default()),
            });
            e.adj = e.adj.with_look(look);
            let a = e.adj.clone();
            e.history.commit(&a);
        });
        self.photos.update(|ps| {
            if let Some(p) = ps.iter_mut().find(|p| p.id == id) {
                p.edited = !look.is_neutral();
            }
        });
    }

    pub fn remove_photo(&self, id: u32) {
        self.files.update_value(|m| {
            m.remove(&id);
        });
        self.edits.update_value(|m| {
            m.remove(&id);
        });
        self.photos.update(|ps| ps.retain(|p| p.id != id));
    }

    pub fn set_thumb(&self, id: u32, url: String) {
        self.photos.update(|ps| {
            if let Some(p) = ps.iter_mut().find(|p| p.id == id) {
                p.thumb = Some(url);
            }
        });
    }

    // ---- presets -------------------------------------------------------

    pub fn add_preset(&self, p: Preset) {
        self.presets.update(|ps| {
            ps.retain(|q| q.id != p.id);
            ps.push(p);
        });
        self.save_custom_presets();
    }

    pub fn delete_preset(&self, id: &str) {
        self.presets.update(|ps| ps.retain(|p| p.builtin || p.id != id));
        self.thumbs.update(|t| {
            t.remove(id);
        });
        self.save_custom_presets();
    }

    fn save_custom_presets(&self) {
        let custom: Vec<Preset> = self.presets.with_untracked(|ps| ps.iter().filter(|p| !p.builtin).cloned().collect());
        if let Ok(json) = serde_json::to_string(&custom) {
            crate::platform::storage_set(PRESETS_KEY, &json);
        }
    }

    // ---- feedback ------------------------------------------------------

    pub fn toast(&self, text: impl Into<String>, kind: ToastKind) {
        let id = (js_sys::Math::random() * 1e9) as u32;
        let toasts = self.toasts;
        toasts.update(|t| {
            t.push(Toast { id, text: text.into(), kind });
            if t.len() > 3 {
                t.remove(0);
            }
        });
        let ms = if kind == ToastKind::Error { 5200 } else { 2600 };
        set_timeout(move || toasts.update(|t| t.retain(|x| x.id != id)), std::time::Duration::from_millis(ms));
    }

    pub fn info(&self, text: impl Into<String>) {
        self.toast(text, ToastKind::Info)
    }

    pub fn success(&self, text: impl Into<String>) {
        self.toast(text, ToastKind::Success)
    }

    pub fn error(&self, text: impl Into<String>) {
        self.toast(text, ToastKind::Error)
    }
}

fn load_custom_presets() -> Vec<Preset> {
    crate::platform::storage_get(PRESETS_KEY)
        .and_then(|s| serde_json::from_str::<Vec<Preset>>(&s).ok())
        .unwrap_or_default()
        .into_iter()
        .map(|mut p| {
            p.builtin = false;
            p
        })
        .collect()
}

pub fn use_editor() -> Editor {
    expect_context::<Editor>()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn history_steps() {
        let mut h = History::new(Adjustments::default());
        assert!(!h.can_undo());
        let a = Adjustments { exposure: 1.0, ..Default::default() };
        assert!(h.commit(&a));
        assert!(!h.commit(&a), "identical states are not recorded twice");
        assert_eq!(h.undo().unwrap().exposure, 0.0);
        assert!(h.can_redo());
        assert_eq!(h.redo().unwrap().exposure, 1.0);
        h.undo();
        h.commit(&Adjustments { contrast: 5.0, ..Default::default() });
        assert!(!h.can_redo(), "a new edit drops the redo branch");
    }
}
