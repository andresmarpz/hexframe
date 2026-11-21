use greyscreen_core::encoder::{FAILED_HELP, FAILED_TITLE};
use greyscreen_core::problem::Problem;
use rune::{El, col, row, text};
use rune_kit::{Button, ButtonVariant, ControlSize, Kit};

use crate::ui::Ui;
use crate::view::card;

pub(super) fn error_card(s: &Ui, k: &Kit) -> Option<El> {
    let failed = s.encoder.as_ref().is_some_and(|check| check.failed());
    failed.then(|| {
        let faq = Button::new("Read the FAQ")
            .variant(ButtonVariant::Neutral)
            .control_size(ControlSize::Large)
            .on_press_tx(|_: &mut Ui, tx: &mut rune::Tx| {
                if let Err(err) = tx.open_url(&Problem::NoEncoder.help_url()) {
                    tracing::warn!(%err, "cannot open the FAQ");
                }
            })
            .build(k)
            .key("encoder-faq");
        let feedback = Button::new("Send feedback")
            .variant(ButtonVariant::Outline)
            .control_size(ControlSize::Large)
            .on_press_tx(|ui: &mut Ui, tx: &mut rune::Tx| ui.open_feedback(tx))
            .build(k)
            .key("encoder-feedback");
        card(k)
            .key("encoder-error")
            .padding(20.0)
            .gap(14.0)
            .border(1.0, k.c.danger.icon)
            .child(
                col()
                    .gap(4.0)
                    .child(
                        text(FAILED_TITLE)
                            .key("encoder-error-title")
                            .font(k.ty.body_medium)
                            .color(k.c.danger.fg),
                    )
                    .child(
                        text(FAILED_HELP)
                            .wrap()
                            .w_full()
                            .font(k.ty.body)
                            .color(k.c.text_muted),
                    ),
            )
            .child(row().gap(8.0).child(faq).child(feedback))
    })
}
