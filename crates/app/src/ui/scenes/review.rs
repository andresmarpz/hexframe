use super::{Scene, death, game, idle};
use crate::ui::{PanelTab, Ui, preview};

fn set_health(ui: &mut Ui, health: Option<greyscreen_core::health::RecordingHealth>) {
    if let Some(review) = &mut ui.review {
        review.entry.meta.health = health;
    }
}

fn seek_preview(ui: &mut Ui, fraction: f64, with_tile: bool) {
    game(ui);
    if let Some(review) = &mut ui.review {
        if !with_tile {
            review.storyboard = None;
        }
        let at = review.duration() * fraction;
        review.hover_seek(Some(at));
    }
}

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
    ("review-health", |ui| {
        game(ui);
        set_health(ui, Some(preview::healthy()));
        ui.review_health(true);
    }),
    ("review-health-warnings", |ui| {
        game(ui);
        set_health(ui, Some(preview::struggling()));
        ui.review_health(true);
    }),
    ("review-health-none", |ui| {
        game(ui);
        ui.review_health(true);
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
    ("review-seek-preview", |ui| seek_preview(ui, 0.4, true)),
    ("review-seek-preview-time", |ui| {
        seek_preview(ui, 0.4, false)
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
