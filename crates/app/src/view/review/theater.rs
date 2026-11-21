use std::f32::consts::PI;

use rune::{Color, El, Tx, col, el, row, text};
use rune_kit::{ControlSize, IconButton, IconButtonVariant, Kit};

use super::{icons, timeline, video_frame};
use crate::ui::{ReviewState, Ui, format_clock};

const BAR_PAD_X: f32 = 24.0;
const BAR_PAD_TOP: f32 = 64.0;
const BAR_PAD_BOTTOM: f32 = 16.0;
const SCRIM_ALPHA: f32 = 0.88;
const MOVE_THROTTLE_SECS: f64 = 0.2;

pub(crate) fn screen(review: &ReviewState) -> El {
    let k = Kit::neutral(true);
    let mut root = el()
        .key("theater")
        .w_full()
        .h_full()
        .fill(Color::BLACK)
        .on_pointer_move_local(|ui: &mut Ui, _, tx: &mut Tx| {
            let now = tx.clock();
            if let Some(review) = &mut ui.review
                && (!review.controls_visible || now - review.last_move > MOVE_THROTTLE_SECS)
            {
                review.wake_controls(now);
            }
        })
        .child(video_frame(
            review,
            &k,
            el().key("review-video")
                .pin(Some(0.0), Some(0.0), Some(0.0), Some(0.0))
                .fill(Color::BLACK),
        ));
    if review.controls_visible || !review.is_playing() {
        root = root.child(bar(review, &k));
    }
    root
}

fn bar(review: &ReviewState, k: &Kit) -> El {
    let play = if review.is_playing() {
        IconButton::new(&icons::PAUSE).label("Pause").help("Pause")
    } else {
        IconButton::new(&icons::PLAY).label("Play").help("Play")
    };
    let play = play
        .variant(IconButtonVariant::Ghost)
        .control_size(ControlSize::Large)
        .shortcut("Space")
        .on_press(|ui: &mut Ui| {
            if let Some(review) = &ui.review {
                review.toggle();
            }
        })
        .build(k)
        .key("review-play");
    let exit = IconButton::new(&icons::SHRINK)
        .label("Exit theater")
        .help("Exit theater")
        .shortcut("Esc")
        .variant(IconButtonVariant::Ghost)
        .control_size(ControlSize::Large)
        .on_press_tx(|ui: &mut Ui, tx: &mut Tx| ui.set_theater(false, tx))
        .build(k)
        .key("theater-exit");
    let clock = format!(
        "{} / {}",
        format_clock(review.time() as u32),
        format_clock(review.duration() as u32)
    );
    let scrim = [
        (0.0, Color::BLACK.alpha(0.0)),
        (0.6, Color::BLACK.alpha(SCRIM_ALPHA * 0.6)),
        (1.0, Color::BLACK.alpha(SCRIM_ALPHA)),
    ];
    col()
        .key("theater-bar")
        .pin(Some(0.0), None, Some(0.0), Some(0.0))
        .px(BAR_PAD_X)
        .pt(BAR_PAD_TOP)
        .pb(BAR_PAD_BOTTOM)
        .linear_gradient(PI, scrim)
        .child(
            row()
                .w_full()
                .gap(12.0)
                .center_y()
                .child(play)
                .child(
                    text(clock)
                        .key("review-clock")
                        .font(k.ty.body)
                        .color(k.c.text_base),
                )
                .child(timeline::timeline(review, k))
                .child(exit),
        )
}
