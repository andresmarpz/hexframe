use greyscreen_core::review::DeathTag;
use rune::{Drag, El, Phase, TextEdit, Tx, col, el, row, text};
use rune_kit::{Badge, BadgeVariant, Button, ButtonVariant, ControlSize, Kit, Switch, TextArea};

use super::timeline::thumb_rail;
use super::{PAGE_PADDING, icons, play_button, tabular, video_frame};
use crate::ui::{DeathReviewState, ReviewState, Ui, title_line};
use crate::view::card;
use crate::view::champion::champion_icon;

const ICON: f32 = 28.0;
const PIP_HIT: f32 = 20.0;
const PIP: f32 = 8.0;
const PIP_CURRENT: f32 = 12.0;
const BAR_HEIGHT: f32 = 24.0;
const TRACK_HEIGHT: f32 = 4.0;
const MARKER_WIDTH: f32 = 3.0;
const MARKER_HEIGHT: f32 = 16.0;
const THUMB: f32 = 12.0;
const CHIP_HEIGHT: f32 = 26.0;
const NOTE_ROWS: usize = 2;

pub(super) fn screen(
    review: &ReviewState,
    state: &DeathReviewState,
    note: &TextEdit,
    k: &Kit,
) -> El {
    col()
        .key("death-review")
        .w_full()
        .h_full()
        .min_h(0.0)
        .padding(PAGE_PADDING)
        .gap(12.0)
        .child(header(review, state, k))
        .child(video(review, k))
        .child(window_bar(review, state, k))
        .child(controls(review, state, k))
        .child(coach(review, state, note, k))
}

fn killer_icon(review: &ReviewState, state: &DeathReviewState, k: &Kit) -> Option<El> {
    let death = state.current()?;
    let player = review.entry.meta.game.info.player_named(&death.killer)?;
    Some(champion_icon(k, &player.champion, ICON))
}

fn pip(index: usize, state: &DeathReviewState, review: &ReviewState, k: &Kit) -> El {
    let death = &state.deaths[index];
    let current = index == state.index;
    let reviewed = review.entry.meta.review.is_reviewed(death.event_id);
    let color = if current {
        k.c.text_base
    } else if reviewed {
        k.c.success.icon
    } else {
        k.c.border_strong
    };
    let size = if current { PIP_CURRENT } else { PIP };
    let dot = el().size(size, size).radius(size / 2.0).fill(color);
    let hit = row()
        .key(format!("death-pip-{index}"))
        .role("button")
        .label(format!("Go to death {}", index + 1))
        .size(PIP_HIT, PIP_HIT)
        .center()
        .on_press(move |ui: &mut Ui| ui.death_go(index))
        .child(dot);
    k.interactive(hit)
}

fn header(review: &ReviewState, state: &DeathReviewState, k: &Kit) -> El {
    let finish = Button::new("Finish")
        .shortcut("Esc")
        .icon(&icons::ARROW_LEFT)
        .variant(ButtonVariant::Neutral)
        .control_size(ControlSize::Regular)
        .on_press(|ui: &mut Ui| ui.death_finish())
        .build(k)
        .key("death-finish");
    let mut line = row()
        .key("death-header")
        .w_full()
        .gap(12.0)
        .center_y()
        .child(finish);
    if let Some(icon) = killer_icon(review, state, k) {
        line = line.child(icon);
    }
    let pips = row()
        .key("death-pips")
        .center_y()
        .children((0..state.deaths.len()).map(|i| pip(i, state, review, k)));
    line.child(
        text(title_line(state, review))
            .key("death-title")
            .font(k.ty.body_medium)
            .color(k.c.text_base)
            .truncate(),
    )
    .child(el().grow(1.0))
    .child(pips)
}

fn video(review: &ReviewState, k: &Kit) -> El {
    let frame = el()
        .key("review-video")
        .w_full()
        .grow(1.0)
        .min_h(0.0)
        .radius(8.0)
        .fill(rune::Color::BLACK);
    video_frame(review, k, frame)
}

fn window_bar(review: &ReviewState, state: &DeathReviewState, k: &Kit) -> El {
    let (Some((start, end)), Some(death)) = (review.death_window(), state.current()) else {
        return el().key("death-window");
    };
    let span = (end - start).max(f64::EPSILON);
    let share = |secs: f64| ((secs - start) / span).clamp(0.0, 1.0) as f32;
    let progress = share(review.time());
    let y = (BAR_HEIGHT - TRACK_HEIGHT) / 2.0;
    let mut bar = el()
        .key("death-window")
        .w_full()
        .h(BAR_HEIGHT)
        .on_drag(move |ui: &mut Ui, drag: &mut Drag, _| {
            if matches!(drag.phase, Phase::Down | Phase::Move) && drag.w > 0.0 {
                ui.death_scrub(f64::from(drag.x / drag.w));
            }
        })
        .child(
            el().key("death-track")
                .absolute(0.0, y)
                .w_frac(1.0)
                .h(TRACK_HEIGHT)
                .radius(TRACK_HEIGHT / 2.0)
                .fill(k.c.layer_03),
        );
    if state.slow_mo {
        let from = share(death.slow_mo_from(review.duration()));
        bar = bar.child(
            row()
                .absolute(0.0, y)
                .w_frac(1.0)
                .h(TRACK_HEIGHT)
                .a11y_hidden()
                .child(el().w_frac(from))
                .child(
                    el().key("death-slow-zone")
                        .grow(1.0)
                        .h(TRACK_HEIGHT)
                        .radius(TRACK_HEIGHT / 2.0)
                        .fill(k.c.warning.icon.alpha(0.55)),
                ),
        );
    }
    bar.child(
        el().key("death-fill")
            .absolute(0.0, y)
            .w_frac(progress)
            .h(TRACK_HEIGHT)
            .radius(TRACK_HEIGHT / 2.0)
            .fill(k.c.bg_accent)
            .instant_changes()
            .a11y_hidden(),
    )
    .child(thumb_rail(
        el().key("death-marker")
            .size(MARKER_WIDTH, MARKER_HEIGHT)
            .radius(MARKER_WIDTH / 2.0)
            .fill(k.c.danger.icon),
        MARKER_WIDTH,
        BAR_HEIGHT,
        share(death.rec_secs),
    ))
    .child(thumb_rail(
        el().key("death-thumb")
            .size(THUMB, THUMB)
            .radius(THUMB / 2.0)
            .fill(rune::Color::WHITE)
            .border(1.0, k.c.border_strong),
        THUMB,
        BAR_HEIGHT,
        progress,
    ))
}

