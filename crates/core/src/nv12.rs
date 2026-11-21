const FIXED: u32 = 16;
const ONE: u32 = 1 << FIXED;

#[derive(Debug, Clone, Copy)]
pub struct Nv12Source<'a> {
    pub data: &'a [u8],
    pub pitch: usize,
    pub rows: usize,
    pub crop: (usize, usize, usize, usize),
}

pub fn frame_len(width: usize, height: usize) -> usize {
    width * height * 3 / 2
}

pub fn scale(source: &Nv12Source<'_>, width: usize, height: usize, out: &mut Vec<u8>) -> bool {
    let (x0, y0, crop_w, crop_h) = source.crop;
    let (x0, y0) = (x0 & !1, y0 & !1);
    let (width, height) = (width & !1, height & !1);
    let chroma_start = source.rows * source.pitch;
    let fits = width >= 2
        && height >= 2
        && crop_w >= 2
        && crop_h >= 2
        && x0 + crop_w <= source.pitch
        && y0 + crop_h <= source.rows
        && source.data.len() >= chroma_start + source.pitch * source.rows / 2;
    if !fits {
        return false;
    }
    out.clear();
    out.resize(frame_len(width, height), 0);
    let (luma_out, chroma_out) = out.split_at_mut(width * height);
    let luma = &source.data[..chroma_start];
    let chroma = &source.data[chroma_start..];
    resample(
        &Plane {
            data: luma,
            pitch: source.pitch,
            x0,
            y0,
            width: crop_w,
            height: crop_h,
            step: 1,
        },
        luma_out,
        width,
        height,
    );
    resample(
        &Plane {
            data: chroma,
            pitch: source.pitch,
            x0: x0 / 2,
            y0: y0 / 2,
            width: crop_w / 2,
            height: crop_h / 2,
            step: 2,
        },
        chroma_out,
        width / 2,
        height / 2,
    );
    true
}

struct Plane<'a> {
    data: &'a [u8],
    pitch: usize,
    x0: usize,
    y0: usize,
    width: usize,
    height: usize,
    step: usize,
}

struct Tap {
    low: usize,
    high: usize,
    weight: u32,
}

fn taps(source: usize, target: usize) -> Vec<Tap> {
    let ratio = (source as u64) << FIXED;
    (0..target)
        .map(|index| {
            let centre =
                ((2 * index as u64 + 1) * ratio / (2 * target as u64)) as i64 - i64::from(ONE / 2);
            let centre = centre.clamp(0, ((source as i64 - 1) << FIXED).max(0));
            let low = (centre >> FIXED) as usize;
            Tap {
                low,
                high: (low + 1).min(source - 1),
                weight: (centre as u32) & (ONE - 1),
            }
        })
        .collect()
}

fn lerp(a: u32, b: u32, weight: u32) -> u32 {
    (a * (ONE - weight) + b * weight + ONE / 2) >> FIXED
}

fn resample(plane: &Plane<'_>, out: &mut [u8], width: usize, height: usize) {
    if plane.width == width && plane.height == height {
        for row in 0..height {
            for col in 0..width {
                for lane in 0..plane.step {
                    out[(row * width + col) * plane.step + lane] = plane.data
                        [(plane.y0 + row) * plane.pitch + (plane.x0 + col) * plane.step + lane];
                }
            }
        }
        return;
    }
    let columns = taps(plane.width, width);
    for (row, tap) in taps(plane.height, height).iter().enumerate() {
        let top = (plane.y0 + tap.low) * plane.pitch;
        let bottom = (plane.y0 + tap.high) * plane.pitch;
        for (col, column) in columns.iter().enumerate() {
            for lane in 0..plane.step {
                let at = |base: usize, x: usize| {
                    u32::from(plane.data[base + (plane.x0 + x) * plane.step + lane])
                };
                let upper = lerp(at(top, column.low), at(top, column.high), column.weight);
                let lower = lerp(
                    at(bottom, column.low),
                    at(bottom, column.high),
                    column.weight,
                );
                out[(row * width + col) * plane.step + lane] = lerp(upper, lower, tap.weight) as u8;
            }
        }
    }
}

#[cfg(test)]
mod tests;
