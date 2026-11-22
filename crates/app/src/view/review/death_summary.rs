use greyscreen_core::review::{Death, ReviewSummary};
use rune::{Color, El, col, el, row, text};
use rune_kit::{Button, ButtonVariant, ControlSize, Kit};

use super::{PAGE_PADDING, tabular};
use crate::ui::{DeathReviewState, ReviewState, Ui, format_clock};
use crate::view::{card, hairline, title_style};

const BAR_HEIGHT: f32 = 8.0;
const LABEL_WIDTH: f32 = 130.0;
const COUNT_WIDTH: f32 = 24.0;
const TRACK_HEIGHT: f32 = 4.0;
const AXIS_HEIGHT: f32 = 28.0;
const DOT: f32 = 12.0;
const CARD_WIDTH: f32 = 620.0;

pub(super) fn screen(review: &ReviewState, state: &DeathReviewState, k: &Kit) -> El {
    let summary = review.entry.meta.review.summary(&state.deaths);
    col()
        .key("death-summary")
        .w_full()
        .h_full()
        .min_h(0.0)
        .padding(PAGE_PADDING)
        .items_center()
        .gap(16.0)
        .child(
            card(k)
                .key("death-summary-card")
                .max_w(CARD_WIDTH)
                .px(20.0)
                .py(20.0)
                .gap(18.0)
                .child(heading(&summary, k))
                .child(stats(&summary, k))
                .child(hairline(k))
                .child(by_tag(&summary, k))
                .child(hairline(k))
                .child(timeline(review, state, k))
                .child(footer(k)),
        )
}

fn heading(summary: &ReviewSummary, k: &Kit) -> El {
    let line = if summary.deaths == 1 {
        "1 death this game".to_owned()
    } else {
        format!("{} deaths this game", summary.deaths)
    };
    col()
        .gap(2.0)
        .child(
            text("Death review complete")
                .font(title_style(k))
                .color(k.c.text_base),
        )
        .child(text(line).font(k.ty.body).color(k.c.text_muted))
}

fn stat(key: &str, value: usize, label: &str, k: &Kit) -> El {
    col()
        .key(key.to_owned())
        .grow(1.0)
        .gap(2.0)
        .child(
            text(value.to_string())
                .font(tabular(rune::TextStyle {
                    size: 28.0,
                    line_height: 34.0,
                    ..k.ty.body_medium
                }))
                .color(k.c.text_base),
        )
        .child(
            text(label.to_owned())
                .font(k.ty.small)
                .color(k.c.text_muted),
        )
}

fn stats(summary: &ReviewSummary, k: &Kit) -> El {
    row()
        .w_full()
        .child(stat("death-stat-deaths", summary.deaths, "Deaths", k))
        .child(stat("death-stat-tagged", summary.reviewed, "Reviewed", k))
        .child(stat("death-stat-notes", summary.notes, "Notes", k))
}

fn section_title(label: &str, k: &Kit) -> El {
    text(label.to_owned())
        .font(k.ty.body_medium)
        .color(k.c.text_base)
}

fn tag_bar(label: &str, count: usize, most: usize, k: &Kit) -> El {
    let share = count as f32 / most.max(1) as f32;
    row()
        .w_full()
        .gap(12.0)
        .center_y()
        .child(
            text(label.to_owned())
                .font(k.ty.body)
                .color(k.c.text_base)
                .w(LABEL_WIDTH),
        )
        .child(
            row()
                .grow(1.0)
                .h(BAR_HEIGHT)
                .radius(BAR_HEIGHT / 2.0)
                .fill(k.c.layer_03)
                .child(
                    el().w_frac(share)
                        .h(BAR_HEIGHT)
                        .radius(BAR_HEIGHT / 2.0)
                        .fill(k.c.bg_accent),
                ),
        )
        .child(
            row().w(COUNT_WIDTH).justify_end().child(
                text(count.to_string())
                    .font(tabular(k.ty.body_medium))
                    .color(k.c.text_base),
            ),
        )
}

