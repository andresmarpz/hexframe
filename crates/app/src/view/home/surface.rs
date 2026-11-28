use rune::{El, Shadow, TextStyle, col, row, text};
use rune_kit::Kit;

use crate::view::TABULAR;

const RADIUS: f32 = 12.0;

pub(super) fn surface(k: &Kit) -> El {
    let soft = k
        .elevation
        .md
        .first()
        .map_or(k.c.black.alpha(0.2), |shadow| shadow.color);
    let shadow = |y: f32, blur: f32, spread: f32, color, inset| Shadow {
        x: 0.0,
        y,
        blur,
        spread,
        color,
        inset,
    };
    col()
        .w_full()
        .radius(RADIUS)
        .fill(k.c.bg_base)
        .box_shadow(shadow(0.5, 0.0, 0.0, k.light(0.06), true))
        .box_shadow(shadow(2.0, 4.0, 0.0, soft, false))
        .box_shadow(shadow(1.0, 2.0, 0.0, soft, false))
        .box_shadow(shadow(0.0, 0.0, 0.5, k.light(0.16), false))
}

pub(super) fn header(k: &Kit, title: &str, aside: Option<String>) -> El {
    let mut header = row().w_full().px(16.0).center_y().justify_between().child(
        text(title.to_owned())
            .font(sized(k.ty.ui_medium, 13.0, 16.0))
            .color(k.c.text_base),
    );
    if let Some(aside) = aside {
        header = header.child(
            text(aside)
                .font(sized(k.ty.ui, 12.0, 16.0).features(TABULAR))
                .color(k.c.text_muted),
        );
    }
    header
}

pub(super) fn sized(base: TextStyle, size: f32, line_height: f32) -> TextStyle {
    TextStyle {
        size,
        line_height,
        ..base
    }
}

pub(super) fn plural(count: usize, one: &str, many: &str) -> String {
    format!("{count} {}", if count == 1 { one } else { many })
}
