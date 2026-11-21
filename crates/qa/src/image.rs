use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

#[derive(Debug, thiserror::Error)]
pub enum ImageError {
    #[error("{path}: {source}")]
    Io {
        path: String,
        source: std::io::Error,
    },
    #[error("{path}: {source}")]
    Decode {
        path: String,
        source: png::DecodingError,
    },
    #[error("{path}: {source}")]
    Encode {
        path: String,
        source: png::EncodingError,
    },
    #[error("{path}: unsupported PNG layout {layout}")]
    Layout { path: String, layout: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rgba {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

impl Rgba {
    pub fn read(path: &Path) -> Result<Rgba, ImageError> {
        let shown = || path.display().to_string();
        let file = File::open(path).map_err(|source| ImageError::Io {
            path: shown(),
            source,
        })?;
        let mut decoder = png::Decoder::new(std::io::BufReader::new(file));
        decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
        let mut reader = decoder.read_info().map_err(|source| ImageError::Decode {
            path: shown(),
            source,
        })?;
        let size = reader.output_buffer_size().unwrap_or(0);
        let mut buf = vec![0; size];
        let info = reader
            .next_frame(&mut buf)
            .map_err(|source| ImageError::Decode {
                path: shown(),
                source,
            })?;
        buf.truncate(info.buffer_size());
        let pixels = match info.color_type {
            png::ColorType::Rgba => buf,
            png::ColorType::Rgb => buf
                .as_chunks::<3>()
                .0
                .iter()
                .flat_map(|[r, g, b]| [*r, *g, *b, 255])
                .collect(),
            other => {
                return Err(ImageError::Layout {
                    path: shown(),
                    layout: format!("{other:?}"),
                });
            }
        };
        Ok(Rgba {
            width: info.width,
            height: info.height,
            pixels,
        })
    }

    pub fn write(&self, path: &Path) -> Result<(), ImageError> {
        let shown = || path.display().to_string();
        let file = File::create(path).map_err(|source| ImageError::Io {
            path: shown(),
            source,
        })?;
        let mut encoder = png::Encoder::new(BufWriter::new(file), self.width, self.height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let encode = |source| ImageError::Encode {
            path: shown(),
            source,
        };
        let mut writer = encoder.write_header().map_err(encode)?;
        writer.write_image_data(&self.pixels).map_err(encode)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Diff {
    pub same_size: bool,
    pub changed: usize,
    pub total: usize,
    pub image: Option<Rgba>,
}

impl Diff {
    pub fn ratio(&self) -> f64 {
        if self.total == 0 {
            return 0.0;
        }
        self.changed as f64 / self.total as f64
    }

    pub fn within(&self, max_ratio: f64) -> bool {
        self.same_size && self.ratio() <= max_ratio
    }
}

pub fn compare(golden: &Rgba, actual: &Rgba, tolerance: u8) -> Diff {
    if golden.width != actual.width || golden.height != actual.height {
        return Diff {
            same_size: false,
            changed: 0,
            total: 0,
            image: None,
        };
    }
    let mut image = Vec::with_capacity(golden.pixels.len());
    let mut changed = 0;
    let pairs = golden
        .pixels
        .as_chunks::<4>()
        .0
        .iter()
        .zip(actual.pixels.as_chunks::<4>().0);
    for (g, a) in pairs {
        let delta = g
            .iter()
            .zip(a)
            .map(|(x, y)| x.abs_diff(*y))
            .max()
            .unwrap_or(0);
        if delta > tolerance {
            changed += 1;
            image.extend_from_slice(&[255, 32, 64, 255]);
        } else {
            let grey = ((u16::from(g[0]) + u16::from(g[1]) + u16::from(g[2])) / 3 / 3) as u8;
            image.extend_from_slice(&[grey, grey, grey, 255]);
        }
    }
    Diff {
        same_size: true,
        changed,
        total: golden.pixels.len() / 4,
        image: Some(Rgba {
            width: golden.width,
            height: golden.height,
            pixels: image,
        }),
    }
}

#[cfg(test)]
mod tests;
