/******************************************************************************
    Copyright (C) 2014 by Ruwen Hahn <palana@stunned.de>

    This program is free software: you can redistribute it and/or modify
    it under the terms of the GNU General Public License as published by
    the Free Software Foundation, either version 2 of the License, or
    (at your option) any later version.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License
    along with this program.  If not, see <http://www.gnu.org/licenses/>.
******************************************************************************/

// Ported from libobs/media-io/video-matrices.c (initialize_matrix, BT.709 studio range, 8 bit).

const KB: f32 = 0.0722;
const KR: f32 = 0.2126;
const KG: f32 = 1.0 - KB - KR;
const LUMA_MIN: f32 = 16.0;
const LUMA_MAX: f32 = 235.0;
const CHROMA_MIN: f32 = 16.0;
const CHROMA_MAX: f32 = 240.0;
const CHROMA_MID: f32 = 128.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ycbcr {
    pub y: u8,
    pub cb: u8,
    pub cr: u8,
}

pub const BLACK: Ycbcr = Ycbcr {
    y: 16,
    cb: 128,
    cr: 128,
};

pub fn bt709_studio(red: u8, green: u8, blue: u8) -> Ycbcr {
    let (r, g, b) = (
        f32::from(red) / 255.0,
        f32::from(green) / 255.0,
        f32::from(blue) / 255.0,
    );
    let luma = KR * r + KG * g + KB * b;
    let cb = (b - luma) / (2.0 * (1.0 - KB));
    let cr = (r - luma) / (2.0 * (1.0 - KR));
    Ycbcr {
        y: code(LUMA_MIN + luma * (LUMA_MAX - LUMA_MIN)),
        cb: code(CHROMA_MID + cb * (CHROMA_MAX - CHROMA_MIN)),
        cr: code(CHROMA_MID + cr * (CHROMA_MAX - CHROMA_MIN)),
    }
}

pub fn bt709_studio_over_black(red: u8, green: u8, blue: u8, alpha: u8) -> Ycbcr {
    let weigh = |channel: u8| ((u16::from(channel) * u16::from(alpha) + 127) / 255) as u8;
    bt709_studio(weigh(red), weigh(green), weigh(blue))
}

fn code(value: f32) -> u8 {
    value.round().clamp(0.0, 255.0) as u8
}
