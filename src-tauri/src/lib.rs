//! MirrorLab native shell.
//!
//! The editor UI is the same Rust/WebAssembly app as the web version. This
//! crate adds what a browser can't do well:
//! * decode camera RAW / TIFF natively on all CPU cores (`decode_image`),
//! * save exports through a native dialog on desktop, or into the photo
//!   gallery on Android (`save_image`).

mod gallery;

use tauri::ipc::{InvokeBody, Request, Response};
use tauri::AppHandle;

/// A string parameter: an `x-…` header on desktop (raw binary IPC), or a field of
/// the JSON body on mobile, where the IPC bridge only carries JSON.
fn header(req: &Request<'_>, name: &str) -> Option<String> {
    if let Some(v) = req.headers().get(name).and_then(|v| v.to_str().ok()) {
        return Some(v.to_string());
    }
    let field = name.trim_start_matches("x-");
    match req.body() {
        InvokeBody::Json(serde_json::Value::Object(m)) => m.get(field).and_then(|v| match v {
            serde_json::Value::String(s) => Some(s.clone()),
            other => Some(other.to_string()),
        }),
        _ => None,
    }
}

/// The file bytes: a raw body on desktop, base64 in `data` on mobile.
fn body(req: &Request<'_>) -> Result<Vec<u8>, String> {
    use base64::Engine;
    match req.body() {
        InvokeBody::Raw(b) => Ok(b.clone()),
        InvokeBody::Json(serde_json::Value::Object(m)) => {
            let data = m.get("data").and_then(|v| v.as_str()).ok_or("Missing file data")?;
            base64::engine::general_purpose::STANDARD.decode(data).map_err(|e| e.to_string())
        }
        _ => Err("Expected file data".into()),
    }
}

/// Decode a file and return it in the wire format read by `platform::native_decode`:
/// `w, h, full_w, full_h, note_len` (u32 LE), the note, padding to 2 bytes, RGBA f16 LE.
fn decode_to_wire(bytes: &[u8], name: &str, max_edge: usize) -> Result<Vec<u8>, String> {
    use mirror_core::decode::{self, Source};
    let d = decode::decode(bytes, name)?;
    let (fw, fh) = (d.image.width as u32, d.image.height as u32);
    let note = match d.source {
        Source::Raw => d.camera.map(|c| format!("RAW · {c}")).unwrap_or_else(|| "RAW".into()),
        Source::RawPreview => "RAW preview".into(),
        Source::Bitmap => decode::extension(name).to_uppercase(),
    };
    let img = d.image.fit(max_edge);
    let px = img.to_rgba_f16_bits();
    let mut out = Vec::with_capacity(24 + note.len() + px.len() * 2);
    for v in [img.width as u32, img.height as u32, fw, fh, note.len() as u32] {
        out.extend_from_slice(&v.to_le_bytes());
    }
    out.extend_from_slice(note.as_bytes());
    if out.len() % 2 == 1 {
        out.push(0);
    }
    for v in px {
        out.extend_from_slice(&v.to_le_bytes());
    }
    Ok(out)
}

#[tauri::command]
async fn decode_image(request: Request<'_>) -> Result<Response, String> {
    let bytes = body(&request)?;
    let name = header(&request, "x-name").unwrap_or_default();
    let max_edge = header(&request, "x-max-edge").and_then(|v| v.parse().ok()).unwrap_or(8192);
    let wire = tauri::async_runtime::spawn_blocking(move || decode_to_wire(&bytes, &name, max_edge))
        .await
        .map_err(|e| e.to_string())??;
    Ok(Response::new(wire))
}

#[tauri::command]
async fn save_image(app: AppHandle, request: Request<'_>) -> Result<Option<String>, String> {
    let bytes = body(&request)?;
    let name = header(&request, "x-name").unwrap_or_else(|| "mirrorlab.jpg".into());
    let mime = header(&request, "x-mime").unwrap_or_else(|| "image/jpeg".into());
    save(app, bytes, name, mime).await
}

#[cfg(desktop)]
async fn save(app: AppHandle, bytes: Vec<u8>, name: String, _mime: String) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    let ext = name.rsplit_once('.').map(|(_, e)| e.to_lowercase()).unwrap_or_else(|| "jpg".into());
    let label = match ext.as_str() {
        "jpg" | "jpeg" => "JPEG image",
        "png" => "PNG image",
        "webp" => "WebP image",
        "mlpreset" => "MirrorLab look",
        _ => "File",
    };
    let picked = tauri::async_runtime::spawn_blocking(move || {
        app.dialog().file().set_title("Save").set_file_name(&name).add_filter(label, &[ext.as_str()]).blocking_save_file()
    })
    .await
    .map_err(|e| e.to_string())?;
    let Some(path) = picked else { return Ok(None) };
    let path = path.into_path().map_err(|e| e.to_string())?;
    std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
    Ok(Some(format!("Saved to {}", path.display())))
}

#[cfg(mobile)]
async fn save(app: AppHandle, bytes: Vec<u8>, name: String, mime: String) -> Result<Option<String>, String> {
    gallery::save(&app, &bytes, &name, &mime).map(Some)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(gallery::init())
        .invoke_handler(tauri::generate_handler![decode_image, save_image])
        .run(tauri::generate_context!())
        .expect("error while running MirrorLab");
}

#[cfg(test)]
mod tests {
    #[test]
    fn wire_format_roundtrip() {
        let mut png = Vec::new();
        let img = image_png(4, 3);
        png.extend_from_slice(&img);
        let wire = super::decode_to_wire(&png, "x.png", 8192).unwrap();
        let rd = |i: usize| u32::from_le_bytes([wire[i], wire[i + 1], wire[i + 2], wire[i + 3]]);
        assert_eq!((rd(0), rd(4), rd(8), rd(12)), (4, 3, 4, 3));
        let note_len = rd(16) as usize;
        let mut off = 20 + note_len;
        off += off % 2;
        assert_eq!(wire.len() - off, 4 * 3 * 4 * 2);
    }

    fn image_png(w: u32, h: u32) -> Vec<u8> {
        mirror_core::encode::png(&vec![128u8; (w * h * 4) as usize], w, h).unwrap()
    }
}
