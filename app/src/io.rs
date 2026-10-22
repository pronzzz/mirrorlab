//! Opening photos, thumbnails, exporting and preset files.

use leptos::prelude::*;
use leptos::task::spawn_local;
use mirror_core::decode::{self, Source};
use mirror_core::histogram::Histogram;
use mirror_core::model::Adjustments;
use mirror_core::presets::Preset;
use wasm_bindgen::prelude::*;
use wasm_bindgen::{Clamped, JsCast};
use wasm_bindgen_futures::JsFuture;
use web_sys::{File, HtmlCanvasElement, ImageBitmap, ImageBitmapOptions};

use crate::platform::{self, error_text, next_frame};
use crate::state::{Editor, Loaded};
use crate::view::{bump_generation, request_render};

thread_local! {
    static LOADING: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

pub const ACCEPT: &str = "image/*,.dng,.cr2,.cr3,.nef,.nrw,.arw,.orf,.rw2,.raf,.pef,.srw,.3fr,.iiq,.erf,.kdc,.mos,.mrw,.rwl,.tif,.tiff,.heic,.heif,.avif";

fn is_supported(f: &File) -> bool {
    let name = f.name();
    let ext = decode::extension(&name);
    f.type_().starts_with("image/") || decode::is_raw(&name) || decode::IMAGE_EXTENSIONS.contains(&ext.as_str())
}

/// Add files to the library and open the first one.
pub fn open_files(ed: Editor, files: Vec<File>) {
    let (ok, skipped): (Vec<File>, Vec<File>) = files.into_iter().partition(is_supported);
    if !skipped.is_empty() {
        ed.error(format!("Skipped {} file(s) that aren't photos", skipped.len()));
    }
    if ok.is_empty() {
        return;
    }
    let ids: Vec<u32> = ok.into_iter().map(|f| ed.add_photo(f)).collect();
    spawn_local(async move {
        load_photo(ed, ids[0]).await;
        for id in ids.iter().skip(1) {
            if let Some(f) = ed.file(*id) {
                if let Some(url) = file_thumb(&f).await {
                    ed.set_thumb(*id, url);
                }
            }
        }
    });
}

pub fn file_list(list: Option<web_sys::FileList>) -> Vec<File> {
    let Some(list) = list else { return Vec::new() };
    (0..list.length()).filter_map(|i| list.get(i)).collect()
}

/// Make `id` the photo being edited.
pub async fn load_photo(ed: Editor, id: u32) {
    if ed.current.get_untracked() == Some(id) && ed.has_photo() {
        return;
    }
    if LOADING.with(|l| l.replace(true)) {
        return;
    }
    let Some(file) = ed.file(id) else {
        LOADING.with(|l| l.set(false));
        return;
    };
    // Keep a thumbnail of the photo we're leaving that shows its edits.
    if let Some(prev) = ed.current.get_untracked() {
        let adj = ed.adj.get_untracked();
        if let Some(url) = render_thumb(&ed, &adj, 200) {
            ed.set_thumb(prev, url);
        }
    }
    ed.stash_current();
    ed.busy.set(Some(format!("Opening {}", file.name())));
    next_frame().await;
    next_frame().await;

    match decode_and_upload(&ed, &file).await {
        Ok(info) => {
            ed.current.set(Some(id));
            ed.restore(id);
            ed.loaded.set(Some(info));
            ed.view.reset_now();
            bump_generation();
            request_render(ed);
            let adj = ed.adj.get_untracked();
            if let Some(url) = render_thumb(&ed, &adj, 200) {
                ed.set_thumb(id, url);
            }
            refresh_preset_thumbs(ed, true);
        }
        Err(e) => {
            ed.error(format!("Couldn't open {}: {e}", file.name()));
            if ed.current.get_untracked() != Some(id) {
                ed.remove_photo(id);
            }
        }
    }
    ed.busy.set(None);
    LOADING.with(|l| l.set(false));
}

async fn bitmap(blob: &web_sys::Blob, resize: Option<(u32, u32)>) -> Result<ImageBitmap, String> {
    let opts = ImageBitmapOptions::new();
    opts.set_image_orientation(web_sys::ImageOrientation::FromImage);
    opts.set_premultiply_alpha(web_sys::PremultiplyAlpha::None);
    if let Some((w, h)) = resize {
        opts.set_resize_width(w);
        opts.set_resize_height(h);
        opts.set_resize_quality(web_sys::ResizeQuality::High);
    }
    let p = platform::window().create_image_bitmap_with_blob_and_image_bitmap_options(blob, &opts).map_err(error_text)?;
    let v = JsFuture::from(p).await.map_err(error_text)?;
    v.dyn_into::<ImageBitmap>().map_err(|_| "Not an image".to_string())
}

fn fit(w: u32, h: u32, max: u32) -> (u32, u32) {
    if w.max(h) <= max {
        return (w, h);
    }
    let s = max as f64 / w.max(h) as f64;
    (((w as f64 * s).round() as u32).max(1), ((h as f64 * s).round() as u32).max(1))
}

async fn decode_and_upload(ed: &Editor, file: &File) -> Result<Loaded, String> {
    let name = file.name();
    let max = ed.gl.with_value(|g| g.as_ref().map(|r| r.max_texture)).ok_or("The GPU isn't ready yet")?.min(16384);

    // Formats the browser decodes natively (fast, hardware accelerated).
    if !decode::needs_engine(&name) {
        if let Ok(bmp) = bitmap(file, None).await {
            let (fw, fh) = (bmp.width(), bmp.height());
            let bmp = if fw.max(fh) > max {
                bmp.close();
                bitmap(file, Some(fit(fw, fh, max))).await?
            } else {
                bmp
            };
            let (w, h) = (bmp.width(), bmp.height());
            let res = ed.gl.try_update_value(|g| g.as_mut().map(|r| r.set_source_bitmap(&bmp, (fw, fh)))).flatten();
            bmp.close();
            res.ok_or("The GPU isn't ready yet")??;
            return Ok(Loaded { width: w, height: h, full_width: fw, full_height: fh, note: None });
        }
    }

    // RAW, TIFF and anything the browser can't decode: the Rust engine.
    let bytes = platform::file_bytes(file).await?;
    let (w, h, fw, fh, note, px) = if platform::is_tauri() {
        platform::native_decode(&bytes, &name, max).await?
    } else {
        ed.busy.set(Some(if decode::is_raw(&name) { "Developing RAW…".into() } else { format!("Decoding {name}") }));
        next_frame().await;
        let d = decode::decode(&bytes, &name)?;
        let (fw, fh) = (d.image.width as u32, d.image.height as u32);
        let img = d.image.fit(max as usize);
        let note = match d.source {
            Source::Raw => d.camera.map(|c| format!("RAW · {c}")).unwrap_or_else(|| "RAW".into()),
            Source::RawPreview => "RAW preview".into(),
            Source::Bitmap => decode::extension(&name).to_uppercase(),
        };
        (img.width as u32, img.height as u32, fw, fh, note, img.to_rgba_f16_bits())
    };
    ed.gl
        .try_update_value(|g| g.as_mut().map(|r| r.set_source_f16(w, h, &px, (fw, fh))))
        .flatten()
        .ok_or("The GPU isn't ready yet")??;
    Ok(Loaded { width: w, height: h, full_width: fw, full_height: fh, note: Some(note).filter(|n| !n.is_empty()) })
}

// ---- thumbnails -------------------------------------------------------

fn canvas(w: u32, h: u32) -> Result<HtmlCanvasElement, String> {
    let c: HtmlCanvasElement = platform::document().create_element("canvas").map_err(error_text)?.unchecked_into();
    c.set_width(w);
    c.set_height(h);
    Ok(c)
}

fn ctx2d(c: &HtmlCanvasElement) -> Result<web_sys::CanvasRenderingContext2d, String> {
    c.get_context("2d").map_err(error_text)?.ok_or("No 2D canvas")?.dyn_into().map_err(|_| "No 2D canvas".to_string())
}

fn pixels_canvas(px: &[u8], w: u32, h: u32) -> Result<HtmlCanvasElement, String> {
    let c = canvas(w, h)?;
    let data = web_sys::ImageData::new_with_u8_clamped_array_and_sh(Clamped(px), w, h).map_err(error_text)?;
    ctx2d(&c)?.put_image_data(&data, 0.0, 0.0).map_err(error_text)?;
    Ok(c)
}

pub fn pixels_to_url(px: &[u8], w: u32, h: u32) -> Option<String> {
    pixels_canvas(px, w, h).ok()?.to_data_url_with_type_and_encoder_options("image/jpeg", &JsValue::from_f64(0.82)).ok()
}

/// Render the current photo with `adj` into a small JPEG data URL.
pub fn render_thumb(ed: &Editor, adj: &Adjustments, max_edge: u32) -> Option<String> {
    ed.gl
        .try_update_value(|g| {
            let r = g.as_mut()?;
            let (w, h) = r.output_size(adj);
            if w == 0 {
                return None;
            }
            let (tw, th) = fit(w, h, max_edge);
            let px = r.render_pixels(adj, tw, th);
            pixels_to_url(&px, tw, th)
        })
        .flatten()
}

/// Thumbnail straight from a file (before it's been opened).
async fn file_thumb(file: &File) -> Option<String> {
    if decode::needs_engine(&file.name()) {
        return None;
    }
    let probe = bitmap(file, None).await.ok()?;
    let (w, h) = fit(probe.width(), probe.height(), 200);
    probe.close();
    let bmp = bitmap(file, Some((w, h))).await.ok()?;
    let c = canvas(w, h).ok()?;
    ctx2d(&c).ok()?.draw_image_with_image_bitmap(&bmp, 0.0, 0.0).ok()?;
    bmp.close();
    c.to_data_url_with_type_and_encoder_options("image/jpeg", &JsValue::from_f64(0.8)).ok()
}

/// Re-render the preset thumbnails for the current photo, a few per frame.
pub fn refresh_preset_thumbs(ed: Editor, force: bool) {
    let Some(id) = ed.current.get_untracked() else { return };
    let geom = ed.adj.with_untracked(|a| serde_json::to_string(&a.geometry).unwrap_or_default());
    let n = ed.presets.with_untracked(|p| p.len());
    let key = format!("{id}|{geom}|{n}");
    if !force && ed.thumbs_key.get_value() == key {
        return;
    }
    ed.thumbs_key.set_value(key.clone());
    let presets = ed.presets.get_untracked();
    spawn_local(async move {
        let base = ed.adj.get_untracked();
        let mut jobs: Vec<(String, Adjustments)> = vec![("original".into(), base.with_look(&Adjustments::default()))];
        jobs.extend(presets.iter().map(|p| (p.id.clone(), base.with_look(&p.look))));
        for chunk in jobs.chunks(3) {
            next_frame().await;
            if ed.thumbs_key.get_value() != key {
                return;
            }
            for (pid, adj) in chunk {
                let edge = (148.0 * platform::dpr()) as u32;
                if let Some(url) = render_thumb(&ed, adj, edge) {
                    ed.thumbs.update(|t| {
                        t.insert(pid.clone(), url);
                    });
                }
            }
        }
    });
}

// ---- auto ---------------------------------------------------------------

pub fn auto_light(ed: Editor) {
    let base = ed.adj.get_untracked().with_look(&Adjustments::default());
    let hist = ed.gl.try_update_value(|g| {
        let r = g.as_mut()?;
        let (w, h) = r.output_size(&base);
        let (tw, th) = fit(w, h, 256);
        Some(Histogram::from_rgba(&r.render_pixels(&base, tw, th)))
    });
    if let Some(Some(h)) = hist {
        let s = mirror_core::auto::auto_light(&h.bins[3]);
        let from = ed.adj.get_untracked();
        let mut to = from.clone();
        s.apply(&mut to);
        // Glide into the new values instead of jumping.
        crate::view::tween(
            0.0,
            1.0,
            260.0,
            move |t| {
                ed.adj.update(|a| {
                    let l = |x: f32, y: f32| x + (y - x) * t;
                    a.exposure = l(from.exposure, to.exposure);
                    a.contrast = l(from.contrast, to.contrast);
                    a.highlights = l(from.highlights, to.highlights);
                    a.shadows = l(from.shadows, to.shadows);
                    a.whites = l(from.whites, to.whites);
                    a.blacks = l(from.blacks, to.blacks);
                })
            },
            move || ed.commit(),
        );
    }
}

// ---- export -------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    Jpeg,
    Png,
    Webp,
}

