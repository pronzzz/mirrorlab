//! JPEG / PNG encoding of RGBA8 buffers (used when the platform encoder isn't available).

use std::io::Cursor;

use ::image::codecs::jpeg::JpegEncoder;
use ::image::codecs::png::PngEncoder;
use ::image::{ExtendedColorType, ImageEncoder};

fn rgb(rgba: &[u8]) -> Vec<u8> {
    rgba.chunks_exact(4).flat_map(|p| [p[0], p[1], p[2]]).collect()
}

pub fn jpeg(rgba: &[u8], w: u32, h: u32, quality: u8) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    JpegEncoder::new_with_quality(&mut out, quality.clamp(1, 100))
        .write_image(&rgb(rgba), w, h, ExtendedColorType::Rgb8)
        .map_err(|e| e.to_string())?;
    Ok(out)
}

pub fn png(rgba: &[u8], w: u32, h: u32) -> Result<Vec<u8>, String> {
    let mut out = Cursor::new(Vec::new());
    PngEncoder::new(&mut out).write_image(&rgb(rgba), w, h, ExtendedColorType::Rgb8).map_err(|e| e.to_string())?;
    Ok(out.into_inner())
}

#[cfg(test)]
mod tests {
    #[test]
    fn encodes_and_decodes() {
        let px = vec![200u8; 4 * 6 * 4];
        let j = super::jpeg(&px, 6, 4, 90).unwrap();
        assert_eq!(&j[..2], &[0xFF, 0xD8]);
        let p = super::png(&px, 6, 4).unwrap();
        let back = crate::decode::bitmap(&p).unwrap();
        assert_eq!((back.width, back.height), (6, 4));
    }
}
