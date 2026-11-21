use std::borrow::Cow;

use greyscreen_core::highlight::Highlight;
use greyscreen_engine::{EngineCmd, EngineStatus, FeedItem};
use rune::{Color, El, GrowthSpec, ScrollSpec, col, el, row, text};
use rune_kit::{Badge, BadgeVariant, Button, ButtonVariant, ControlSize, Kit};

use super::{card, hairline, title_style};
use crate::ui::{TEST_RECORDING_SECS, Ui, format_clock};

const FEED_ROW_HEIGHT: f32 = 36.0;
const BADGE_WIDTH: f32 = 140.0;

pub(super) fn page(s: &Ui, k: &Kit) -> El {
    col()
        .key("status")
        .w_full()
        .h_full()
        .min_h(0.0)
        .gap(16.0)
        .child(state_card(s, k))
        .child(feed_card(s, k))
}

struct Headline {
    pill: El,
    title: String,
    detail: String,
    clock: Option<String>,
    danger: bool,
}

fn headline(s: &Ui, k: &Kit) -> Headline {
    let (pill, title, detail, clock, danger) = match &s.status {
        EngineStatus::Idle => (
            pill(k, "Idle", k.c.text_faint, k.c.layer_02, k.c.text_muted),
            "Waiting for League".to_owned(),
            "Greyscreen starts recording when a game starts.".to_owned(),
            None,
            false,
        ),
        EngineStatus::Waiting => (
            pill(k, "Waiting", k.c.text_faint, k.c.layer_02, k.c.text_muted),
            "League is running".to_owned(),
            "Waiting for the game to load.".to_owned(),
            None,
            false,
        ),
        EngineStatus::Recording { game, .. } => {
            let (title, detail) = match game {
                Some(info) => (
                    info.me.champion.clone(),
                    format!("{} - recording the game", info.mode),
                ),
                None => (
                    "Test recording".to_owned(),
                    "Recording the primary screen.".to_owned(),
                ),
            };
            (
                pill_recording(k),
                title,
                detail,
                Some(format_clock(s.elapsed_secs)),
                false,
            )
        }
        EngineStatus::Finalizing => (
            pill(k, "Saving", k.c.text_faint, k.c.layer_02, k.c.text_muted),
            "Saving...".to_owned(),
            "Finishing the video file.".to_owned(),
            None,
            false,
        ),
        EngineStatus::Error(message) => (
            pill(k, "Error", k.c.danger.icon, k.c.danger.bg, k.c.danger.fg),
            message.clone(),
            "Check the logs folder in Settings for details.".to_owned(),
            None,
            true,
        ),
    };
    Headline {
        pill,
        title,
        detail,
        clock,
        danger,
    }
}

fn pill_recording(k: &Kit) -> El {
    pill(
        k,
        "Recording",
        k.c.danger.icon,
        k.c.danger.bg,
        k.c.danger.fg,
    )
}

fn buttons(s: &Ui, k: &Kit) -> El {
    let recording = matches!(s.status, EngineStatus::Recording { .. });
    let idle = matches!(s.status, EngineStatus::Idle);
    let mut buttons = row().gap(10.0).center_y();
    if recording {
        buttons = buttons
            .child(
                Button::new("Stop")
                    .variant(ButtonVariant::Neutral)
                    .control_size(ControlSize::Large)
                    .on_press(|ui: &mut Ui| ui.send(EngineCmd::StopRecording))
                    .build(k)
                    .key("stop-recording"),
            )
            .child(
                Button::new("Save clip (last 30 s)")
                    .variant(ButtonVariant::Neutral)
                    .control_size(ControlSize::Large)
                    .on_press(|ui: &mut Ui| ui.send(EngineCmd::SaveClipNow))
                    .build(k)
                    .key("save-clip"),
            );
    } else {
        buttons = buttons.child(
            Button::new("Test recording (10 s)")
                .variant(ButtonVariant::Neutral)
                .control_size(ControlSize::Large)
                .disabled(!idle)
                .on_press(|ui: &mut Ui| {
                    ui.send(EngineCmd::StartTestRecording {
                        secs: TEST_RECORDING_SECS,
                    })
                })
                .build(k)
                .key("test-recording"),
        );
    }
    buttons
}

