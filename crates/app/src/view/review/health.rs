use greyscreen_core::health::{Finding, Level, RecordingHealth, no_health_text};
use rune::{El, col, row, text};
use rune_kit::{ControlSize, IconButton, IconButtonVariant, Kit, Placement, Popover, StateColors};

use super::{icons, tabular, tone_pill};
use crate::ui::{ReviewState, Ui};

const BODY_WIDTH: f32 = 288.0;
const BODY_GAP: f32 = 10.0;
const FINDING_GAP: f32 = 2.0;
const PILL_WIDTH: f32 = 76.0;

fn popover(open: bool) -> Popover {
    Popover::new("health")
        .title("Recording health")
        .placement(Placement::BottomEnd)
        .gutter(4.0)
        .open(open)
        .on_close(|ui: &mut Ui| ui.review_health(false))
}

pub(super) fn trigger(review: &ReviewState, k: &Kit) -> El {
    let open = review.health_open;
    let mut button = IconButton::new(&icons::INFO)
        .label("Recording health")
        .variant(IconButtonVariant::Ghost)
        .control_size(ControlSize::Regular)
        .on_press(move |ui: &mut Ui| ui.review_health(!open));
    if !open {
        button = button.help("Recording health");
    }
    popover(open)
        .trigger(button.build(k).key("review-health"))
        .build(k, &mut Vec::new())
}

pub(crate) fn overlay(review: &ReviewState, k: &Kit, layer: &mut Vec<El>) {
    if review.health_open {
        popover(true)
            .child(body(review.entry.meta.health.as_ref(), k))
            .build(k, layer);
    }
}

fn tone(k: &Kit, level: Level) -> StateColors {
    match level {
        Level::Ok => k.c.success,
        Level::Warn => k.c.warning,
        Level::Bad => k.c.danger,
    }
}

fn body(health: Option<&RecordingHealth>, k: &Kit) -> El {
    let Some(health) = health else {
        return col().key("health-body").w(BODY_WIDTH).child(
            text(no_health_text())
                .key("health-empty")
                .wrap()
                .w_full()
                .font(k.ty.body)
                .color(k.c.text_muted),
        );
    };
    let mut body = col().key("health-body").w(BODY_WIDTH).gap(BODY_GAP).child(
        text(health.summary_line())
            .key("health-summary")
            .wrap()
            .w_full()
            .font(tabular(k.ty.body_medium))
            .color(k.c.text_strong),
    );
    for (index, finding) in health.findings().iter().enumerate() {
        body = body.child(finding_row(finding, index, k));
    }
    body
}

fn finding_row(finding: &Finding, index: usize, k: &Kit) -> El {
    let pill = tone_pill(k, finding.level.label(), tone(k, finding.level))
        .w(PILL_WIDTH)
        .key(format!("health-level-{index}"));
    row()
        .key(format!("health-finding-{index}"))
        .w_full()
        .gap(BODY_GAP)
        .child(pill)
        .child(
            col()
                .grow(1.0)
                .shrink()
                .gap(FINDING_GAP)
                .child(
                    text(finding.area.label())
                        .font(k.ty.small_medium)
                        .color(k.c.text_base),
                )
                .child(
                    text(finding.text.clone())
                        .wrap()
                        .w_full()
                        .font(tabular(k.ty.small))
                        .color(k.c.text_muted),
                ),
        )
}
