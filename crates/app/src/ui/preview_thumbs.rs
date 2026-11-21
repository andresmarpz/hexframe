use std::fs;
use std::path::{Path, PathBuf};

use greyscreen_engine::RecordingEntry;
use greyscreen_thumbs::{Frame, write_jpeg};

const WIDTH: u32 = 640;
const HEIGHT: u32 = 360;

const PALETTES: [([u8; 3], [u8; 3]); 6] = [
    ([38, 64, 52], [88, 128, 74]),
    ([52, 44, 82], [120, 86, 112]),
    ([30, 58, 84], [64, 118, 128]),
    ([74, 52, 40], [150, 112, 70]),
    ([36, 40, 68], [74, 98, 140]),
    ([58, 70, 50], [112, 120, 82]),
];

pub(super) fn dir() -> PathBuf {
    std::env::temp_dir().join("greyscreen-preview-thumbs-1")
}

pub(super) fn write(dest: &Path, seed: u32) {
    if dest.is_file() {
        return;
    }
    if let Some(parent) = dest.parent()
        && fs::create_dir_all(parent).is_err()
    {
        return;
    }
    if let Err(err) = write_jpeg(&frame(seed), dest) {
        tracing::debug!(%err, "no preview thumbnail");
    }
}

pub(in crate::ui) fn copy_playable(folder: &Path, rec: &RecordingEntry) {
    let files = std::iter::once(rec.meta.video.clone())
        .chain(rec.meta.clips.iter().map(|c| c.file.clone()));
    let sample = Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/review.mp4");
    for file in files {
        let dest = folder.join(file);
        if dest.is_file() {
            continue;
        }
        if let Some(parent) = dest.parent()
            && fs::create_dir_all(parent).is_err()
        {
            return;
        }
        if let Err(err) = fs::copy(&sample, &dest) {
            tracing::debug!(%err, "no preview video");
        }
    }
}

struct Noise(u32);

impl Noise {
    fn next(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        self.0 >> 8
    }

    fn below(&mut self, limit: u32) -> u32 {
        self.next() % limit.max(1)
    }
}

fn frame(seed: u32) -> Frame {
    let mut noise = Noise(seed.wrapping_mul(2_654_435_761).wrapping_add(7));
    let (top, bottom) = PALETTES[noise.below(PALETTES.len() as u32) as usize];
    let mut rgba = vec![255u8; (WIDTH * HEIGHT * 4) as usize];
    let mut paint = |x: i64, y: i64, rgb: [u8; 3], alpha: f32| {
        if x < 0 || y < 0 || x >= i64::from(WIDTH) || y >= i64::from(HEIGHT) {
            return;
        }
        let at = ((y as u32 * WIDTH + x as u32) * 4) as usize;
        for (slot, value) in rgba[at..at + 3].iter_mut().zip(rgb) {
            *slot = (f32::from(*slot) * (1.0 - alpha) + f32::from(value) * alpha) as u8;
        }
    };
    for y in 0..HEIGHT {
        let t = y as f32 / HEIGHT as f32;
        for x in 0..WIDTH {
            let shade = 1.0 - 0.35 * ((x as f32 / WIDTH as f32) - 0.5).abs();
            let mix = |a: u8, b: u8| ((f32::from(a) * (1.0 - t) + f32::from(b) * t) * shade) as u8;
            let rgb = [
                mix(top[0], bottom[0]),
                mix(top[1], bottom[1]),
                mix(top[2], bottom[2]),
            ];
            paint(i64::from(x), i64::from(y), rgb, 1.0);
        }
    }
    for _ in 0..4 {
        let (cx, cy) = (
            i64::from(noise.below(WIDTH)),
            i64::from(60 + noise.below(HEIGHT - 150)),
        );
        let reach = 40 + noise.below(50) as i64;
        for y in -reach..reach {
            for x in -reach..reach {
                let d = ((x * x + y * y) as f32).sqrt() / reach as f32;
                if d < 1.0 {
                    paint(cx + x, cy + y, [255, 244, 214], 0.16 * (1.0 - d));
                }
            }
        }
    }
    for team in 0..2 {
        let ink = if team == 0 {
            [84, 160, 255]
        } else {
            [255, 98, 98]
        };
        for _ in 0..3 {
            let (cx, cy) = (
                i64::from(40 + noise.below(WIDTH - 80)),
                i64::from(50 + noise.below(HEIGHT - 150)),
            );
            let radius = 11 + noise.below(6) as i64;
            for y in -radius..=radius {
                for x in -radius..=radius {
                    let d = ((x * x + y * y) as f32).sqrt();
                    if d <= radius as f32 {
                        let ring = d > radius as f32 - 3.0;
                        paint(cx + x, cy + y, if ring { ink } else { [24, 28, 36] }, 0.9);
                    }
                }
            }
        }
    }
    for y in i64::from(HEIGHT) - 52..i64::from(HEIGHT) {
        for x in 0..i64::from(WIDTH) {
            paint(x, y, [10, 12, 18], 0.82);
        }
    }
    for slot in 0..6 {
        let left = 196 + slot * 40;
        for y in 0..30 {
            for x in 0..30 {
                paint(left + x, i64::from(HEIGHT) - 42 + y, [70, 78, 98], 0.9);
            }
        }
    }
    Frame {
        width: WIDTH,
        height: HEIGHT,
        rgba,
    }
}
