use image::codecs::jpeg::JpegEncoder;
use image::imageops::{self, FilterType};
use image::{ExtendedColorType, RgbaImage};

use crate::{Frame, ThumbError};

pub const THUMB_WIDTH: u32 = 400;
pub const THUMB_HEIGHT: u32 = 225;

pub(crate) fn encode_jpeg(frame: &Frame, quality: u8) -> Result<Vec<u8>, ThumbError> {
    encode_rgba(&cover(frame)?, quality)
}

pub(crate) fn encode_rgba(image: &RgbaImage, quality: u8) -> Result<Vec<u8>, ThumbError> {
    let rgb: Vec<u8> = image
        .pixels()
        .flat_map(|p| [p.0[0], p.0[1], p.0[2]])
        .collect();
    let mut out = Vec::new();
    JpegEncoder::new_with_quality(&mut out, quality)
        .encode(&rgb, image.width(), image.height(), ExtendedColorType::Rgb8)
        .map_err(|err| ThumbError::Write(err.to_string()))?;
    Ok(out)
}

pub(crate) fn cover(frame: &Frame) -> Result<RgbaImage, ThumbError> {
    cover_to(frame, THUMB_WIDTH, THUMB_HEIGHT)
}

pub(crate) fn cover_to(frame: &Frame, width: u32, height: u32) -> Result<RgbaImage, ThumbError> {
    let image = RgbaImage::from_raw(frame.width, frame.height, frame.rgba.clone())
        .ok_or_else(|| ThumbError::Decode("frame size does not match its bytes".into()))?;
    let (x, y, w, h) = crop_16_9(frame.width, frame.height);
    let cropped = imageops::crop_imm(&image, x, y, w, h).to_image();
    if w <= width {
        return Ok(cropped);
    }
    Ok(imageops::resize(
        &cropped,
        width,
        height,
        FilterType::Triangle,
    ))
}

pub(crate) fn crop_16_9(width: u32, height: u32) -> (u32, u32, u32, u32) {
    let wide = u64::from(width) * 9 > u64::from(height) * 16;
    if wide {
        let w = (u64::from(height) * 16 / 9) as u32;
        ((width - w) / 2, 0, w.max(1), height)
    } else {
        let h = (u64::from(width) * 9 / 16) as u32;
        (0, (height - h) / 2, width, h.max(1))
    }
}
