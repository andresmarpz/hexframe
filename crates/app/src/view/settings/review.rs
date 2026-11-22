use greyscreen_core::categories::{CategorySet, EventCategory};
use rune::{El, col, row};
use rune_kit::{Button, ButtonVariant, Checkbox, ControlSize, Kit};

use super::stacked_row;
use crate::ui::Ui;

const EVENTS_NOTE: &str = "The events listed and marked on the timeline when you open a game. You can still change them for one game.";
const COLUMNS: usize = 2;

pub(super) fn render(s: &Ui, k: &Kit) -> El {
    stacked_row(
        k,
        "review-events-row",
        "Events shown",
        Some(EVENTS_NOTE),
        categories(s, k),
    )
}

fn categories(s: &Ui, k: &Kit) -> El {
    let shown = s.settings.review_events;
    let mut grid = col().key("review-events").w_full().gap(6.0);
    for line in EventCategory::ALL.chunks(COLUMNS) {
        let mut cells = row().w_full();
        for category in line {
            let category = *category;
            let cell = Checkbox::new(shown.contains(category))
                .label(category.label())
                .on_toggle(move |ui: &mut Ui| {
                    ui.change(|s| s.review_events = s.review_events.toggled(category));
                })
                .build(k)
                .key(format!("review-event-{}", category.id()));
            cells = cells.child(col().w_frac(1.0 / COLUMNS as f32).child(cell));
        }
        grid = grid.child(cells);
    }
    grid
}

pub(super) fn reset_button(s: &Ui, k: &Kit) -> El {
    Button::new("Reset to default")
        .variant(ButtonVariant::Ghost)
        .control_size(ControlSize::Small)
        .disabled(s.settings.review_events == CategorySet::recommended())
        .on_press(|ui: &mut Ui| ui.change(|s| s.review_events = CategorySet::recommended()))
        .build(k)
        .key("review-events-reset")
}
