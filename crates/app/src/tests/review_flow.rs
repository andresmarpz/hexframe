#![allow(clippy::expect_used)]

use rune::{Key, Runtime};

use super::{assert_seeks_to, key_review_runtime, press, press_char, refresh, review_runtime, tap};
use crate::ui::fake_player::{self, Call};
use crate::ui::{EventFilter, PanelTab, Ui};

fn shown_ids(rt: &Runtime<Ui>) -> Vec<u32> {
    let review = rt.state.review.as_ref().expect("review");
    review.shown_events().map(|e| e.id).collect()
}

#[test]
fn filter_chips_narrow_the_event_list() {
    let mut rt = review_runtime();
    assert_eq!(shown_ids(&rt).len(), 10);
    tap(&mut rt, "filter-deaths");
    assert_eq!(shown_ids(&rt), vec![5]);
    assert!(rt.find("event-5").is_some());
    assert!(rt.find("event-3").is_none());
    tap(&mut rt, "filter-kills");
    assert_eq!(shown_ids(&rt), vec![2, 3, 8]);
    tap(&mut rt, "filter-objectives");
    assert_eq!(shown_ids(&rt), vec![4, 6, 9, 10]);
    tap(&mut rt, "filter-all");
    assert_eq!(shown_ids(&rt).len(), 10);
}

#[test]
fn the_events_tab_counts_the_filtered_events() {
    let mut rt = review_runtime();
    assert!(super::card_text(&rt).contains("Events 10"));
    tap(&mut rt, "filter-objectives");
    assert!(super::card_text(&rt).contains("Events 4 of 10"));
    assert_eq!(
        rt.state.review.as_ref().map(|r| r.filter),
        Some(EventFilter::Objectives)
    );
}

#[test]
fn the_row_at_the_playhead_is_highlighted() {
    let mut rt = review_runtime();
    assert!(rt.find("event-active-1").is_none());
    fake_player::set_time(3.0);
    refresh(&mut rt);
    assert_eq!(
        rt.state.review.as_ref().and_then(|r| r.active_event()),
        Some(3)
    );
    assert!(rt.find("event-active-3").is_some());
    assert!(rt.find("event-active-2").is_none());
    fake_player::set_time(9.5);
    refresh(&mut rt);
    assert!(rt.find("event-active-10").is_some());
    fake_player::set_time(0.0);
}

#[test]
fn the_active_row_follows_the_filter() {
    let mut rt = review_runtime();
    tap(&mut rt, "filter-deaths");
    fake_player::set_time(3.0);
    refresh(&mut rt);
    assert_eq!(
        rt.state.review.as_ref().and_then(|r| r.active_event()),
        None
    );
    fake_player::set_time(5.0);
    refresh(&mut rt);
    assert_eq!(
        rt.state.review.as_ref().and_then(|r| r.active_event()),
        Some(5)
    );
    fake_player::set_time(0.0);
}

#[test]
fn the_tabs_swap_the_panel_body() {
    let mut rt = review_runtime();
    assert!(rt.find("event-1").is_some());
    assert!(rt.find("clip-0").is_none());
    tap(&mut rt, "tab-clips");
    assert_eq!(
        rt.state.review.as_ref().map(|r| r.tab),
        Some(PanelTab::Clips)
    );
    assert!(rt.find("clip-0").is_some());
    assert!(rt.find("event-1").is_none());
    assert!(rt.find("filter-all").is_none());
    tap(&mut rt, "tab-events");
    assert!(rt.find("event-1").is_some());
    assert!(rt.find("clip-0").is_none());
}

#[test]
fn the_clip_holding_the_playhead_is_highlighted() {
    let mut rt = review_runtime();
    tap(&mut rt, "tab-clips");
    fake_player::set_time(3.0);
    refresh(&mut rt);
    assert!(rt.find("clip-active-0").is_some());
    assert!(rt.find("clip-folder-1").is_some());
    fake_player::set_time(0.0);
}

#[test]
fn the_speed_pill_cycles_and_tells_the_player() {
    let mut rt = review_runtime();
    let rate = |rt: &Runtime<Ui>| rt.state.review.as_ref().map(|r| r.speed_rate());
    assert_eq!(rate(&rt), Some(1.0));
    let mut seen = Vec::new();
    for _ in 0..5 {
        tap(&mut rt, "review-speed");
        seen.push(rate(&rt).expect("rate"));
    }
    assert_eq!(seen, vec![1.5, 2.0, 0.25, 0.5, 1.0]);
    let rates: Vec<f32> = fake_player::calls()
        .into_iter()
        .filter_map(|call| match call {
            Call::Rate(r) => Some(r),
            _ => None,
        })
        .collect();
    assert_eq!(rates, seen);
}

#[test]
fn the_speed_survives_a_clip_swap() {
    let mut rt = review_runtime();
    tap(&mut rt, "review-speed");
    tap(&mut rt, "tab-clips");
    fake_player::clear_calls();
    tap(&mut rt, "clip-1");
    assert!(fake_player::calls().contains(&Call::Rate(1.5)));
}

