use greyscreen_core::highlight::Tone;
use rune::{Color, El, el, row, text};
use rune_kit::Kit;

pub(crate) fn tone_color(k: &Kit, tone: Tone) -> Color {
    match tone {
        Tone::Big => k.c.warning.icon,
        Tone::Death => k.c.danger.icon,
        Tone::Kill => k.c.info.icon,
        Tone::Team => k.c.success.icon,
        Tone::Enemy => k.c.danger.fg,
        Tone::Neutral => k.c.icon_faint,
    }
}

pub(crate) fn status_pill(
    k: &Kit,
    label: impl Into<String>,
    bg: Color,
    fg: Color,
    dot_color: Option<Color>,
) -> El {
    let mut pill = row()
        .h(22.0)
        .px(10.0)
        .gap(6.0)
        .center_y()
        .radius(11.0)
        .fill(bg);
    if let Some(c) = dot_color {
        pill = pill.child(el().size(7.0, 7.0).radius(3.5).fill(c).a11y_hidden());
    }
    pill.child(text(label.into()).font(k.ty.small_medium).color(fg))
}

pub(crate) fn state_pill(
    k: &Kit,
    label: impl Into<String>,
    colors: rune_kit::StateColors,
    with_dot: bool,
) -> El {
    status_pill(
        k,
        label,
        colors.bg,
        colors.fg,
        with_dot.then_some(colors.icon),
    )
}

pub(crate) fn tone_pill(k: &Kit, label: impl Into<String>, tone: Tone) -> El {
    match tone {
        Tone::Big => state_pill(k, label, k.c.warning, false),
        Tone::Death | Tone::Enemy => state_pill(k, label, k.c.danger, false),
        Tone::Kill => state_pill(k, label, k.c.info, false),
        Tone::Team => state_pill(k, label, k.c.success, false),
        Tone::Neutral => status_pill(k, label, k.c.layer_02, k.c.text_muted, None),
    }
}
