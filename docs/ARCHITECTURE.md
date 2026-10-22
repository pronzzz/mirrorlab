# Architecture

MirrorLab is one Rust workspace. The same editor runs in a browser tab, a desktop window and a phone; only a thin native shell differs.

```
┌──────────────────────────── app (Rust → WebAssembly) ────────────────────────────┐
│  Leptos UI (state.rs, ui/*)  ──signals──▶  frame loop (view.rs)  ──▶  WebGL2 (gl.rs) │
│        │                                                         develop.frag      │
│        └── io.rs: open / thumbnails / export ── platform.rs ──┐                     │
└───────────────────────────────────────────────────────────────┼─────────────────────┘
                                   mirror-core (wasm)            │  Tauri IPC (binary)
                                   decode RAW/TIFF, encode       ▼
                                                     src-tauri (native Rust, all cores)
                                                     decode_image · save_image · Android GalleryPlugin.kt
```

## mirror-core

Plain Rust with no platform code.

- `model.rs` — `Adjustments` (serde, every field defaulted so partial presets work), `Param` (one table that drives every slider and the CLI), `Geometry`.
- `pipeline.rs` — `Prepared` turns slider values into pipeline constants; `shade()` is the per-pixel function; `render()` runs it on the CPU (Rayon when the `parallel` feature is on).
- `curve.rs` — monotone cubic (Fritsch–Carlson) splines and the composite 1024-entry LUT.
- `geometry.rs` — the source → frame → view → output mapping, crop constraint solving, rotate/flip that carry the crop with them.
- `decode.rs` — `rawler` RAW development (demosaic, white balance, camera matrix, sRGB transfer with highlight headroom kept above 1.0) with embedded-preview fallback; `image` for TIFF and others; EXIF orientation.
- `presets.rs`, `auto.rs`, `histogram.rs`, `encode.rs`.

### Pipeline order

1. White balance and exposure in linear light
2. Dehaze (airlight estimate from a low-frequency luminance map)
3. Clarity (large-radius local contrast) and sharpening (1 px)
4. Highlights / shadows / whites / blacks on luminance, chroma preserving
5. Contrast (film-style S-curve around mid grey)
6. Tone curve LUT
7. Saturation, vibrance
8. HSL mixer — 8 bands, neighbouring bands blend so gradients never band
9. Colour grading — shadow / midtone / highlight tints with a movable split
10. Vignette (relative to the crop)
11. Grain (deterministic value noise, so preview and export match)

## The GPU path

`app/src/shaders/develop.frag` is a line-by-line port of `shade()` and the geometry mapping. The renderer uploads the photo once (8-bit for camera JPEGs, half-float for RAW so highlight headroom survives), builds the blur map once, and then every frame is a single full-screen draw. Zoom and pan are uniforms, so zooming in re-samples the full-resolution texture instead of scaling a bitmap.

Exports render the same shader into an off-screen framebuffer in tiles (any size), read the pixels back and encode them with the platform encoder, falling back to `mirror-core`'s encoder for images beyond the browser's canvas limit.

The debug build exposes `wasmBindings.mirrorlabSelfTest()`, which renders a test chart through both paths for every adjustment and reports the difference. Current results: mean < 0.2/255, max ≤ 3/255.

## UI

- `state.rs` — every piece of state is a `Copy` signal handle in `Editor`, provided as context. History stores one entry per finished gesture.
- `view.rs` — viewport maths, eased zoom/pan, the coalesced `requestAnimationFrame` loop and the throttled histogram.
- `ui/*` — components. Panels are recreated on tab switch with a short enter animation; sliders, wheels and curves animate resets and land on defaults with a soft detent.
- `style.css` — design tokens, neutral greys, one accent; desktop three-column grid and a mobile bottom-sheet layout chosen at 820 px.

## Native shell (src-tauri)

- `decode_image` — receives the file as a raw IPC body, decodes on a blocking thread with all cores, returns half-float pixels as a binary response.
- `save_image` — desktop: native save dialog; Android: writes to the cache and `GalleryPlugin.kt` copies it into `Pictures/MirrorLab` via MediaStore; iOS: the app's Documents folder.
- Android `MainActivity.kt` draws edge-to-edge with dark system bars and pads the web view by the system-bar, cutout and keyboard insets.
