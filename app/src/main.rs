//! MirrorLab: a fast, private photo editor.
//!
//! The UI is Rust (Leptos) compiled to WebAssembly; pixels are developed on
//! the GPU with WebGL2; decoding and file formats come from `mirror-core`.

mod app;
mod gl;
mod icons;
mod io;
mod platform;
#[cfg(debug_assertions)]
mod selftest;
mod state;
mod ui;
mod view;

fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(app::App);
}
