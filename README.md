# MirrorLab

A fast, private photo editor — RAW development, tone curves, colour grading and looks — built entirely in **Rust** and rendered on the **GPU**. One codebase runs in the browser, as a desktop app (macOS, Windows, Linux) and as a mobile app (Android, iOS). Photos never leave your device.

![License](https://img.shields.io/badge/license-MIT-blue.svg)
![Version](https://img.shields.io/badge/version-2.0.0-green.svg)
![Rust](https://img.shields.io/badge/Rust-2021-orange.svg)

## What's new in 2.0

MirrorLab 2.0 is a ground-up rewrite of the original React/TypeScript app.

| | 1.x (TypeScript) | 2.0 (Rust) |
|---|---|---|
| Image processing | Single-threaded JS loop, re-run per slider tick | WebGL2 shader, 60 fps at any zoom |
| Export | The 2000 px preview | Full resolution, tiled on the GPU |
| RAW | Embedded JPEG only | Real development (demosaic, white balance, colour matrix) via `rawler`, with preview fallback |
| Tone curve | Straight-line segments | Monotone cubic spline (no overshoot) |
| HSL mixer | Hard hue bins (banding) | Smoothly blended bands |
| Undo | One step per slider tick | One step per gesture |
| Zoom & pan | — | Wheel, pinch, double-tap, 1:1 |
| Crop | — | Aspect ratios, rotate, flip, straighten with auto-fit |
| Android | Capacitor wrapper of a copied codebase | Tauri 2 app with native RAW decode and gallery saving |
| Desktop | Electron shells pointing at a missing preload | Tauri 2 (≈10 MB instead of ≈150 MB) |
| Batch | — | `mirrorlab` CLI using all CPU cores |

## Features

- **Light** — exposure (in stops), contrast, highlights, shadows, whites, blacks, one-tap **Auto**
- **Color** — temperature, tint, vibrance, saturation
- **Curve** — RGB + per-channel curves over a live histogram
- **Mixer** — hue / saturation / luminance for 8 colour bands
- **Grade** — shadow / midtone / highlight colour wheels with balance
- **Effects** — clarity, dehaze, sharpen, vignette, film grain
- **Crop** — free or fixed ratios, rotate, flip, straighten
- **Looks** — 14 built-in looks with live thumbnails of your photo; save your own, import/export `.mlpreset` files (1.x presets import too)
- **Library** — open many photos, copy/paste a look, paste to all, export all
- **Compare** — before/after split, or hold `\` / long-press for the original
- **Export** — JPEG, PNG, WebP (where supported), full size or resized
- **Formats** — JPEG, PNG, WebP, HEIC (Safari/macOS), TIFF, and camera RAW: DNG, CR2, CR3, NEF, ARW, RAF, ORF, RW2, PEF, SRW, IIQ and more

### Shortcuts

| Key | Action |
|---|---|
| ⌘/Ctrl O · E | Open · Export |
| ⌘/Ctrl Z · ⇧Z | Undo · Redo |
| ⌘/Ctrl C · V | Copy · paste look |
| `\` (hold) | Show original |
| C | Crop |
| 0 · 1 · + · − | Fit · 100% · Zoom |
| ← → | Previous / next photo |
| Double-click a slider | Reset it |

## Project layout

```
crates/core   mirror-core: edit model, pipeline, curves, geometry, presets, RAW/TIFF decoding  (Rust, native + wasm)
crates/cli    mirrorlab: batch editing from the terminal                                     (Rust)
app           The editor UI: Leptos components + WebGL2 renderer                              (Rust → wasm, GLSL)
src-tauri     Native shell for desktop and mobile                                              (Rust + a little Kotlin)
```

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for how the pieces fit.

## Building

Prerequisites: [Rust](https://rustup.rs) 1.85+, and

```bash
rustup target add wasm32-unknown-unknown
cargo install trunk --locked
cargo install tauri-cli --version "^2" --locked
```

### Web

```bash
cd app
trunk serve --dist dist-dev   # http://localhost:1420
trunk build --release  # static site in app/dist
```

### Desktop

```bash
cd src-tauri
cargo tauri dev
cargo tauri build      # .app/.dmg, .msi/.exe, .deb/.AppImage/.rpm depending on the OS
```

### Android

Needs the Android SDK, NDK and Java 17.

```bash
rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android i686-linux-android
export ANDROID_HOME=~/Library/Android/sdk NDK_HOME=$ANDROID_HOME/ndk/<version>
cd src-tauri
cargo tauri android dev
cargo tauri android build --apk --target aarch64   # signed with your keystore, see Tauri docs
```

Exports are saved to **Pictures/MirrorLab** and appear in the gallery.

### iOS

```bash
cd src-tauri
cargo tauri ios init
cargo tauri ios dev
```

Exports are saved to the app's folder in the Files app. iOS support is scaffolded through Tauri but hasn't been built or tested yet.

### Command line

```bash
cargo run --release -p mirrorlab-cli -- presets
cargo run --release -p mirrorlab-cli -- edit shoot/*.NEF --preset golden-hour --set exposure=0.3 --long-edge 2048 -o out/
```

## Tests

```bash
cargo test                 # engine, CLI, native shell
cd app && trunk serve      # then in the browser console of the debug build:
                           #   await wasmBindings.mirrorlabSelfTest()
```

The self-test renders the same photo through the GPU shader and the Rust reference pipeline and reports the per-channel difference for every adjustment.

## License

MIT — see [LICENSE](LICENSE).
