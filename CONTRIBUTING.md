# Contributing

Thanks for helping improve MirrorLab!

1. Fork and create a branch.
2. `cargo test` must pass, and `cargo clippy --workspace --all-targets` should be clean.
3. If you change the pipeline maths, change **both** `crates/core/src/pipeline.rs` and `app/src/shaders/develop.frag`, then run `await wasmBindings.mirrorlabSelfTest()` in a debug build — every case should say `ok`.
4. Keep the UI minimal: one accent colour, icons from `app/src/icons.rs` (24 px grid, 1.6 px stroke), motion through the CSS tokens in `style.css`, and respect `prefers-reduced-motion`.
5. Open a pull request describing what changed and how you tested it.
