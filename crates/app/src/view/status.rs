use std::borrow::Cow;

use rune::{Color, El, GrowthSpec, ScrollSpec, col, el, row, text};
use rune_kit::{Badge, BadgeVariant, Button, ButtonVariant, ControlSize, Kit};

use super::{card, hairline, title_style};
use crate::ui::{FeedKind, FeedRow, PreviewStatus, Ui, format_clock};

const FEED_ROW_HEIGHT: f32 = 36.0;

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

fn state_card(s: &Ui, k: &Kit) -> El {
    let (pill, headline, detail, clock) = match &s.status {
        PreviewStatus::Recording {
            elapsed_secs,
            champion,
            mode,
        } => (
            pill(
                k,
                "Recording",
                k.c.danger.icon,
                k.c.danger.bg,
                k.c.danger.fg,
            ),
            champion.clone(),
            format!("{mode} - recording the full game"),
            Some(format_clock(*elapsed_secs)),
        ),
        PreviewStatus::Idle => (
            pill(k, "Idle", k.c.text_faint, k.c.layer_02, k.c.text_muted),
            "Waiting for a game".to_owned(),
            "Greyscreen starts recording when League of Legends starts.".to_owned(),
            None,
        ),
    };
    let mut left = col().gap(10.0).child(pill).child(
        col()
            .gap(2.0)
            .child(
                text(headline)
                    .key("status-headline")
                    .font(title_style(k))
                    .color(k.c.text_base),
            )
            .child(text(detail).font(k.ty.body).color(k.c.text_muted)),
    );
    let test_button = Button::new("Test recording (10 s)")
        .variant(ButtonVariant::Neutral)
        .control_size(ControlSize::Large)
        .disabled(true)
        .help("coming soon")
        .build(k)
        .self_start()
        .key("test-recording");
    left = left.child(el().h(2.0)).child(test_button);
    let mut body = row().w_full().justify_between().center_y().child(left);
    if let Some(clock) = clock {
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
    card(k).key("state-card").padding(20.0).child(body)
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

fn feed_row(item: &FeedRow, k: &Kit) -> El {
    let (label, variant) = kind_badge(item.kind);
    row()
        .key(format!("feed-{}", item.id))
        .w_full()
        .h(FEED_ROW_HEIGHT)
        .px(16.0)
        .gap(12.0)
        .center_y()
        .child(
            el().w(48.0).child(
                text(format_clock(item.game_secs))
                    .font(k.ty.mono)
                    .color(k.c.text_muted),
            ),
        )
        .child(
            el().w(100.0)
                .child(Badge::new(label).variant(variant).build(k)),
        )
        .child(
            text(item.text.clone())
                .font(k.ty.body)
                .color(k.c.text_base)
                .truncate(),
        )
}

fn kind_badge(kind: FeedKind) -> (&'static str, BadgeVariant) {
    match kind {
        FeedKind::Kill => ("KILL", BadgeVariant::Accent),
        FeedKind::Death => ("DEATH", BadgeVariant::Subtle),
        FeedKind::Assist => ("ASSIST", BadgeVariant::Neutral),
        FeedKind::Multikill => ("MULTIKILL", BadgeVariant::Accent),
        FeedKind::FirstBlood => ("FIRST BLOOD", BadgeVariant::Accent),
        FeedKind::Dragon => ("DRAGON", BadgeVariant::Neutral),
        FeedKind::Herald => ("HERALD", BadgeVariant::Neutral),
        FeedKind::Baron => ("BARON", BadgeVariant::Neutral),
        FeedKind::Turret => ("TURRET", BadgeVariant::Subtle),
        FeedKind::Inhibitor => ("INHIBITOR", BadgeVariant::Subtle),
        FeedKind::Ace => ("ACE", BadgeVariant::Accent),
    }
}
