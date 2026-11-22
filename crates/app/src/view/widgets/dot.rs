use rune::{Color, El, el};

const SIZE: f32 = 8.0;

pub(crate) fn dot(color: Color) -> El {
    el().size(SIZE, SIZE)
        .radius(SIZE / 2.0)
        .fill(color)
        .a11y_hidden()
}
