use super::*;

fn codes(y: u8, cb: u8, cr: u8) -> Ycbcr {
    Ycbcr { y, cb, cr }
}

#[test]
fn black_is_the_studio_floor() {
    assert_eq!(bt709_studio(0, 0, 0), codes(16, 128, 128));
    assert_eq!(bt709_studio(0, 0, 0), BLACK);
}

#[test]
fn white_is_the_studio_ceiling_with_neutral_chroma() {
    assert_eq!(bt709_studio(255, 255, 255), codes(235, 128, 128));
}

#[test]
fn mid_grey_sits_between_with_neutral_chroma() {
    assert_eq!(bt709_studio(128, 128, 128), codes(126, 128, 128));
}

#[test]
fn red_has_low_luma_and_peak_cr() {
    assert_eq!(bt709_studio(255, 0, 0), codes(63, 102, 240));
}

#[test]
fn blue_has_lowest_luma_and_peak_cb() {
    assert_eq!(bt709_studio(0, 0, 255), codes(32, 240, 118));
}

#[test]
fn green_has_high_luma_and_low_chroma() {
    assert_eq!(bt709_studio(0, 255, 0), codes(173, 42, 26));
}

#[test]
fn half_transparent_white_equals_half_white_over_black() {
    assert_eq!(
        bt709_studio_over_black(255, 255, 255, 128),
        bt709_studio(128, 128, 128)
    );
}

#[test]
fn zero_alpha_over_black_is_black() {
    assert_eq!(bt709_studio_over_black(200, 10, 90, 0), BLACK);
}
