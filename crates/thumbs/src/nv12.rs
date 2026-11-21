use crate::Frame;

pub(crate) fn nv12_to_rgba(
    data: &[u8],
    pitch: usize,
    rows: usize,
    (x0, y0, w, h): (usize, usize, usize, usize),
    full_range: bool,
) -> Frame {
    let chroma = &data[rows * pitch..];
    let mut rgba = Vec::with_capacity(w * h * 4);
    for row in y0..y0 + h {
        for col in x0..x0 + w {
            let luma = f32::from(data[row * pitch + col]);
            let pair = (row / 2) * pitch + (col & !1);
            let u = f32::from(chroma[pair]) - 128.0;
            let v = f32::from(chroma[pair + 1]) - 128.0;
            let (y, scale) = if full_range {
                (luma, 1.0)
            } else {
                ((luma - 16.0) * 1.164_383, 1.138_393)
            };
            let r = y + 1.5748 * v * scale;
            let g = y - 0.1873 * u * scale - 0.4681 * v * scale;
            let b = y + 1.8556 * u * scale;
            rgba.extend_from_slice(&[clamp(r), clamp(g), clamp(b), 255]);
        }
    }
    Frame {
        width: w as u32,
        height: h as u32,
        rgba,
    }
}

fn clamp(value: f32) -> u8 {
    value.round().clamp(0.0, 255.0) as u8
}
