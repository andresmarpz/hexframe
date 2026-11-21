use rune::{Color, El, LoopSpec, el};

const SIZE: f32 = 8.0;
const PULSE_SECS: f64 = 1.4;
const PULSE_LOW: f32 = 0.25;

pub(crate) fn dot(color: Color) -> El {
    el().size(SIZE, SIZE)
        .radius(SIZE / 2.0)
        .fill(color)
        .a11y_hidden()
}

pub(crate) fn pulsing_dot(color: Color) -> El {
    dot(color).looping(LoopSpec::opacity(PULSE_SECS, 0.0, PULSE_LOW, 1.0))
}
