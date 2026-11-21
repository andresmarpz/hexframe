use std::path::{Path, PathBuf};

use image::imageops::{self, FilterType};
use image::{Rgba, RgbaImage};

use crate::shape::{cover_to, encode_rgba};
use crate::{Frame, ThumbError, write_file_atomic};

const SHEET_QUALITY: u8 = 70;
const DECODE_FACTOR: u32 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SheetLayout {
    pub tile_width: u32,
    pub tile_height: u32,
    pub columns: u32,
    pub rows: u32,
}

impl SheetLayout {
    fn per_sheet(self) -> usize {
        (self.columns * self.rows) as usize
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pass {
    Finished { sheets: usize },
    Stopped,
}

pub(crate) struct Packer {
    layout: SheetLayout,
    canvas: RgbaImage,
    filled: usize,
    last: Option<RgbaImage>,
    pub(crate) written: usize,
}

impl Packer {
    pub(crate) fn new(layout: SheetLayout) -> Self {
        Self {
            layout,
            canvas: blank(layout),
            filled: 0,
            last: None,
            written: 0,
        }
    }

    fn tile(&self, frame: Option<&Frame>) -> RgbaImage {
        let SheetLayout {
            tile_width: w,
            tile_height: h,
            ..
        } = self.layout;
        let shaped = frame
            .and_then(|frame| cover_to(frame, w, h).ok())
            .map(|image| {
                if image.dimensions() == (w, h) {
                    image
                } else {
                    imageops::resize(&image, w, h, FilterType::Triangle)
                }
            });
        shaped
            .or_else(|| self.last.clone())
            .unwrap_or_else(|| RgbaImage::from_pixel(w, h, Rgba([0, 0, 0, 255])))
    }

    pub(crate) fn push(
        &mut self,
        frame: Option<&Frame>,
        dest: &dyn Fn(usize) -> PathBuf,
    ) -> Result<(), ThumbError> {
        let tile = self.tile(frame);
        let slot = self.filled as u32;
        let (x, y) = (
            slot % self.layout.columns * self.layout.tile_width,
            slot / self.layout.columns * self.layout.tile_height,
        );
        imageops::replace(&mut self.canvas, &tile, i64::from(x), i64::from(y));
        self.last = Some(tile);
        self.filled += 1;
        if self.filled == self.layout.per_sheet() {
            self.flush(dest)?;
        }
        Ok(())
    }

    pub(crate) fn flush(&mut self, dest: &dyn Fn(usize) -> PathBuf) -> Result<(), ThumbError> {
        if self.filled == 0 {
            return Ok(());
        }
        let bytes = encode_rgba(&self.canvas, SHEET_QUALITY)?;
        write_file_atomic(&dest(self.written), &bytes, false)?;
        self.written += 1;
        self.filled = 0;
        self.canvas = blank(self.layout);
        Ok(())
    }
}

fn blank(layout: SheetLayout) -> RgbaImage {
    RgbaImage::from_pixel(
        layout.columns * layout.tile_width,
        layout.rows * layout.tile_height,
        Rgba([0, 0, 0, 255]),
    )
}

pub fn write_sheets(
    video: &Path,
    times: &[f64],
    layout: SheetLayout,
    dest: &dyn Fn(usize) -> PathBuf,
    stop: &dyn Fn() -> bool,
) -> Result<Pass, ThumbError> {
    let mut packer = Packer::new(layout);
    let mut failure = None;
    let mut stopped = false;
    let mut decoded = 0usize;
    crate::keyframes(
        video,
        times,
        layout.tile_width * DECODE_FACTOR,
        &mut |_, frame| {
            if stop() {
                stopped = true;
                return false;
            }
            decoded += usize::from(frame.is_some());
            match packer.push(frame.as_ref(), dest) {
                Ok(()) => true,
                Err(err) => {
                    failure = Some(err);
                    false
                }
            }
        },
    )?;
    if let Some(err) = failure {
        return Err(err);
    }
    if stopped {
        return Ok(Pass::Stopped);
    }
    if decoded == 0 {
        return Err(ThumbError::Decode("no frame could be decoded".into()));
    }
    packer.flush(dest)?;
    Ok(Pass::Finished {
        sheets: packer.written,
    })
}
