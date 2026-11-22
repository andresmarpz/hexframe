use rune::{Key, Runtime};

use super::{assert_seeks_to, key_review_runtime, press, press_char, refresh, review_runtime, tap};
use crate::ui::fake_player::{self, Call};
use crate::ui::{PanelTab, Ui};

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
    assert!(rt.find("event-filter").is_none());
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

fn rates() -> Vec<f32> {
    fake_player::calls()
        .into_iter()
        .filter_map(|call| match call {
            Call::Rate(r) => Some(r),
            _ => None,
        })
        .collect()
}

fn speed_open(rt: &Runtime<Ui>) -> bool {
    rt.state.review.as_ref().is_some_and(|r| r.speed_menu)
}

#[test]
fn picking_a_speed_from_the_menu_sets_the_rate_and_closes_the_menu() {
    let mut rt = review_runtime();
    let rate = |rt: &Runtime<Ui>| rt.state.review.as_ref().map(|r| r.speed_rate());
    assert_eq!(rate(&rt), Some(1.0));
    tap(&mut rt, "review-speed");
    assert!(speed_open(&rt));
    for index in 0..7 {
        assert!(rt.find(&format!("speed-menu-item{index}")).is_some());
    }
    tap(&mut rt, "speed-menu-item5");
    assert_eq!(rate(&rt), Some(1.5));
    assert!(!speed_open(&rt));
    assert!(rt.find("speed-menu-item5").is_none());
    assert_eq!(rates(), vec![1.5]);
}

#[test]
fn normal_speed_is_one_pick_from_any_speed() {
    for index in [0, 1, 2, 4, 5, 6] {
        let mut rt = review_runtime();
        rt.update(move |ui, _| ui.review_set_speed(index));
        rt.frame(1.0 / 60.0);
        tap(&mut rt, "review-speed");
        tap(&mut rt, "speed-menu-item3");
        assert_eq!(
            rt.state.review.as_ref().map(|r| r.speed_rate()),
            Some(1.0),
            "from index {index}"
        );
    }
}

#[test]
fn the_speed_trigger_shows_the_current_speed() {
    let mut rt = review_runtime();
    rt.update(|ui, _| ui.review_set_speed(0));
    rt.frame(1.0 / 60.0);
    assert!(super::card_text(&rt).contains("0.25×"));
}

#[test]
fn escape_closes_the_speed_menu_without_changing_it() {
    let mut rt = review_runtime();
    tap(&mut rt, "review-speed");
    press(&mut rt, Key::Escape);
    assert!(!speed_open(&rt));
    assert_eq!(rt.state.review.as_ref().map(|r| r.speed_rate()), Some(1.0));
}

#[test]
fn the_speed_survives_a_clip_swap() {
    let mut rt = review_runtime();
    tap(&mut rt, "review-speed");
    tap(&mut rt, "speed-menu-item5");
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
    let theater = super::rect(&rt, "review-theater");
    let full = super::rect(&rt, "review-fullscreen");
    for key in [
        "review-export",
        "review-speed",
        "review-mute",
        "review-play",
    ] {
        assert!(super::rect(&rt, key)[0] < theater[0], "{key}");
    }
    assert!(theater[0] < full[0]);
    tap(&mut rt, "review-fullscreen");
    assert!(rt.find("review-theater").is_none());
    let toggle = super::rect(&rt, "theater-shrink");
    assert!(super::rect(&rt, "review-mute")[0] < toggle[0]);
}

#[test]
fn t_and_theater_button_toggle_windowed_theater_without_os_fullscreen() {
    let mut rt = key_review_runtime();
    press_char(&mut rt, "t");
    assert!(rt.state.in_theater());
    assert!(!rt.state.window_full_screen);
    press_char(&mut rt, "t");
    assert!(!rt.state.in_theater());
    assert!(!rt.state.window_full_screen);

    tap(&mut rt, "review-theater");
    assert!(rt.state.in_theater());
    assert!(!rt.state.window_full_screen);
    press(&mut rt, Key::Escape);
    assert!(!rt.state.in_theater());
    assert!(!rt.state.window_full_screen);
}

#[test]
fn f_and_fullscreen_button_toggle_os_fullscreen_theater() {
    let mut rt = key_review_runtime();
    press_char(&mut rt, "f");
    assert!(rt.state.in_theater());
    assert!(rt.state.window_full_screen);
    press_char(&mut rt, "f");
    assert!(!rt.state.in_theater());
    assert!(!rt.state.window_full_screen);

    tap(&mut rt, "review-fullscreen");
    assert!(rt.state.in_theater());
    assert!(rt.state.window_full_screen);
    press(&mut rt, Key::Escape);
    assert!(!rt.state.in_theater());
    assert!(!rt.state.window_full_screen);
}

