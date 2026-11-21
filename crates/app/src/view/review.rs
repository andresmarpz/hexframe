use rune::{El, col, text};
use rune_kit::{Button, ButtonVariant, ControlSize, Kit};

use super::page_header;
use crate::ui::{Screen, Ui};

pub(super) fn page(s: &Ui, k: &Kit) -> El {
    let subtitle = s
        .recordings
        .iter()
        .find(|r| Some(&r.meta.id) == s.review.as_ref())
        .map(|r| {
            format!(
                "{} - {}",
                r.meta.game.info.me.champion,
                crate::ui::format_date(&r.meta.started_at)
            )
        })
        .unwrap_or_default();
    let back = Button::new("Back")
        .variant(ButtonVariant::Neutral)
        .control_size(ControlSize::Large)
        .on_press(|ui: &mut Ui| ui.go(Screen::Recordings))
        .build(k)
        .key("review-back");
    col()
        .key("review")
        .w_full()
        .h_full()
        .min_h(0.0)
        .gap(16.0)
        .child(page_header(k, "Review", &subtitle, Some(back)))
        .child(
            col().w_full().grow(1.0).center().child(
                text("Review comes next")
                    .key("review-placeholder")
                    .font(k.ty.body)
                    .color(k.c.text_muted),
            ),
        )
}
