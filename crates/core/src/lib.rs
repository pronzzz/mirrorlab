//! # mirror-core
//!
//! The engine behind MirrorLab: the edit model, tone curves, geometry, the
//! colour pipeline, presets, histograms and file decoding (including camera
//! RAW). It has no UI or platform code, so the same crate runs natively
//! (desktop, Android, iOS, the `mirrorlab` CLI) and in WebAssembly.

pub mod auto;
pub mod curve;
pub mod geometry;
pub mod histogram;
pub mod image;
pub mod model;
pub mod pipeline;
pub mod presets;

#[cfg(feature = "decode")]
pub mod decode;
#[cfg(feature = "decode")]
pub mod encode;

pub use crate::image::{ImageF32, ImageU8};
pub use model::*;
pub use pipeline::{render, Prepared, RenderOptions};
pub use presets::Preset;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