#[test]
fn f_in_windowed_theater_upgrades_to_os_fullscreen() {
    let mut rt = key_review_runtime();
    press_char(&mut rt, "t");
    assert!(rt.state.in_theater());
    assert!(!rt.state.window_full_screen);
    press_char(&mut rt, "f");
    assert!(rt.state.in_theater());
    assert!(rt.state.window_full_screen);
    press(&mut rt, Key::Escape);
    assert!(!rt.state.in_theater());
    assert!(!rt.state.window_full_screen);
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

fn right_edge(rt: &Runtime<Ui>, key: &str) -> f32 {
    let r = super::nav_rect_of(rt, key);
    r[0] + r[2]
}

#[test]
fn the_review_toolbar_never_runs_the_title_under_the_buttons() {
    let mut rt = review_runtime();
    rt.update(|ui, _| ui.settings.sidebar_collapsed = true);
    for width in [1000.0, 1100.0, 1280.0, 1600.0] {
        rt.set_viewport((width, 600.0), 1.0);
        super::settle(&mut rt);
        let trailing = super::nav_rect_of(&rt, "trailing");
        for key in ["review-header", "review-stats", "review-result"] {
            assert!(
                right_edge(&rt, key) <= trailing[0] + 0.5,
                "{key} at {width}: {} over {}",
                right_edge(&rt, key),
                trailing[0]
            );
        }
        for key in ["review-deaths", "review-more", "review-toggle-inspector"] {
            let r = super::nav_rect_of(&rt, key);
            assert!(
                r[0] >= trailing[0] - 0.5 && r[2] >= 24.0,
                "{key} at {width}"
            );
        }
    }
}

#[test]
fn toggling_the_inspector_hides_the_panel_widens_the_player_and_keeps_clip_notice() {
    let mut rt = key_review_runtime();
    let before = super::rect(&rt, "review-player")[2];
    assert!(rt.find("review-panel").is_some());
    tap(&mut rt, "tab-clips");
    tap(&mut rt, "clip-1");
    assert!(rt.find("clip-notice").is_some());
    assert!(rt.find("review-game").is_some());

    tap(&mut rt, "review-toggle-inspector");
    assert!(rt.state.settings.review_inspector_hidden);
    assert!(rt.find("review-panel").is_none());
    let widened = super::rect(&rt, "review-player")[2];
    assert!(widened > before + 200.0, "{before} -> {widened}");
    assert!(rt.find("clip-notice").is_some());
    assert!(rt.find("review-game").is_some());

    press_char(&mut rt, "i");
    assert!(!rt.state.settings.review_inspector_hidden);
    assert!(rt.find("review-panel").is_some());
    press_char(&mut rt, "i");
    assert!(rt.state.settings.review_inspector_hidden);
    assert!(rt.find("review-panel").is_none());
}

#[test]
fn toggling_the_games_list_hides_and_restores_it_and_persists() {
    let mut rt = key_review_runtime();
    assert!(rt.find("games-list").is_some());
    tap(&mut rt, "review-toggle-games-list");
    assert!(rt.state.settings.games_list_hidden);
    assert!(rt.find("games-list").is_none());
    press_char(&mut rt, "g");
    assert!(!rt.state.settings.games_list_hidden);
    assert!(rt.find("games-list").is_some());
    press_char(&mut rt, "g");
    assert!(rt.state.settings.games_list_hidden);
    assert!(rt.find("games-list").is_none());
}

#[test]
fn death_review_hides_the_games_list_and_restores_it_when_done() {
    let mut rt = key_review_runtime();
    assert!(rt.find("games-list").is_some());
    assert!(rt.find("review-more").is_some());
    assert!(!rt.state.settings.games_list_hidden);
    press_char(&mut rt, "d");
    assert!(rt.find("games-list").is_none());
    assert!(rt.find("review-toggle-games-list").is_none());
    assert!(rt.find("review-toggle-inspector").is_none());
    assert!(rt.find("review-more").is_none());
    assert!(!rt.state.settings.games_list_hidden);
    press(&mut rt, Key::Escape);
    press(&mut rt, Key::Escape);
    assert!(rt.find("games-list").is_some());
    assert!(rt.find("review-more").is_some());
}

#[test]
fn theater_drawer_switches_between_events_and_clips() {
    let mut rt = key_review_runtime();
    press_char(&mut rt, "t");
    tap(&mut rt, "theater-events");
    rt.frame(1.0);
    assert!(rt.find("tab-clips").is_some());
    assert!(rt.find("event-1").is_some());
    tap(&mut rt, "tab-clips");
    assert_eq!(
        rt.state.review.as_ref().map(|r| r.tab),
        Some(PanelTab::Clips)
    );
    assert!(rt.find("clip-0").is_some());
    assert!(rt.find("event-1").is_none());
    tap(&mut rt, "tab-events");
    assert!(rt.find("event-1").is_some());
    assert!(rt.find("clip-0").is_none());
}
