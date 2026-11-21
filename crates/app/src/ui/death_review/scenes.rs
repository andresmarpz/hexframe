use greyscreen_core::review::DeathTag;

use super::fixture::death_entry;
use crate::ui::Ui;

const MID_DEATH_INDEX: usize = 2;
const MID_DEATH_OFFSET_SECS: f64 = 0.6;
const WINDOW_BAR_WIDTH: f32 = 832.0;
const SUMMARY_BAR_WIDTH: f32 = 580.0;

fn open_with_reviews(ui: &mut Ui) {
    ui.open_review_entry(death_entry());
    let Some(review) = &mut ui.review else { return };
    let data = &mut review.entry.meta.review;
    for (id, tags) in [
        (2, &[DeathTag::OutOfPosition, DeathTag::NoVision][..]),
        (3, &[DeathTag::Ganked]),
        (4, &[DeathTag::Teamfight, DeathTag::BadTrade]),
        (7, &[DeathTag::Overextended, DeathTag::NoVision]),
        (9, &[DeathTag::BadTrade]),
    ] {
        for tag in tags {
            data.toggle_tag(id, *tag);
        }
    }
    data.set_note(
        4,
        "Walked into three of them. Should have waited for the wave.",
    );
    data.set_note(7, "Pushed past river with no ward and Zed missing.");
}

pub(crate) fn scene_mid(ui: &mut Ui) {
    open_with_reviews(ui);
    ui.review_deaths();
    ui.death_go(MID_DEATH_INDEX);
    let Some(review) = &mut ui.review else { return };
    review.track_width = WINDOW_BAR_WIDTH;
    review.toggle_slow_mo();
    let Some(death) = review.death.as_ref().and_then(|d| d.current()) else {
        return;
    };
    let at = death.rec_secs + MID_DEATH_OFFSET_SECS;
    review.seek(at);
    if let Some(player) = &review.player {
        player.pause();
    }
}

pub(crate) fn scene_summary(ui: &mut Ui) {
    open_with_reviews(ui);
    ui.review_deaths();
    ui.death_finish();
    if let Some(review) = &mut ui.review {
        review.track_width = SUMMARY_BAR_WIDTH;
    }
}

pub(crate) fn scene_demo(ui: &mut Ui) {
    ui.open_review_entry(death_entry());
}
