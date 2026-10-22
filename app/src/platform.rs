//! Platform glue: browser storage, downloads, and the bridge to the native
//! Tauri shell (desktop + mobile) when the app runs inside it.

use js_sys::{Array, Object, Reflect, Uint8Array};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;

pub fn window() -> web_sys::Window {
    web_sys::window().expect("no window")
}

pub fn document() -> web_sys::Document {
    window().document().expect("no document")
}

pub fn storage_get(key: &str) -> Option<String> {
    window().local_storage().ok().flatten()?.get_item(key).ok().flatten()
}

pub fn storage_set(key: &str, value: &str) {
    if let Some(s) = window().local_storage().ok().flatten() {
        let _ = s.set_item(key, value);
    }
}

pub fn now_ms() -> f64 {
    window().performance().map(|p| p.now()).unwrap_or(0.0)
}

pub fn dpr() -> f32 {
    window().device_pixel_ratio().clamp(1.0, 3.0) as f32
}

fn user_agent() -> String {
    window().navigator().user_agent().unwrap_or_default().to_lowercase()
}

pub fn is_apple() -> bool {
    let ua = user_agent();
    ua.contains("mac os") || ua.contains("iphone") || ua.contains("ipad")
}

pub fn prefers_reduced_motion() -> bool {
    window()
        .match_media("(prefers-reduced-motion: reduce)")
        .ok()
        .flatten()
        .map(|m| m.matches())
        .unwrap_or(false)
}

// ---- Tauri bridge ------------------------------------------------------

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = invoke, catch)]
    fn tauri_invoke(cmd: &str, args: &JsValue, options: &JsValue) -> Result<js_sys::Promise, JsValue>;
}

/// True when running inside the MirrorLab native app.
pub fn is_tauri() -> bool {
    Reflect::get(&window(), &"__TAURI__".into()).map(|v| !v.is_undefined()).unwrap_or(false)
}

pub fn is_mobile_native() -> bool {
    is_tauri() && {
        let ua = user_agent();
        ua.contains("android") || ua.contains("iphone") || ua.contains("ipad")
    }
}

pub fn error_text(e: JsValue) -> String {
    e.as_string()
        .or_else(|| Reflect::get(&e, &"message".into()).ok().and_then(|m| m.as_string()))
        .unwrap_or_else(|| "Unknown error".into())
}

/// Call a native command with a raw byte body and string headers.
async fn invoke_bytes(cmd: &str, body: &[u8], headers: &[(&str, &str)]) -> Result<JsValue, String> {
    let p = if is_mobile_native() {
        // The mobile IPC bridge only carries JSON: send the bytes as base64 and the
        // headers as fields (`x-name` becomes `name`).
        use base64::Engine;
        let args = Object::new();
        for (k, v) in headers {
            Reflect::set(&args, &k.trim_start_matches("x-").into(), &(*v).into()).ok();
        }
        let data = base64::engine::general_purpose::STANDARD.encode(body);
        Reflect::set(&args, &"data".into(), &data.into()).ok();
        tauri_invoke(cmd, &args.into(), &JsValue::UNDEFINED)
    } else {
        let h = Object::new();
        for (k, v) in headers {
            Reflect::set(&h, &(*k).into(), &(*v).into()).ok();
        }
        let opts = Object::new();
        Reflect::set(&opts, &"headers".into(), &h).ok();
        let bytes = Uint8Array::from(body);
        tauri_invoke(cmd, &bytes.into(), &opts.into())
    }
    .map_err(error_text)?;
    JsFuture::from(p).await.map_err(error_text)
}

/// Decode a RAW/TIFF file natively (multi-threaded). Returns
/// `(width, height, full_width, full_height, note, rgba_f16_bits)`.
pub async fn native_decode(bytes: &[u8], name: &str, max_edge: u32) -> Result<(u32, u32, u32, u32, String, Vec<u16>), String> {
    let edge = max_edge.to_string();
    let v = invoke_bytes("decode_image", bytes, &[("x-name", name), ("x-max-edge", &edge)]).await?;
    let buf = Uint8Array::new(&v).to_vec();
    if buf.len() < 20 {
        return Err("Bad response from decoder".into());
    }
    let rd = |i: usize| u32::from_le_bytes([buf[i], buf[i + 1], buf[i + 2], buf[i + 3]]);
    let (w, h, fw, fh, note_len) = (rd(0), rd(4), rd(8), rd(12), rd(16) as usize);
    let note = String::from_utf8_lossy(&buf[20..20 + note_len]).to_string();
    let mut off = 20 + note_len;
    off += (2 - off % 2) % 2;
    let px: Vec<u16> = buf[off..].chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
    if px.len() != (w * h * 4) as usize {
        return Err("Decoder returned the wrong number of pixels".into());
    }
    Ok((w, h, fw, fh, note, px))
}