impl Format {
    pub fn mime(self) -> &'static str {
        match self {
            Format::Jpeg => "image/jpeg",
            Format::Png => "image/png",
            Format::Webp => "image/webp",
        }
    }

    pub fn ext(self) -> &'static str {
        match self {
            Format::Jpeg => "jpg",
            Format::Png => "png",
            Format::Webp => "webp",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Format::Jpeg => "JPEG",
            Format::Png => "PNG",
            Format::Webp => "WebP",
        }
    }

    pub fn lossy(self) -> bool {
        self != Format::Png
    }

    /// Formats this platform can encode.
    pub fn available() -> Vec<Format> {
        let mut v = vec![Format::Jpeg, Format::Png];
        if webp_supported() {
            v.push(Format::Webp);
        }
        v
    }
}

fn webp_supported() -> bool {
    canvas(1, 1)
        .ok()
        .and_then(|c| c.to_data_url_with_type("image/webp").ok())
        .map(|u| u.starts_with("data:image/webp"))
        .unwrap_or(false)
}

#[derive(Clone, Debug, PartialEq)]
pub struct ExportOptions {
    pub format: Format,
    pub quality: u8,
    pub long_edge: Option<u32>,
    pub name: String,
}

pub fn export_size(ed: &Editor, adj: &Adjustments, long_edge: Option<u32>) -> (u32, u32) {
    let (w, h) = ed.gl.with_value(|g| g.as_ref().map(|r| r.output_size(adj))).unwrap_or((0, 0));
    match long_edge {
        Some(e) => fit(w, h, e),
        None => (w, h),
    }
}

