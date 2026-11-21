use super::{Scene, death, game, idle};
use crate::ui::PanelTab;

pub(super) const SCENES: &[Scene] = &[
    ("review", game),
    ("review-clips", |ui| {
        game(ui);
        if let Some(review) = &mut ui.review {
            review.tab = PanelTab::Clips;
        }
    }),
    ("review-filter-menu", |ui| {
        game(ui);
        ui.review_filter_menu(true);
    }),
    ("review-filter-custom", |ui| {
        game(ui);
        if let Some(review) = &mut ui.review {
            review.set_filter(
                review
                    .default_filter
                    .toggled(greyscreen_core::categories::EventCategory::TeammateKills)
                    .toggled(greyscreen_core::categories::EventCategory::Structures),
            );
            review.filter_menu = true;
        }
    }),
    ("review-speed-menu", |ui| {
        game(ui);
        ui.review_speed_menu(true);
    }),
    ("review-volume", |ui| {
        game(ui);
        ui.review_set_volume(0.6);
        ui.review_volume_hover(|hover| hover.button = true);
    }),
    ("review-theater", |ui| {
        game(ui);
        if let Some(review) = &mut ui.review {
            review.theater = true;
        }
    }),
    ("death-review-start", |ui| {
        idle(ui);
        death::demo(ui);
    }),
    ("death-review", |ui| {
        idle(ui);
        death::mid(ui);
    }),
    ("death-review-summary", |ui| {
        idle(ui);
        death::summary(ui);
    }),
];
