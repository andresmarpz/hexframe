use greyscreen_core::highlight::Tone;
use rune::Color;
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