#[test]
fn the_mute_button_toggles_the_player() {
    let mut rt = review_runtime();
    tap(&mut rt, "review-mute");
    assert!(rt.state.review.as_ref().is_some_and(|r| r.muted));
    tap(&mut rt, "review-mute");
    assert_eq!(
        fake_player::calls(),
        vec![Call::Muted(true), Call::Muted(false)]
    );
}

#[test]
fn the_skip_buttons_move_five_seconds() {
    let mut rt = review_runtime();
    fake_player::set_time(6.0);
    tap(&mut rt, "review-forward");
    assert_seeks_to(10.0);
    fake_player::clear_calls();
    tap(&mut rt, "review-rewind");
    assert_seeks_to(1.0);
    fake_player::set_time(0.0);
}

#[test]
fn comma_and_period_step_a_frame_only_while_paused() {
    let mut rt = key_review_runtime();
    fake_player::set_time(2.0);
    press_char(&mut rt, ".");
    assert_seeks_to(2.0 + 1.0 / 60.0);
    fake_player::clear_calls();
    press_char(&mut rt, ",");
    assert_seeks_to(2.0 - 1.0 / 60.0);
    fake_player::clear_calls();
    fake_player::set_playing(true);
    press_char(&mut rt, ".");
    assert!(fake_player::calls().is_empty());
    fake_player::set_playing(false);
    fake_player::set_time(0.0);
}

#[test]
fn m_mutes_the_player() {
    let mut rt = key_review_runtime();
    press_char(&mut rt, "m");
    assert_eq!(fake_player::calls(), vec![Call::Muted(true)]);
}

#[test]
fn theater_has_the_events_drawer_behind_a_toggle() {
    let mut rt = key_review_runtime();
    press_char(&mut rt, "f");
    assert!(rt.find("theater-drawer").is_none());
    tap(&mut rt, "theater-events");
    assert!(rt.find("theater-drawer").is_some());
    rt.frame(1.0);
    fake_player::clear_calls();
    tap(&mut rt, "event-6");
    assert_seeks_to(0.5);
    assert!(rt.find("theater-drawer").is_some());
    tap(&mut rt, "theater-events");
    rt.frame(1.0);
    assert!(rt.find("theater-drawer").is_none());
}

#[test]
fn leaving_theater_closes_the_drawer() {
    let mut rt = key_review_runtime();
    press_char(&mut rt, "f");
    tap(&mut rt, "theater-events");
    press(&mut rt, Key::Escape);
    assert!(rt.state.review.as_ref().is_some_and(|r| !r.drawer));
}

#[test]
fn theater_top_bar_names_the_game_and_exits() {
    let mut rt = key_review_runtime();
    press_char(&mut rt, "f");
    let text = super::card_text(&rt);
    assert!(text.contains("Ahri · Summoner's Rift · Victory"), "{text}");
    tap(&mut rt, "theater-exit");
    assert!(!rt.state.in_theater());
}

#[test]
fn the_fullscreen_toggle_sits_at_the_far_right_of_the_transport() {
    let mut rt = review_runtime();
    let toggle = super::rect(&rt, "review-theater");
    for key in ["review-speed", "review-mute", "review-play"] {
        assert!(super::rect(&rt, key)[0] < toggle[0], "{key}");
    }
    tap(&mut rt, "review-theater");
    let toggle = super::rect(&rt, "theater-shrink");
    assert!(super::rect(&rt, "review-mute")[0] < toggle[0]);
}

fn clips_only_entry() -> greyscreen_engine::RecordingEntry {
    let mut entry = crate::ui::review_entry();
    entry.meta.video = String::new();
    entry.meta.clips.truncate(1);
    entry.meta.clips[0].file = "review.mp4".into();
    entry
}

#[test]
fn a_game_without_its_video_opens_its_first_clip() {
    fake_player::clear_calls();
    let mut ui = Ui::preview();
    ui.player_factory = fake_player::factory;
    ui.open_review_entry(clips_only_entry());
    let review = ui.review.as_ref().expect("review");
    assert_eq!(review.source, crate::ui::Source::Clip(0));
    assert!(review.player.is_some());
    assert!(
        matches!(fake_player::calls().first(), Some(Call::Open(p)) if p.ends_with("review.mp4"))
    );
}

#[test]
fn a_game_with_no_video_and_no_clips_says_so() {
    let mut entry = clips_only_entry();
    entry.meta.clips.clear();
    let mut ui = Ui::preview();
    ui.player_factory = fake_player::factory;
    ui.open_review_entry(entry);
    let mut rt = super::runtime_with(ui);
    rt.frame(1.0 / 60.0);
    let review = rt.state.review.as_ref().expect("review");
    assert!(review.player.is_none());
    assert!(
        super::visible_texts(&rt)
            .iter()
            .any(|t| t == "No video for this game")
    );
}