/// Save bytes through the native shell. Desktop shows a save dialog; mobile
/// writes to the photo gallery. Returns a human readable location.
pub async fn native_save(bytes: &[u8], name: &str, mime: &str) -> Result<Option<String>, String> {
    let v = invoke_bytes("save_image", bytes, &[("x-name", name), ("x-mime", mime)]).await?;
    Ok(v.as_string())
}

// ---- Browser saving ----------------------------------------------------

pub fn blob_from_bytes(bytes: &[u8], mime: &str) -> Result<web_sys::Blob, String> {
    let parts = Array::new();
    parts.push(&Uint8Array::from(bytes));
    let opts = web_sys::BlobPropertyBag::new();
    opts.set_type(mime);
    web_sys::Blob::new_with_u8_array_sequence_and_options(&parts, &opts).map_err(error_text)
}

/// Trigger a browser download of `blob`.
pub fn download_blob(blob: &web_sys::Blob, name: &str) -> Result<(), String> {
    let url = web_sys::Url::create_object_url_with_blob(blob).map_err(error_text)?;
    let a: web_sys::HtmlAnchorElement = document().create_element("a").map_err(error_text)?.unchecked_into();
    a.set_href(&url);
    a.set_download(name);
    a.style().set_property("display", "none").ok();
    document().body().map(|b| b.append_child(&a));
    a.click();
    a.remove();
    let u = url.clone();
    leptos::prelude::set_timeout(move || {
        let _ = web_sys::Url::revoke_object_url(&u);
    }, std::time::Duration::from_secs(20));
    Ok(())
}

/// Use the system share sheet with a file (mobile browsers). Returns false if unsupported.
pub async fn share_file(blob: &web_sys::Blob, name: &str, mime: &str) -> Result<bool, String> {
    let nav = window().navigator();
    let can_share = Reflect::get(&nav, &"canShare".into()).ok().filter(|f| f.is_function());
    let Some(can_share) = can_share else { return Ok(false) };
    let parts = Array::new();
    parts.push(blob);
    let props = web_sys::FilePropertyBag::new();
    props.set_type(mime);
    let file = web_sys::File::new_with_blob_sequence_and_options(&parts, name, &props).map_err(error_text)?;
    let files = Array::new();
    files.push(&file);
    let data = Object::new();
    Reflect::set(&data, &"files".into(), &files).ok();
    let ok = can_share
        .unchecked_ref::<js_sys::Function>()
        .call1(&nav, &data)
        .ok()
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if !ok {
        return Ok(false);
    }
    let share = Reflect::get(&nav, &"share".into()).map_err(error_text)?;
    let p: js_sys::Promise = share.unchecked_ref::<js_sys::Function>().call1(&nav, &data).map_err(error_text)?.unchecked_into();
    match JsFuture::from(p).await {
        Ok(_) => Ok(true),
        // The user closing the sheet is not an error.
        Err(e) if error_text(e.clone()).to_lowercase().contains("abort") => Ok(true),
        Err(e) => Err(error_text(e)),
    }
}

pub fn is_touch() -> bool {
    window()
        .match_media("(pointer: coarse)")
        .ok()
        .flatten()
        .map(|m| m.matches())
        .unwrap_or(false)
}

/// Read a `File` fully into memory.
pub async fn file_bytes(file: &web_sys::File) -> Result<Vec<u8>, String> {
    let buf = JsFuture::from(file.array_buffer()).await.map_err(error_text)?;
    Ok(Uint8Array::new(&buf).to_vec())
}

/// Wait for the next animation frame (lets the UI paint before heavy work).
pub async fn next_frame() {
    let p = js_sys::Promise::new(&mut |resolve, _| {
        let _ = window().request_animation_frame(&resolve);
    });
    let _ = JsFuture::from(p).await;
}


