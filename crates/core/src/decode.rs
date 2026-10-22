//! Decoding files into [`ImageF32`]: camera RAW (via `rawler`) and common
//! formats (via `image`), with EXIF orientation applied.

use std::io::Cursor;

use ::image::{DynamicImage, ImageDecoder, ImageReader};

use crate::image::ImageF32;
use crate::pipeline::s_curve;

/// File extensions treated as camera RAW.
pub const RAW_EXTENSIONS: &[&str] = &[
    "dng", "cr2", "cr3", "crw", "nef", "nrw", "arw", "srf", "sr2", "orf", "rw2", "raf", "pef", "srw", "x3f", "3fr",
    "fff", "iiq", "mos", "mef", "mrw", "erf", "kdc", "dcr", "rwl", "raw",
];

/// Extensions the editor can open.
pub const IMAGE_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "webp", "tif", "tiff", "bmp", "gif", "avif", "heic", "heif"];

pub fn extension(name: &str) -> String {
    name.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase()).unwrap_or_default()
}

pub fn is_raw(name: &str) -> bool {
    RAW_EXTENSIONS.contains(&extension(name).as_str())
}

/// Formats the platform's own image decoder may not handle (so the engine should).
pub fn needs_engine(name: &str) -> bool {
    let e = extension(name);
    is_raw(name) || e == "tif" || e == "tiff"
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    /// Fully developed from sensor data.
    Raw,
    /// RAW file we couldn't develop; used the JPEG embedded by the camera.
    RawPreview,
    Bitmap,
}

pub struct Decoded {
    pub image: ImageF32,
    pub source: Source,
    pub camera: Option<String>,
}

/// Decode any supported file. `name` is only used as a format hint.
pub fn decode(bytes: &[u8], name: &str) -> Result<Decoded, String> {
    #[cfg(feature = "raw")]
    if is_raw(name) {
        return raw::decode(bytes);
    }
    let image = bitmap(bytes)?;
    Ok(Decoded { image, source: Source::Bitmap, camera: None })
}

/// Decode a regular image with the `image` crate and apply its EXIF orientation.
pub fn bitmap(bytes: &[u8]) -> Result<ImageF32, String> {
    let reader = ImageReader::new(Cursor::new(bytes)).with_guessed_format().map_err(|e| e.to_string())?;
    let mut decoder = reader.into_decoder().map_err(|e| e.to_string())?;
    let orientation = decoder.orientation().ok();
    let mut img = DynamicImage::from_decoder(decoder).map_err(|e| e.to_string())?;
    if let Some(o) = orientation {
        img.apply_orientation(o);
    }
    Ok(from_dynamic(img))
}

pub fn from_dynamic(img: DynamicImage) -> ImageF32 {
    let (w, h) = (img.width() as usize, img.height() as usize);
    match img {
        DynamicImage::ImageRgb16(_) | DynamicImage::ImageRgba16(_) | DynamicImage::ImageLuma16(_) | DynamicImage::ImageLumaA16(_) => {
            ImageF32::from_rgb16(w, h, img.to_rgb16().as_raw())
        }
        DynamicImage::ImageRgb32F(_) | DynamicImage::ImageRgba32F(_) => ImageF32 { width: w, height: h, data: img.to_rgb32f().into_raw() },
        _ => {
            let rgb = img.to_rgb8();
            ImageF32 { width: w, height: h, data: rgb.as_raw().iter().map(|v| *v as f32 / 255.0).collect() }
        }
    }
}

/// sRGB transfer curve, extended above 1.0 so RAW highlight headroom survives.
pub fn srgb_encode(v: f32) -> f32 {
    if v <= 0.003_130_8 {
        (v * 12.92).max(0.0)
    } else {
        1.055 * v.powf(1.0 / 2.4) - 0.055
    }
}

/// Gentle base contrast for developed RAW files (cameras apply something similar to their JPEGs).
pub fn base_tone(v: f32) -> f32 {
    if v <= 1.0 {
        s_curve(v, 1.18)
    } else {
        v
    }
}

#[cfg(feature = "raw")]
mod raw {
    use super::*;
    use rawler::decoders::RawDecodeParams;
    use rawler::imgop::develop::{Intermediate, ProcessingStep, RawDevelop};
    use rawler::rawsource::RawSource;