async fn to_blob(c: &HtmlCanvasElement, mime: &str, q: f64) -> Option<web_sys::Blob> {
    let c = c.clone();
    let mime = mime.to_string();
    let p = js_sys::Promise::new(&mut |resolve, _| {
        let on_fail = resolve.clone();
        let cb = Closure::once_into_js(move |b: JsValue| {
            let _ = resolve.call1(&JsValue::NULL, &b);
        });
        if c.to_blob_with_type_and_encoder_options(cb.unchecked_ref(), &mime, &JsValue::from_f64(q)).is_err() {
            let _ = on_fail.call1(&JsValue::NULL, &JsValue::NULL);
        }
    });
    let v = JsFuture::from(p).await.ok()?;
    v.dyn_into::<web_sys::Blob>().ok()
}

async fn encode(px: Vec<u8>, w: u32, h: u32, format: Format, quality: u8) -> Result<(web_sys::Blob, Format), String> {
    if let Ok(c) = pixels_canvas(&px, w, h) {
        if let Some(b) = to_blob(&c, format.mime(), quality as f64 / 100.0).await {
            if b.type_() == format.mime() {
                c.set_width(1);
                return Ok((b, format));
            }
        }
        c.set_width(1);
    }
    // Very large images can exceed the browser's canvas limits: encode in Rust instead.
    let (bytes, fmt) = match format {
        Format::Png => (mirror_core::encode::png(&px, w, h)?, Format::Png),
        _ => (mirror_core::encode::jpeg(&px, w, h, quality)?, Format::Jpeg),
    };
    Ok((platform::blob_from_bytes(&bytes, fmt.mime())?, fmt))
}

