use std::f32::consts::PI;

use rune::{Color, El, Transition, Tx, col, el, row, text};
use rune_kit::{Button, ButtonVariant, ControlSize, Kit};

use super::panel::{PANEL_WIDTH, events_body};
use super::{icons, result_label, transport, video_frame};
use crate::ui::library::format::mode_label;
use crate::ui::{ReviewState, Ui};

const BAR_PAD_X: f32 = 24.0;
const BAR_PAD_TOP: f32 = 64.0;
const BAR_PAD_BOTTOM: f32 = 16.0;
const TOP_PAD_BOTTOM: f32 = 56.0;
const TOP_PAD_TOP: f32 = 16.0;
const TOP_INSET: f32 = 72.0;
const BOTTOM_INSET: f32 = 124.0;
const DRAWER_GAP: f32 = 16.0;
const DRAWER_SLIDE: f32 = 48.0;
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
    if review.drawer {
        root = root.child(drawer(review, &k));
    }
    if review.controls_visible || !review.is_playing() {
        root = root
            .child(top_bar(review, &k))
            .child(bottom_bar(review, &k));
    }
    root
}

fn scrim() -> [(f32, Color); 3] {
    [
        (0.0, Color::BLACK.alpha(0.0)),
        (0.6, Color::BLACK.alpha(SCRIM_ALPHA * 0.6)),
        (1.0, Color::BLACK.alpha(SCRIM_ALPHA)),
    ]
}

fn title_line(review: &ReviewState) -> String {
    let meta = &review.entry.meta;
    let mut parts = vec![
        meta.game.info.me.champion.clone(),
        mode_label(&meta.game.info.mode),
    ];
    parts.extend(result_label(meta.game.result).map(str::to_owned));
    parts.join(" · ")
}

fn top_bar(review: &ReviewState, k: &Kit) -> El {
    let exit = Button::new("Exit")
        .shortcut("Esc")
        .icon(&icons::ARROW_LEFT)
        .variant(ButtonVariant::Ghost)
        .control_size(ControlSize::Regular)
        .on_press_tx(|ui: &mut Ui, tx: &mut Tx| ui.set_theater(false, tx))
        .build(k)
        .key("theater-exit");
    let events = Button::new("Events")
        .icon(&icons::LIST)
        .variant(if review.drawer {
            ButtonVariant::Neutral
        } else {
            ButtonVariant::Ghost
        })
        .control_size(ControlSize::Regular)
        .on_press(|ui: &mut Ui| {
            let open = ui.review.as_ref().is_some_and(|r| !r.drawer);
            ui.review_drawer(open);
        })
        .build(k)
        .key("theater-events");
    col()
        .key("theater-top")
        .pin(Some(0.0), Some(0.0), Some(0.0), None)
        .px(BAR_PAD_X)
        .pt(TOP_PAD_TOP)
        .pb(TOP_PAD_BOTTOM)
        .linear_gradient(0.0, scrim())
        .on_background_press(|_: &mut Ui| {})
        .child(
            row()
                .w_full()
                .gap(12.0)
                .center_y()
                .child(exit)
                .child(
                    text(title_line(review))
                        .key("theater-title")
                        .font(k.ty.body)
                        .color(k.c.text_base),
                )
                .child(el().grow(1.0))
                .child(events),
        )
}

fn bottom_bar(review: &ReviewState, k: &Kit) -> El {
    col()
        .key("theater-bar")
        .pin(Some(0.0), None, Some(0.0), Some(0.0))
        .px(BAR_PAD_X)
        .pt(BAR_PAD_TOP)
        .pb(BAR_PAD_BOTTOM)
        .linear_gradient(PI, scrim())
        .on_background_press(|_: &mut Ui| {})
        .child(transport::transport(review, k, true, None))
}

fn drawer(review: &ReviewState, k: &Kit) -> El {
    col()
        .key("theater-drawer")
        .pin(None, Some(TOP_INSET), Some(DRAWER_GAP), Some(BOTTOM_INSET))
        .w(PANEL_WIDTH)
        .radius(10.0)
        .fill(Color::BLACK.alpha(0.82))
        .clip_rounded(10.0)
        .on_background_press(|_: &mut Ui| {})
        .transition(Transition::offset(DRAWER_SLIDE, 0.0).combined(Transition::opacity()))
        .child(events_body(review, k))
}