    pub fn decode(bytes: &[u8]) -> Result<Decoded, String> {
        let attempt = || develop(bytes);
        #[cfg(not(target_arch = "wasm32"))]
        let developed = std::panic::catch_unwind(attempt).unwrap_or_else(|_| Err("RAW decoder crashed".into()));
        #[cfg(target_arch = "wasm32")]
        let developed = attempt();
        match developed {
            Ok(d) => Ok(d),
            Err(err) => embedded_preview(bytes).ok_or(err),
        }
    }

    fn develop(bytes: &[u8]) -> Result<Decoded, String> {
        let src = RawSource::new_from_slice(bytes);
        let decoder = rawler::get_decoder(&src).map_err(|e| e.to_string())?;
        let params = RawDecodeParams { image_index: 0 };
        let raw = decoder.raw_image(&src, &params, false).map_err(|e| e.to_string())?;
        let camera = Some(format!("{} {}", raw.clean_make, raw.clean_model).trim().to_string()).filter(|s| !s.is_empty());
        let (transpose, flip_h, flip_v) = raw.orientation.to_flips();
        let steps = [
            ProcessingStep::Rescale,
            ProcessingStep::Demosaic,
            ProcessingStep::FujiRotate,
            ProcessingStep::CropActiveArea,
            ProcessingStep::WhiteBalance,
            ProcessingStep::Calibrate,
            ProcessingStep::CropDefault,
        ];
        let inter = RawDevelop::new_with(&steps).develop_intermediate(&raw).map_err(|e| e.to_string())?;
        let tone = |v: f32| base_tone(srgb_encode(v.max(0.0)));
        let image = match inter {
            Intermediate::ThreeColor(px) => {
                let data = px.data.iter().flat_map(|p| [tone(p[0]), tone(p[1]), tone(p[2])]).collect();
                ImageF32 { width: px.width, height: px.height, data }
            }
            Intermediate::Monochrome(px) => {
                let data = px.data.iter().flat_map(|v| {
                    let t = tone(*v);
                    [t, t, t]
                });
                let w = px.width;
                let h = px.height;
                ImageF32 { width: w, height: h, data: data.collect() }
            }
            Intermediate::FourColor(_) => return Err("Unsupported 4-colour sensor layout".into()),
        };
        Ok(Decoded { image: image.oriented(transpose, flip_h, flip_v), source: Source::Raw, camera })
    }

    /// Largest JPEG the camera embedded in the RAW, if any.
    fn embedded_preview(bytes: &[u8]) -> Option<Decoded> {
        let src = RawSource::new_from_slice(bytes);
        let decoder = rawler::get_decoder(&src).ok()?;
        let params = RawDecodeParams { image_index: 0 };
        let img = decoder
            .full_image(&src, &params)
            .ok()
            .flatten()
            .or_else(|| decoder.preview_image(&src, &params).ok().flatten())
            .or_else(|| decoder.thumbnail_image(&src, &params).ok().flatten())?;
        Some(Decoded { image: from_dynamic(img), source: Source::RawPreview, camera: None })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extension_detection() {
        assert!(is_raw("IMG_0001.CR2"));
        assert!(is_raw("a.b.dng"));
        assert!(!is_raw("photo.jpg"));
        assert!(needs_engine("scan.TIFF"));
        assert!(!needs_engine("photo.png"));
    }

    #[test]
    fn decodes_png_bytes() {
        let mut buf = Vec::new();
        let img = ::image::RgbImage::from_fn(3, 2, |x, _| ::image::Rgb([x as u8 * 100, 0, 255]));
        img.write_to(&mut Cursor::new(&mut buf), ::image::ImageFormat::Png).unwrap();
        let d = decode(&buf, "x.png").unwrap();
        assert_eq!((d.image.width, d.image.height), (3, 2));
        assert_eq!(d.image.rgb(2, 0), [200.0 / 255.0, 0.0, 1.0]);
    }

    #[test]
    fn srgb_is_monotone_past_one() {
        assert!(srgb_encode(1.5) > srgb_encode(1.0));
        assert!((srgb_encode(1.0) - 1.0).abs() < 1e-5);
        assert_eq!(base_tone(0.5), 0.5);
    }

    #[test]
    fn rejects_garbage() {
        assert!(decode(b"not an image", "x.jpg").is_err());
        #[cfg(feature = "raw")]
        assert!(decode(b"not a raw file at all", "x.nef").is_err());
    }
}