async fn blob_bytes(b: &web_sys::Blob) -> Result<Vec<u8>, String> {
    let buf = JsFuture::from(b.array_buffer()).await.map_err(error_text)?;
    Ok(js_sys::Uint8Array::new(&buf).to_vec())
}

/// Save a finished file. Returns where it went, for the toast.
pub async fn save_blob(blob: &web_sys::Blob, name: &str, mime: &str) -> Result<Option<String>, String> {
    if platform::is_tauri() {
        let bytes = blob_bytes(blob).await?;
        return platform::native_save(&bytes, name, mime).await;
    }
    if platform::is_touch() && platform::share_file(blob, name, mime).await? {
        return Ok(Some("Shared".into()));
    }
    platform::download_blob(blob, name)?;
    Ok(Some("Downloaded".into()))
}

/// Render at full quality and save. Returns a status line for the toast.
pub async fn export_current(ed: Editor, opts: ExportOptions) -> Result<String, String> {
    let adj = ed.adj.get_untracked();
    let (w, h) = export_size(&ed, &adj, opts.long_edge);
    if w == 0 {
        return Err("Open a photo first".into());
    }
    ed.busy.set(Some(format!("Rendering {w} × {h}")));
    next_frame().await;
    next_frame().await;
    let px = ed
        .gl
        .try_update_value(|g| g.as_mut().map(|r| r.render_pixels(&adj, w, h)))
        .flatten()
        .ok_or("The GPU isn't ready")?;
    ed.busy.set(Some(format!("Encoding {}", opts.format.label())));
    next_frame().await;
    let (blob, fmt) = encode(px, w, h, opts.format, opts.quality).await?;
    let stem = opts.name.trim().trim_end_matches(&format!(".{}", fmt.ext())).to_string();
    let stem = if stem.is_empty() { "mirrorlab".to_string() } else { stem };
    let name = format!("{stem}.{}", fmt.ext());
    ed.busy.set(Some("Saving".into()));
    let place = save_blob(&blob, &name, fmt.mime()).await?;
    let note = if fmt != opts.format { format!(" (saved as {})", fmt.label()) } else { String::new() };
    Ok(match place {
        Some(p) => format!("{p} · {name}{note}"),
        None => "Export cancelled".into(),
    })
}