fn by_tag(summary: &ReviewSummary, k: &Kit) -> El {
    let most = summary.tags.first().map_or(0, |(_, count)| *count);
    let body = if summary.tags.is_empty() {
        col().child(
            text("No tags added yet.")
                .font(k.ty.body)
                .color(k.c.text_muted),
        )
    } else {
        col()
            .gap(10.0)
            .children(summary.tags.iter().map(|(tag, count)| {
                tag_bar(tag.label(), *count, most, k).key(format!("death-bar-{}", tag.id()))
            }))
    };
    col()
        .key("death-by-tag")
        .w_full()
        .gap(12.0)
        .child(section_title("Deaths by tag", k))
        .child(body)
}

fn dot(index: usize, death: &Death, review: &ReviewState, x: f32, k: &Kit) -> El {
    let reviewed = review.entry.meta.review.is_reviewed(death.event_id);
    let fill = if reviewed {
        k.c.danger.icon
    } else {
        Color::BLACK.alpha(0.0)
    };
    let tags: Vec<&str> = review
        .entry
        .meta
        .review
        .get(death.event_id)
        .map(|d| d.tags.iter().map(|t| t.label()).collect())
        .unwrap_or_default();
    let tags_suffix = if tags.is_empty() {
        String::new()
    } else {
        format!(" · {}", tags.join(", "))
    };
    let tip = format!(
        "Death {} · {}{}",
        index + 1,
        format_clock(death.rec_secs as u32),
        tags_suffix
    );
    let dot = el()
        .key(format!("death-dot-{index}"))
        .role("button")
        .label(format!("Review death {}", index + 1))
        .help(tip)
        .size(DOT, DOT)
        .absolute(x, (AXIS_HEIGHT - DOT) / 2.0)
        .radius(DOT / 2.0)
        .fill(fill)
        .border(1.5, k.c.danger.icon)
        .on_press(move |ui: &mut Ui| ui.death_go(index));
    k.interactive(dot)
}

fn dot_x(rec_secs: f64, duration: f64, width: f32) -> f32 {
    if duration <= 0.0 {
        return 0.0;
    }
    let centre = (rec_secs / duration).clamp(0.0, 1.0) as f32 * width;
    (centre - DOT / 2.0).clamp(0.0, (width - DOT).max(0.0))
}

fn timeline(review: &ReviewState, state: &DeathReviewState, k: &Kit) -> El {
    let duration = review.duration();
    let width = review.track_width;
    let mut axis = el()
        .key("death-timeline")
        .w_full()
        .h(AXIS_HEIGHT)
        .on_layout(|ui: &mut Ui, rect: [f32; 4]| {
            if let Some(review) = &mut ui.review {
                review.track_width = rect[2];
            }
        })
        .child(
            el().absolute(0.0, (AXIS_HEIGHT - TRACK_HEIGHT) / 2.0)
                .size(width, TRACK_HEIGHT)
                .radius(TRACK_HEIGHT / 2.0)
                .fill(k.c.layer_03)
                .a11y_hidden(),
        );
    for (index, death) in state.deaths.iter().enumerate() {
        axis = axis.child(dot(
            index,
            death,
            review,
            dot_x(death.rec_secs, duration, width),
            k,
        ));
    }
    let end = format_clock(duration as u32);
    col()
        .w_full()
        .gap(6.0)
        .child(section_title("When you died", k))
        .child(axis)
        .child(
            row()
                .w_full()
                .child(
                    text("00:00")
                        .font(tabular(k.ty.small))
                        .color(k.c.text_muted),
                )
                .child(el().grow(1.0))
                .child(text(end).font(tabular(k.ty.small)).color(k.c.text_muted)),
        )
}

fn footer(k: &Kit) -> El {
    let again = Button::new("Review again")
        .variant(ButtonVariant::Neutral)
        .control_size(ControlSize::Regular)
        .shortcut("D")
        .on_press(|ui: &mut Ui| ui.death_again())
        .build(k)
        .key("death-again");
    let done = Button::new("Done")
        .variant(ButtonVariant::Submit)
        .control_size(ControlSize::Regular)
        .shortcut("Esc")
        .on_press(|ui: &mut Ui| ui.death_close())
        .build(k)
        .key("death-done");
    row()
        .w_full()
        .gap(8.0)
        .justify_end()
        .child(again)
        .child(done)
}
