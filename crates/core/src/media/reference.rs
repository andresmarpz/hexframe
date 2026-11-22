/*
 * Copyright (c) 2020 Hans Petter Selasky <hps@selasky.org>
 *
 * Permission to use, copy, modify, and distribute this software for any
 * purpose with or without fee is hereby granted, provided that the above
 * copyright notice and this permission notice appear in all copies.
 *
 * THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES
 * WITH REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF
 * MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR
 * ANY SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES
 * WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS, WHETHER IN AN
 * ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING OUT OF
 * OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THIS SOFTWARE.
 */

/******************************************************************************
    Copyright (C) 2023 by Lain Bailey <lain@obsproject.com>

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

// Ported from libobs/util/util_uint64.h (util_mul_div64, portable branch)
pub(super) fn mul_div64_split(num: u64, mul: u64, div: u64) -> u64 {
    let rem = num % div;
    (num / div)
        .wrapping_mul(mul)
        .wrapping_add(rem.wrapping_mul(mul) / div)
}

// Ported from libobs/util/util_uint64.h (util_mul_div64, 128-bit branch)
pub(super) fn mul_div64_wide(num: u64, mul: u64, div: u64) -> u64 {
    (u128::from(num) * u128::from(mul) / u128::from(div)) as u64
}

// Ported from libobs/media-io/audio-io.h (audio_frames_to_ns)
pub(super) fn audio_frames_to_ns(sample_rate: u64, frames: u64) -> u64 {
    mul_div64_wide(frames, 1_000_000_000, sample_rate)
}