/// Export every photo in the library with the same options.
pub async fn export_all(ed: Editor, opts: ExportOptions) -> Result<String, String> {
    let ids: Vec<(u32, String)> = ed.photos.with_untracked(|p| p.iter().map(|p| (p.id, p.name.clone())).collect());
    let back = ed.current.get_untracked();
    let mut done = 0;
    for (id, name) in &ids {
        load_photo(ed, *id).await;
        if ed.current.get_untracked() != Some(*id) {
            continue;
        }
        let stem = name.rsplit_once('.').map(|(s, _)| s).unwrap_or(name);
        let o = ExportOptions { name: format!("{stem}{}", opts.name), ..opts.clone() };
        export_current(ed, o).await?;
        done += 1;
    }
    if let Some(b) = back {
        load_photo(ed, b).await;
    }
    Ok(format!("Exported {done} of {} photos", ids.len()))
}

// ---- preset files ------------------------------------------------------------

pub async fn import_presets(ed: Editor, files: Vec<File>) {
    let mut added = 0;
    for f in files {
        let text = match JsFuture::from(f.text()).await {
            Ok(t) => t.as_string().unwrap_or_default(),
            Err(e) => {
                ed.error(error_text(e));
                continue;
            }
        };
        let stem = f.name().rsplit_once('.').map(|(s, _)| s.to_string()).unwrap_or(f.name());
        match Preset::from_file(&text, &stem) {
            Ok(p) => {
                ed.add_preset(p);
                added += 1;
            }
            Err(e) => ed.error(format!("{}: {e}", f.name())),
        }
    }
    if added > 0 {
        ed.success(format!("Added {added} look(s)"));
        refresh_preset_thumbs(ed, true);
    }
}

pub async fn export_preset(ed: Editor, p: Preset) {
    let text = p.to_file();
    let name = format!("{}.{}", p.name, mirror_core::presets::FILE_EXTENSION);
    let res = async {
        let blob = platform::blob_from_bytes(text.as_bytes(), "application/json")?;
        if platform::is_tauri() {
            platform::native_save(text.as_bytes(), &name, "application/json").await
        } else {
            platform::download_blob(&blob, &name).map(|_| Some("Downloaded".to_string()))
        }
    }
    .await;
    match res {
        Ok(Some(_)) => ed.success(format!("Saved {name}")),
        Ok(None) => {}
        Err(e) => ed.error(e),
    }
}
