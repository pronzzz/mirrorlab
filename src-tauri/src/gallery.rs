//! Saving exports on mobile.
//!
//! Android: a small Kotlin plugin (`GalleryPlugin.kt`) copies the file into
//! the shared Pictures/MirrorLab folder through MediaStore, so it shows up in
//! the Photos app. iOS: files go to the app's Documents folder, which is
//! visible in the Files app.

use tauri::plugin::{Builder, TauriPlugin};
use tauri::Runtime;

#[cfg(target_os = "android")]
use tauri::Manager;

#[cfg(target_os = "android")]
pub struct Gallery<R: Runtime>(tauri::plugin::PluginHandle<R>);

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("gallery")
        .setup(|_app, _api| {
            #[cfg(target_os = "android")]
            {
                let handle = _api.register_android_plugin("app.mirrorlab.editor", "GalleryPlugin")?;
                _app.manage(Gallery(handle));
            }
            Ok(())
        })
        .build()
}

#[cfg(target_os = "android")]
pub fn save<R: Runtime>(app: &tauri::AppHandle<R>, bytes: &[u8], name: &str, mime: &str) -> Result<String, String> {
    #[derive(serde::Serialize)]
    struct Args<'a> {
        path: String,
        name: &'a str,
        mime: &'a str,
    }
    #[derive(serde::Deserialize)]
    struct Saved {
        location: String,
    }
    let dir = app.path().app_cache_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let tmp = dir.join(format!("export-{}", name));
    std::fs::write(&tmp, bytes).map_err(|e| e.to_string())?;
    let g = app.state::<Gallery<R>>();
    let res: Saved = g
        .0
        .run_mobile_plugin("saveToGallery", Args { path: tmp.to_string_lossy().into_owned(), name, mime })
        .map_err(|e| e.to_string())?;
    Ok(format!("Saved to {}", res.location))
}

#[cfg(target_os = "ios")]
pub fn save<R: Runtime>(app: &tauri::AppHandle<R>, bytes: &[u8], name: &str, _mime: &str) -> Result<String, String> {
    use tauri::Manager;
    let dir = app.path().document_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    std::fs::write(dir.join(name), bytes).map_err(|e| e.to_string())?;
    Ok("Saved to Files › MirrorLab".into())
}
