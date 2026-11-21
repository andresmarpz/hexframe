use greyscreen_core::review::DeathTag;

use super::fixture::death_entry;
use crate::ui::Ui;

const MID_DEATH_INDEX: usize = 2;
const MID_DEATH_OFFSET_SECS: f64 = 0.6;
const SUMMARY_BAR_WIDTH: f32 = 580.0;

fn show_in_list(ui: &mut Ui) {
    let Some(review) = &ui.review else { return };
    let meta = review.entry.meta.clone();
    if let Some(first) = ui.recordings.first_mut() {
        first.meta = meta;
    }
}

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
    show_in_list(ui);
}

pub(crate) fn scene_mid(ui: &mut Ui) {
    open_with_reviews(ui);
    ui.review_deaths();
    ui.death_go(MID_DEATH_INDEX);
    let Some(review) = &mut ui.review else { return };
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
    show_in_list(ui);
}

pub(crate) fn scene_browse(ui: &mut Ui) {
    let entry = death_entry();
    let Some(first) = ui.recordings.first_mut() else {
        return;
    };
    let source = entry.folder.join(&entry.meta.video);
    let target = first.folder.join(&entry.meta.video);
    if !target.is_file() {
        drop(std::fs::copy(source, target));
    }
    first.meta = entry.meta;
    for rec in ui.recordings.iter().skip(1) {
        crate::ui::preview_thumbs::copy_playable(&rec.folder, rec);
    }
}
