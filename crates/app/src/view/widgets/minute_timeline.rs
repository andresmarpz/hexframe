use greyscreen_core::timeline::{BoxKind, MinuteBox, Multikill, ObjectiveKind, Side};
use rune::{Color, El, Icon, Shadow, TextStyle, col, el, row, text};
use rune_kit::Kit;

use super::game_icons::{BARON, DEATH, DRAGON, ELDER, GRUBS, HERALD, INHIBITOR, KILL, TOWER};
use crate::view::TABULAR;

const GAP: f32 = 2.0;
const BOX_HEIGHT: f32 = 20.0;
const BORDER: f32 = 2.0;
const RADIUS: f32 = 2.0;
const KILL_ICON: f32 = 14.0;
const OBJECTIVE_ICON: f32 = 12.0;
const STACK_WIDTH: f32 = 14.0;
const STACK_HEIGHT: f32 = 18.0;
const ICON_TOP: f32 = 1.0;
const BADGE: f32 = 8.0;
const BADGE_TOP: f32 = 10.0;
const LABEL_HEIGHT: f32 = 12.0;
const LABEL_GAP: f32 = 4.0;
const LABEL_EVERY: u32 = 5;

struct Look {
    fill: Color,
    border: Color,
    glyph: Color,
    glow: Option<Color>,
}

pub(crate) fn minute_timeline(k: &Kit, boxes: &[MinuteBox], width: f32) -> El {
    let mut strip = row()
        .w(width)
        .h(BOX_HEIGHT + LABEL_GAP + LABEL_HEIGHT)
        .gap(GAP);
    for minute in boxes {
        strip = strip.child(cell(k, minute));
    }
    strip
}

fn cell(k: &Kit, minute: &MinuteBox) -> El {
    col()
        .w(0.0)
        .grow(1.0)
        .min_w(0.0)
        .items_center()
        .child(minute_box(k, minute))
        .child(label(k, minute.minute))
}

fn label(k: &Kit, minute: u32) -> El {
    let slot = row()
        .w_full()
        .h(LABEL_HEIGHT)
        .mt(LABEL_GAP)
        .center()
        .a11y_hidden();
    if minute % LABEL_EVERY != 0 {
        return slot;
    }
    let font = TextStyle {
        size: 10.0,
        line_height: LABEL_HEIGHT,
        ..k.ty.small.features(TABULAR)
    };
    slot.child(text(minute.to_string()).font(font).color(k.c.text_faint))
}

fn look(k: &Kit, kind: BoxKind) -> Look {
    match kind {
        BoxKind::Empty => Look {
            fill: k.c.layer_02,
            border: k.c.layer_02,
            glyph: k.c.text_base,
            glow: None,
        },
        BoxKind::Kill {
            multikill: Some(Multikill::Penta),
            ..
        } => Look {
            fill: k.c.success.fg,
            border: k.c.success.bg,
            glyph: k.c.success.bg,
            glow: Some(k.c.success.fg),
        },
        BoxKind::Kill { .. } => Look {
            fill: k.c.success.bg,
            border: k.c.success.fg,
            glyph: k.c.success.fg,
            glow: None,
        },
        BoxKind::Death => Look {
            fill: k.c.danger.bg,
            border: k.c.danger.fg,
            glyph: k.c.danger.fg,
            glow: None,
        },
        BoxKind::Objective { side, .. } => Look {
            fill: k.c.layer_04,
            border: k.c.text_muted,
            glyph: match side {
                Side::Mine => k.c.text_base,
                Side::Enemy => k.c.text_muted,
            },
            glow: None,
        },
    }
}

fn glow(k: &Kit, color: Color, blur: f32, spread: f32, strength: f32) -> Shadow {
    Shadow {
        x: 0.0,
        y: 0.0,
        blur,
        spread,
        color: k.alpha(color, strength),
        inset: false,
    }
}

fn minute_box(k: &Kit, minute: &MinuteBox) -> El {
    let look = look(k, minute.kind);
    let mut shell = el()
        .key(format!("minute-{}", minute.minute))
        .w_full()
        .h(BOX_HEIGHT)
        .radius(RADIUS)
        .fill(look.fill);
    if let Some(color) = look.glow {
        shell = shell
            .box_shadow(glow(k, color, 0.0, 1.0, 0.45))
            .box_shadow(glow(k, color, 6.0, 0.0, 0.35));
    }
    if matches!(minute.kind, BoxKind::Empty) {
        return shell.a11y_hidden();
    }
    let inner = el()
        .pin(Some(0.0), Some(0.0), Some(0.0), Some(0.0))
        .radius(RADIUS)
        .clip_rounded(RADIUS)
        .a11y_hidden()
        .child(
            el().pin(Some(0.0), Some(0.0), Some(0.0), None)
                .h(BORDER)
                .fill(look.border),
        )
        .child(
            row()
                .pin(Some(0.0), Some(BORDER), Some(0.0), Some(0.0))
                .center()
                .child(content(k, minute.kind, &look)),
        );
    let shell = shell.child(inner);
    if minute.events.is_empty() {
        shell
    } else {
        shell.help(minute.events.join(" · "))
    }
}

fn content(k: &Kit, kind: BoxKind, look: &Look) -> El {
    match kind {
        BoxKind::Kill {
            multikill: Some(multikill),
            ..
        } => stacked(k, multikill, look),
        BoxKind::Kill { .. } => glyph(&KILL, KILL_ICON, look.glyph),
        BoxKind::Death => glyph(&DEATH, KILL_ICON, look.glyph),
        BoxKind::Objective { kind, .. } => glyph(objective_icon(kind), OBJECTIVE_ICON, look.glyph),
        BoxKind::Empty => el(),
    }
}

fn glyph(icon: &'static Icon, size: f32, color: Color) -> El {
    rune::icon(icon, size).color(color)
}

fn stacked(k: &Kit, multikill: Multikill, look: &Look) -> El {
    let font = TextStyle {
        size: 7.0,
        weight: 700.0,
        line_height: BADGE,
        ..k.ty.small.features(TABULAR)
    };
    let badge = row()
        .absolute((STACK_WIDTH - BADGE) / 2.0, BADGE_TOP)
        .size(BADGE, BADGE)
        .radius(BADGE / 2.0)
        .fill(look.fill)
        .center()
        .child(
            text(multikill.streak().to_string())
                .font(font)
                .color(look.glyph),
        );
    el().size(STACK_WIDTH, STACK_HEIGHT)
        .child(glyph(&KILL, KILL_ICON, look.glyph).absolute(0.0, ICON_TOP))
        .child(badge)
}

fn objective_icon(kind: ObjectiveKind) -> &'static Icon {
    match kind {
        ObjectiveKind::Dragon => &DRAGON,
        ObjectiveKind::Elder => &ELDER,
        ObjectiveKind::Baron => &BARON,
        ObjectiveKind::Herald => &HERALD,
        ObjectiveKind::Grubs => &GRUBS,
        ObjectiveKind::Tower => &TOWER,
        ObjectiveKind::Inhibitor => &INHIBITOR,
    }
}
