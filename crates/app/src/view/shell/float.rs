use rune::{El, Shadow};
use rune_kit::Kit;

use crate::view::layout::edge;

pub(super) fn floating(surface: El, k: &Kit) -> El {
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
    surface
        .fill(k.c.bg_base)
        .box_shadow(shadow(0.5, 0.0, 0.0, k.light(0.06), true))
        .box_shadow(shadow(6.0, 16.0, 0.0, soft, false))
        .box_shadow(shadow(2.0, 4.0, 0.0, soft, false))
        .box_shadow(shadow(0.0, 0.0, 0.5, edge(k), false))
}