fn controls(review: &ReviewState, state: &DeathReviewState, k: &Kit) -> El {
    let previous = Button::new("Previous")
        .icon(&icons::ARROW_LEFT)
        .variant(ButtonVariant::Neutral)
        .control_size(ControlSize::Regular)
        .disabled(!state.has_previous())
        .shortcut("P")
        .on_press(|ui: &mut Ui| ui.death_previous())
        .build(k)
        .key("death-prev");
    let next = Button::new("Next")
        .icon(&icons::ARROW_RIGHT)
        .variant(ButtonVariant::Neutral)
        .control_size(ControlSize::Regular)
        .disabled(!state.has_next())
        .shortcut("N")
        .on_press(|ui: &mut Ui| ui.death_next())
        .build(k)
        .key("death-next");
    let replay_variant = if state.finished {
        ButtonVariant::Contrast
    } else {
        ButtonVariant::Neutral
    };
    let replay = Button::new("Replay")
        .icon(&icons::REWIND)
        .variant(replay_variant)
        .control_size(ControlSize::Regular)
        .shortcut("R")
        .on_press(|ui: &mut Ui| ui.death_replay())
        .build(k)
        .key("death-replay");
    let slow = Switch::new(state.slow_mo)
        .label("Slow-mo the final seconds")
        .on_toggle(|ui: &mut Ui| ui.death_slow_mo())
        .build(k)
        .key("death-slow-mo");
    let clock = {
        let (start, _) = review.death_window().unwrap_or((0.0, 0.0));
        let at = (review.time() - start).max(0.0);
        text(format!("{at:.1} s"))
            .key("death-clock")
            .font(tabular(k.ty.body))
            .color(k.c.text_muted)
    };
    row()
        .key("death-controls")
        .w_full()
        .gap(8.0)
        .center_y()
        .child(previous)
        .child(play_button(review, k, rune_kit::IconButtonVariant::Neutral))
        .child(replay)
        .child(next)
        .child(clock)
        .child(el().grow(1.0))
        .child(slow)
}

fn tag_chip(index: usize, tag: DeathTag, on: bool, k: &Kit) -> El {
    let chip = row()
        .key(format!("death-tag-{}", tag.id()))
        .role("button")
        .label(tag.label())
        .help(format!("Press {} to toggle", index + 1))
        .h(CHIP_HEIGHT)
        .px(12.0)
        .gap(6.0)
        .center()
        .radius(CHIP_HEIGHT / 2.0)
        .fill(if on {
            k.c.bg_accent.alpha(0.18)
        } else {
            rune::Color::BLACK.alpha(0.0)
        })
        .border(1.0, if on { k.c.bg_accent } else { k.c.border_base })
        .on_press_tx(move |ui: &mut Ui, tx: &mut Tx| ui.death_toggle_tag(tag, tx))
        .child(
            text(format!("{}", index + 1))
                .font(tabular(k.ty.small))
                .color(if on { k.c.text_accent } else { k.c.text_muted }),
        )
        .child(
            text(tag.label())
                .font(if on { k.ty.small_medium } else { k.ty.small })
                .color(if on { k.c.text_base } else { k.c.text_muted }),
        );
    k.interactive(chip)
}

fn coach(review: &ReviewState, state: &DeathReviewState, note: &TextEdit, k: &Kit) -> El {
    let id = state.current().map_or(0, |d| d.event_id);
    let data = &review.entry.meta.review;
    let tags = row()
        .key("death-tags")
        .w_full()
        .gaps(8.0, 6.0)
        .flex_wrap()
        .children(
            DeathTag::ALL
                .into_iter()
                .enumerate()
                .map(|(i, tag)| tag_chip(i, tag, data.has_tag(id, tag), k)),
        );
    let saved = if data.is_reviewed(id) {
        Badge::new("Saved").variant(BadgeVariant::Neutral).build(k)
    } else {
        el()
    };
    let heading = row()
        .w_full()
        .center_y()
        .child(
            text("What went wrong?")
                .font(k.ty.body_medium)
                .color(k.c.text_base),
        )
        .child(el().grow(1.0))
        .child(saved);
    let field = TextArea::new(note)
        .label("Note about this death")
        .placeholder("Add a note (optional)")
        .rows(NOTE_ROWS)
        .on_edit(|ui: &mut Ui| &mut ui.death_note)
        .on_change(|ui: &mut Ui, tx: &mut Tx| ui.death_note_edited(tx))
        .build(k)
        .w_full()
        .key("death-note");
    card(k)
        .key("death-coach")
        .px(12.0)
        .py(12.0)
        .gap(10.0)
        .child(heading)
        .child(tags)
        .child(field)
}
