#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScreenRect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl ScreenRect {
    pub fn width(&self) -> i64 {
        i64::from(self.right) - i64::from(self.left)
    }

    pub fn height(&self) -> i64 {
        i64::from(self.bottom) - i64::from(self.top)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CropRect {
    pub left: u32,
    pub top: u32,
    pub width: u32,
    pub height: u32,
}

impl CropRect {
    pub fn full(frame: (u32, u32)) -> Self {
        Self {
            left: 0,
            top: 0,
            width: frame.0,
            height: frame.1,
        }
    }

    pub fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    pub fn is_full(&self, frame: (u32, u32)) -> bool {
        *self == Self::full(frame)
    }
}

const EDGE_SLACK: i64 = 1;

pub fn client_crop(
    frame: (u32, u32),
    frame_bounds: ScreenRect,
    client_origin: (i32, i32),
    client_size: (i32, i32),
) -> CropRect {
    let full = CropRect::full(frame);
    let (bounds_w, bounds_h) = (frame_bounds.width(), frame_bounds.height());
    let (client_w, client_h) = (i64::from(client_size.0), i64::from(client_size.1));
    if frame.0 == 0
        || frame.1 == 0
        || bounds_w <= 0
        || bounds_h <= 0
        || client_w <= 0
        || client_h <= 0
    {
        return full;
    }
    let offset_x = i64::from(client_origin.0) - i64::from(frame_bounds.left);
    let offset_y = i64::from(client_origin.1) - i64::from(frame_bounds.top);
    let (frame_w, frame_h) = (i64::from(frame.0), i64::from(frame.1));
    let left = scale(offset_x, frame_w, bounds_w);
    let top = scale(offset_y, frame_h, bounds_h);
    let right = scale(offset_x + client_w, frame_w, bounds_w);
    let bottom = scale(offset_y + client_h, frame_h, bounds_h);
    let outside = left < -EDGE_SLACK
        || top < -EDGE_SLACK
        || right > frame_w + EDGE_SLACK
        || bottom > frame_h + EDGE_SLACK;
    let (left, top) = (left.max(0), top.max(0));
    let (right, bottom) = (right.min(frame_w), bottom.min(frame_h));
    if outside || right <= left || bottom <= top {
        return full;
    }
    if left == 0 && top == 0 && right == frame_w && bottom == frame_h {
        return full;
    }
    let width = (right - left) & !1;
    let height = (bottom - top) & !1;
    if width == 0 || height == 0 {
        return full;
    }
    CropRect {
        left: left as u32,
        top: top as u32,
        width: width as u32,
        height: height as u32,
    }
}

pub fn fit_crop(crop: CropRect, frame: (u32, u32)) -> CropRect {
    let fits = crop.width > 0
        && crop.height > 0
        && crop.left.saturating_add(crop.width) <= frame.0
        && crop.top.saturating_add(crop.height) <= frame.1;
    if fits { crop } else { CropRect::full(frame) }
}

fn scale(value: i64, numerator: i64, denominator: i64) -> i64 {
    (2 * value * numerator + denominator).div_euclid(2 * denominator)
}

#[cfg(test)]
mod tests;
