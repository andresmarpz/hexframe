use std::time::Duration;

use rune::{El, col, row, text};
use rune_kit::{Kit, Placement, popover};

use super::tabular;
use crate::ui::Ui;
use crate::ui::event_detail::{Detail, Line, Span, describe};

const HOVER_DELAY: Duration = Duration::from_millis(150);
const ANCHOR_GAP: f32 = 8.0;
const LEAD_WIDTH: f32 = 56.0;
const CARD_MIN_WIDTH: f32 = 200.0;
const CARD_MAX_WIDTH: f32 = 360.0;

pub(super) fn hoverable(element: El, event_id: u32, anchor: String) -> El {
    element
        .on_hover_change(move |ui: &mut Ui, inside, _| {
            if let Some(review) = &mut ui.review {
                review.hover_event(event_id, &anchor, inside);
            }
        })
        .hover_delays(HOVER_DELAY, Duration::ZERO)
}

pub(crate) fn hover_card(s: &Ui, k: &Kit) -> Option<El> {
    let review = s.review.as_ref()?;
    let (event, anchor) = review.hovered_event()?;
    let detail = describe(
        event,
        &review.entry.meta.events,
        &review.entry.meta.game.info,
    )?;
    let dark = Kit::neutral(true);
    let k = if review.theater { &dark } else { k };
    let placement = if anchor.starts_with("marker-") {
        Placement::Top
    } else {
        Placement::Left
    };
    let surface = popover::raised(k, body(&detail, k), k.radius.md);
    Some(popover::anchored(k, surface, anchor, placement, ANCHOR_GAP))
}

fn body(detail: &Detail, k: &Kit) -> El {
    let header = row()
        .w_full()
        .gap(12.0)
        .center_y()
        .justify_between()
        .child(spans(&detail.title, k, true))
        .child(
            text(detail.time.clone())
                .font(tabular(k.ty.small))
                .color(k.c.text_muted),
        );
    col()
        .key("event-card")
        .min_w(CARD_MIN_WIDTH)
        .max_w(CARD_MAX_WIDTH)
        .px(12.0)
        .py(10.0)
        .gap(6.0)
        .child(header)
        .children(detail.lines.iter().map(|line| line_row(line, k)))
}

fn line_row(line: &Line, k: &Kit) -> El {
    row()
        .w_full()
        .gap(8.0)
        .center_y()
        .child(
            row().w(LEAD_WIDTH).child(
                text(line.lead.clone())
                    .font(tabular(k.ty.small))
                    .color(k.c.text_muted),
            ),
        )
        .child(spans(&line.spans, k, false))
}

fn spans(spans: &[Span], k: &Kit, title: bool) -> El {
    let strong_font = if title { k.ty.body_medium } else { k.ty.body };
    row()
        .gap(6.0)
        .center_y()
        .children(spans.iter().map(|span| match span {
            Span::Strong(t) => text(t.clone()).font(strong_font).color(k.c.text_base),
            Span::Muted(t) => text(t.clone()).font(k.ty.body).color(k.c.text_muted),
        }))
}
