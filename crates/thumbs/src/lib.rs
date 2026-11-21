#[cfg(any(windows, test))]
mod nv12;
mod shape;
#[cfg(test)]
mod tests;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(windows)]
mod windows;

#[cfg(target_os = "macos")]
use macos as backend;
#[cfg(windows)]
use windows as backend;

use std::fs;
use std::path::Path;

pub use shape::{THUMB_HEIGHT, THUMB_WIDTH};

const DECODE_WIDTH: u32 = THUMB_WIDTH * 2;
const JPEG_QUALITY: u8 = 80;

#[derive(Debug, thiserror::Error)]
pub enum ThumbError {
    #[error("thumbnails are not supported on this platform")]
    Unsupported,
    #[error("cannot decode {0}")]
    Decode(String),
    #[error("cannot write the thumbnail: {0}")]
    Write(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

#[cfg(not(any(target_os = "macos", windows)))]
mod backend {
    use std::path::Path;

    use super::{Frame, ThumbError};

    pub(super) fn grab(_: &Path, _: f64, _: u32) -> Result<Frame, ThumbError> {
        Err(ThumbError::Unsupported)
    }
}

pub fn grab(video: &Path, at_secs: f64, max_width: u32) -> Result<Frame, ThumbError> {
    backend::grab(video, at_secs.max(0.0), max_width)
}

pub fn write_thumbnail(video: &Path, at_secs: f64, dest: &Path) -> Result<(), ThumbError> {
    let frame = grab(video, at_secs, DECODE_WIDTH)?;
    let bytes = shape::encode_jpeg(&frame, JPEG_QUALITY)?;
    write_atomic(dest, &bytes)
}

pub fn write_jpeg(frame: &Frame, dest: &Path) -> Result<(), ThumbError> {
    write_atomic(dest, &shape::encode_jpeg(frame, JPEG_QUALITY)?)
}

fn write_atomic(dest: &Path, bytes: &[u8]) -> Result<(), ThumbError> {
    let write = |err: std::io::Error| ThumbError::Write(err.to_string());
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).map_err(write)?;
    }
    let partial = dest.with_extension("jpg.part");
    fs::write(&partial, bytes).map_err(write)?;
    fs::rename(&partial, dest).map_err(write)
}