fn state_card(s: &Ui, k: &Kit) -> El {
    let h = headline(s, k);
    let title_color = if h.danger {
        k.c.danger.fg
    } else {
        k.c.text_base
    };
    let left = col()
        .gap(10.0)
        .child(h.pill)
        .child(
            col()
                .gap(2.0)
                .child(
                    text(h.title)
                        .key("status-headline")
                        .font(title_style(k))
                        .color(title_color),
                )
                .child(text(h.detail).font(k.ty.body).color(k.c.text_muted)),
        )
        .child(el().h(2.0))
        .child(buttons(s, k));
    let mut body = row().w_full().justify_between().center_y().child(left);
    if let Some(clock) = h.clock {
        body = body.child(
            col()
                .items_end()
                .gap(2.0)
                .child(
                    text(clock)
                        .key("elapsed")
                        .font(rune::TextStyle {
                            size: 40.0,
                            line_height: 44.0,
                            ..k.ty.mono
                        })
                        .color(k.c.text_base),
                )
                .child(text("elapsed").font(k.ty.small).color(k.c.text_muted)),
        );
    }
    let mut card = card(k).key("state-card").padding(20.0);
    if h.danger {
        card = card.border(1.0, k.c.danger.icon);
    }
    card.child(body)
}

fn pill(k: &Kit, label: &str, dot: Color, fill: Color, ink: Color) -> El {
    row()
        .self_start()
        .h(24.0)
        .px(10.0)
        .gap(6.0)
        .center_y()
        .radius(12.0)
        .fill(fill)
        .child(el().size(8.0, 8.0).radius(4.0).fill(dot).a11y_hidden())
        .child(text(label).font(k.ty.ui_medium).color(ink))
}

fn feed_card(s: &Ui, k: &Kit) -> El {
    let count = s.feed.len();
    let header = row()
        .w_full()
        .h(44.0)
        .px(16.0)
        .center_y()
        .justify_between()
        .child(
            text("Live feed")
                .font(k.ty.body_medium)
                .color(k.c.text_base),
        )
        .child(
            text(format!("{count} events"))
                .font(k.ty.small)
                .color(k.c.text_muted),
        );
    let list = if s.feed.is_empty() {
        col().w_full().grow(1.0).center().child(
            text("Game events show up here while you play.")
                .font(k.ty.body)
                .color(k.c.text_muted),
        )
    } else {
        let kit = k.clone();
        col()
            .key("feed-scroll")
            .w_full()
            .grow(1.0)
            .min_h(0.0)
            .scroll(ScrollSpec { stick_bottom: true })
            .clip_fade(0.0, 16.0, 0.0, 0.0)
            .child(col().w_full().growth(GrowthSpec::default()).virtual_rows(
                count,
                FEED_ROW_HEIGHT,
                |s: &Ui, i| Cow::Owned(format!("feed-{}", s.feed[i].id)),
                move |s: &Ui, i| feed_row(&s.feed[i], &kit),
            ))
    };
    card(k)
        .key("feed-card")
        .grow(1.0)
        .min_h(0.0)
        .child(header)
        .child(hairline(k))
        .child(list)
}

fn feed_row(item: &FeedItem, k: &Kit) -> El {
    let (label, variant) = badge(item);
    row()
        .key(format!("feed-{}", item.id))
        .w_full()
        .h(FEED_ROW_HEIGHT)
        .px(16.0)
        .gap(12.0)
        .center_y()
        .child(
            el().w(48.0).child(
                text(format_clock(item.game_secs as u32))
                    .font(k.ty.mono)
                    .color(k.c.text_muted),
            ),
        )
        .child(
            el().w(BADGE_WIDTH)
                .child(Badge::new(label).variant(variant).build(k)),
        )
        .child(
            text(item.text.clone())
                .font(k.ty.body)
                .color(k.c.text_base)
                .truncate(),
        )
}

fn badge(item: &FeedItem) -> (String, BadgeVariant) {
    match item.highlights.first() {
        Some(highlight) => (spaced_upper(highlight.label()), badge_variant(highlight)),
        None => ("EVENT".to_owned(), BadgeVariant::Neutral),
    }
}

fn badge_variant(highlight: &Highlight) -> BadgeVariant {
    match highlight {
        Highlight::MyKill
        | Highlight::MyMultikill { .. }
        | Highlight::MyFirstBlood
        | Highlight::MyAce
        | Highlight::GameWon => BadgeVariant::Accent,
        Highlight::MyDeath
        | Highlight::EnemyKill
        | Highlight::EnemyObjective(_)
        | Highlight::EnemyStructure(_)
        | Highlight::EnemyAce
        | Highlight::GameLost => BadgeVariant::Subtle,
        _ => BadgeVariant::Neutral,
    }
}

fn spaced_upper(label: &str) -> String {
    let mut out = String::with_capacity(label.len() + 2);
    let mut prev_lower = false;
    for ch in label.chars() {
        if (ch.is_uppercase() || ch.is_ascii_digit()) && prev_lower {
            out.push(' ');
        }
        prev_lower = ch.is_lowercase();
        out.extend(ch.to_uppercase());
    }
    out
}
