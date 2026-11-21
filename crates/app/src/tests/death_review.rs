use greyscreen_core::library::{read_meta, write_meta};
use greyscreen_core::review::DeathTag;
use rune::{Key, Runtime};

use super::{card_text, press, press_char, refresh, runtime_with, tap};
use crate::ui::fake_player::{self, Call};
use crate::ui::library::format::{death_hint, review_progress};
use crate::ui::{Detail, Ui, death_entry};

fn death_runtime() -> Runtime<Ui> {
    fake_player::set_duration(60.0);
    let mut ui = Ui::preview();
    ui.player_factory = fake_player::factory;
    let mut rt = runtime_with(ui);
    rt.on_key = Some(Ui::on_key);
    rt.update(|ui, _| ui.open_review_entry(death_entry()));
    rt.frame(1.0 / 60.0);
    rt.frame(2.0 / 60.0);
    fake_player::set_time(0.0);
    fake_player::set_playing(false);
    fake_player::clear_calls();
    rt
}

fn entered() -> Runtime<Ui> {
    let mut rt = death_runtime();
    press_char(&mut rt, "d");
    rt
}

fn index(rt: &Runtime<Ui>) -> Option<usize> {
    rt.state.review.as_ref()?.death.as_ref().map(|d| d.index)
}

fn seeks() -> Vec<f64> {
    fake_player::calls()
        .into_iter()
        .filter_map(|call| match call {
            Call::Seek(at) => Some(at),
            _ => None,
        })
        .collect()
}

fn tick(rt: &mut Runtime<Ui>, time: f64, playing: bool) {
    fake_player::set_time(time);
    fake_player::set_playing(playing);
    rt.update(|ui, tx| {
        ui.poll(1.0, tx);
    });
    refresh(rt);
}

fn review(rt: &Runtime<Ui>) -> &crate::ui::ReviewState {
    rt.state.review.as_ref().expect("review")
}

#[test]
fn the_button_counts_my_deaths_and_the_list_is_in_time_order() {
    let rt = death_runtime();
    let ids: Vec<u32> = review(&rt)
        .death_list()
        .iter()
        .map(|d| d.event_id)
        .collect();
    assert_eq!(ids, vec![2, 3, 4, 6, 7, 8, 9]);
    assert!(card_text(&rt).contains("Review deaths (7)"));
}

#[test]
fn a_game_without_deaths_has_no_button() {
    let mut rt = death_runtime();
    rt.update(|ui, _| {
        if let Some(review) = &mut ui.review {
            review
                .entry
                .meta
                .events
                .retain(|e| !e.highlights.iter().any(|h| h == "MyDeath"));
        }
    });
    rt.frame(1.0 / 60.0);
    assert!(rt.find("review-deaths").is_none());
    press_char(&mut rt, "d");
    assert!(index(&rt).is_none());
}

#[test]
fn the_button_and_the_d_key_both_enter_the_mode() {
    let mut rt = death_runtime();
    tap(&mut rt, "review-deaths");
    assert_eq!(index(&rt), Some(0));
    let mut rt = death_runtime();
    press_char(&mut rt, "d");
    assert_eq!(index(&rt), Some(0));
}

#[test]
fn entering_plays_from_twelve_seconds_before_the_first_death() {
    let _rt = entered();
    assert_eq!(seeks(), vec![2.0]);
    let calls = fake_player::calls();
    assert!(calls.contains(&Call::Rate(1.0)));
    assert_eq!(calls.last(), Some(&Call::Play));
}

#[test]
fn the_header_names_the_killer_and_the_assisters() {
    let mut rt = entered();
    let title = card_text(&rt);
    assert!(
        title.contains("Death 1 of 7 · 10:06 · Killed by Zed (with Lee Sin)"),
        "{title}"
    );
    press_char(&mut rt, "n");
    press_char(&mut rt, "n");
    let title = card_text(&rt);
    assert!(
        title.contains("Death 3 of 7 · 10:23 · Killed by Jinx (with Lux and Caitlyn)"),
        "{title}"
    );
    for _ in 0..2 {
        press_char(&mut rt, "n");
    }
    assert!(card_text(&rt).contains("Death 5 of 7 · 10:38 · Killed by Zed"));
    press_char(&mut rt, "p");
    assert!(card_text(&rt).contains("Death 4 of 7 · 10:32 · Killed by a minion"));
}

#[test]
fn next_and_previous_walk_the_deaths_and_stop_at_the_ends() {
    let mut rt = entered();
    tap(&mut rt, "death-prev");
    assert_eq!(index(&rt), Some(0));
    tap(&mut rt, "death-next");
    assert_eq!(index(&rt), Some(1));
    assert_eq!(seeks().last(), Some(&10.0));
    for _ in 0..10 {
        tap(&mut rt, "death-next");
    }
    assert_eq!(index(&rt), Some(6));
    tap(&mut rt, "death-prev");
    assert_eq!(index(&rt), Some(5));
}

#[test]
fn arrow_keys_and_n_p_navigate() {
    let mut rt = entered();
    press(&mut rt, Key::Right);
    assert_eq!(index(&rt), Some(1));
    press(&mut rt, Key::Right);
    assert_eq!(index(&rt), Some(2));
    press(&mut rt, Key::Left);
    assert_eq!(index(&rt), Some(1));
    press_char(&mut rt, "n");
    assert_eq!(index(&rt), Some(2));
    press_char(&mut rt, "p");
    press_char(&mut rt, "p");
    press_char(&mut rt, "p");
    assert_eq!(index(&rt), Some(0));
}

#[test]
fn the_pips_jump_to_a_death() {
    let mut rt = entered();
    tap(&mut rt, "death-pip-4");
    assert_eq!(index(&rt), Some(4));
    assert_eq!(seeks().last(), Some(&34.0));
}

#[test]
fn keys_do_nothing_outside_death_review() {
    let mut rt = death_runtime();
    press_char(&mut rt, "n");
    press_char(&mut rt, "p");
    press_char(&mut rt, "r");
    assert!(index(&rt).is_none());
    assert!(fake_player::calls().is_empty());
}

#[test]
fn the_player_pauses_three_seconds_after_the_death() {
    let mut rt = entered();
    fake_player::clear_calls();
    tick(&mut rt, 10.0, true);
    assert!(!fake_player::calls().contains(&Call::Pause));
    tick(&mut rt, 16.9, true);
    assert!(!fake_player::calls().contains(&Call::Pause));
    tick(&mut rt, 17.0, true);
    assert!(fake_player::calls().contains(&Call::Pause));
    assert!(review(&rt).death.as_ref().is_some_and(|d| d.finished));
}

#[test]
fn a_stale_clock_from_the_previous_death_does_not_pause_the_next() {
    let mut rt = entered();
    tick(&mut rt, 5.0, true);
    tick(&mut rt, 16.0, true);
    press_char(&mut rt, "p");
    tick(&mut rt, 17.0, true);
    press_char(&mut rt, "n");
    fake_player::clear_calls();
    tick(&mut rt, 17.0, true);
    assert!(!fake_player::calls().contains(&Call::Pause));
    tick(&mut rt, 10.5, true);
    tick(&mut rt, 25.0, true);
    assert!(fake_player::calls().contains(&Call::Pause));
}

#[test]
fn replay_restarts_the_window() {
    let mut rt = entered();
    tick(&mut rt, 17.0, true);
    assert!(review(&rt).death.as_ref().is_some_and(|d| d.finished));
    fake_player::clear_calls();
    press_char(&mut rt, "r");
    assert_eq!(seeks(), vec![2.0]);
    assert_eq!(fake_player::calls().last(), Some(&Call::Play));
    assert!(review(&rt).death.as_ref().is_some_and(|d| !d.finished));
    tick(&mut rt, 17.0, true);
    fake_player::clear_calls();
    tap(&mut rt, "death-replay");
    assert_eq!(seeks(), vec![2.0]);
}

#[test]
fn space_after_the_window_ends_replays_it() {
    let mut rt = entered();
    tick(&mut rt, 17.0, true);
    fake_player::clear_calls();
    fake_player::set_playing(false);
    press(&mut rt, Key::Text(" ".into()));
    assert_eq!(seeks(), vec![2.0]);
}

#[test]
fn slow_mo_is_off_by_default() {
    let mut rt = entered();
    fake_player::clear_calls();
    tick(&mut rt, 5.0, true);
    tick(&mut rt, 14.0, true);
    assert!(!fake_player::calls().contains(&Call::Rate(0.5)));
}

#[test]
fn slow_mo_halves_the_speed_for_the_last_four_seconds() {
    let mut rt = entered();
    tap(&mut rt, "death-slow-mo");
    assert!(review(&rt).death.as_ref().is_some_and(|d| d.slow_mo));
    tick(&mut rt, 5.0, true);
    fake_player::clear_calls();
    tick(&mut rt, 12.9, true);
    assert!(!fake_player::calls().contains(&Call::Rate(0.5)));
    tick(&mut rt, 13.0, true);
    assert!(fake_player::calls().contains(&Call::Rate(0.5)));
    fake_player::clear_calls();
    tick(&mut rt, 17.0, true);
    assert!(fake_player::calls().contains(&Call::Rate(1.0)));
}

#[test]
fn turning_slow_mo_off_restores_the_speed() {
    let mut rt = entered();
    tap(&mut rt, "death-slow-mo");
    tick(&mut rt, 5.0, true);
    tick(&mut rt, 14.0, true);
    fake_player::clear_calls();
    tap(&mut rt, "death-slow-mo");
    assert!(fake_player::calls().contains(&Call::Rate(1.0)));
}

#[test]
fn replaying_goes_back_to_normal_speed() {
    let mut rt = entered();
    tap(&mut rt, "death-slow-mo");
    tick(&mut rt, 5.0, true);
    tick(&mut rt, 14.0, true);
    fake_player::clear_calls();
    press_char(&mut rt, "r");
    assert!(fake_player::calls().contains(&Call::Rate(1.0)));
}

#[test]
fn tag_chips_toggle_for_the_current_death_only() {
    let mut rt = entered();
    tap(&mut rt, "death-tag-no-vision");
    tap(&mut rt, "death-tag-ganked");
    assert!(review(&rt).entry.meta.review.has_tag(2, DeathTag::NoVision));
    assert!(review(&rt).entry.meta.review.has_tag(2, DeathTag::Ganked));
    press_char(&mut rt, "n");
    assert!(!review(&rt).entry.meta.review.has_tag(3, DeathTag::NoVision));
    tap(&mut rt, "death-tag-bad-trade");
    press_char(&mut rt, "p");
    assert!(review(&rt).entry.meta.review.has_tag(2, DeathTag::NoVision));
    tap(&mut rt, "death-tag-no-vision");
    assert!(!review(&rt).entry.meta.review.has_tag(2, DeathTag::NoVision));
    assert!(review(&rt).entry.meta.review.has_tag(3, DeathTag::BadTrade));
}

#[test]
fn the_note_follows_the_death_it_belongs_to() {
    let mut rt = entered();
    rt.update(|ui, tx| {
        ui.death_note = rune::TextEdit::new("No ward in river");
        ui.death_note_edited(tx);
    });
    press_char(&mut rt, "n");
    assert_eq!(rt.state.death_note.text, "");
    press_char(&mut rt, "p");
    assert_eq!(rt.state.death_note.text, "No ward in river");
    assert_eq!(review(&rt).entry.meta.review.note(2), "No ward in river");
}

#[test]
fn typing_in_the_note_does_not_trigger_the_keys() {
    let mut rt = entered();
    tap(&mut rt, "death-note");
    let typed = ["n", "d"];
    typed.iter().for_each(|key| press_char(&mut rt, key));
    assert_eq!(index(&rt), Some(0));
    assert_eq!(rt.state.death_note.text, typed.concat());
}

#[test]
fn tags_and_notes_are_saved_into_game_json_and_come_back() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut entry = death_entry();
    std::fs::copy(
        entry.folder.join(&entry.meta.video),
        dir.path().join(&entry.meta.video),
    )
    .expect("copy video");
    entry.folder = dir.path().to_owned();
    write_meta(dir.path(), &entry.meta).expect("write meta");

    let mut ui = Ui::preview();
    ui.player_factory = fake_player::factory;
    ui.recordings = vec![entry.clone()];
    let mut rt = runtime_with(ui);
    rt.on_key = Some(Ui::on_key);
    rt.update(|ui, _| ui.open_review_entry(entry.clone()));
    rt.frame(1.0 / 60.0);
    press_char(&mut rt, "d");
    tap(&mut rt, "death-tag-overextended");
    tap(&mut rt, "death-tag-no-vision");
    rt.update(|ui, tx| {
        ui.death_note = rune::TextEdit::new("Pushed without wave");
        ui.death_note_edited(tx);
    });

    let saved = read_meta(dir.path()).expect("read meta");
    assert!(saved.review.has_tag(2, DeathTag::Overextended));
    assert!(saved.review.has_tag(2, DeathTag::NoVision));
    assert_eq!(saved.review.note(2), "Pushed without wave");
    assert_eq!(saved.events, entry.meta.events);
    assert_eq!(rt.state.recordings[0].meta.review, saved.review);

    let mut reopened = Ui::preview();
    reopened.player_factory = fake_player::factory;
    let mut again = entry.clone();
    again.meta = saved;
    reopened.open_review_entry(again);
    reopened.review_deaths();
    assert_eq!(reopened.death_note.text, "Pushed without wave");
}

#[test]
fn nothing_is_written_for_a_recording_without_a_sidecar() {
    let mut rt = entered();
    tap(&mut rt, "death-tag-ganked");
    assert!(!review(&rt).entry.folder.join("game.json").exists());
    assert!(review(&rt).entry.meta.review.has_tag(2, DeathTag::Ganked));
}

#[test]
fn escape_opens_the_summary_then_closes_the_mode() {
    let mut rt = entered();
    tap(&mut rt, "death-tag-no-vision");
    tap(&mut rt, "death-pip-3");
    tap(&mut rt, "death-tag-no-vision");
    tap(&mut rt, "death-tag-ganked");
    fake_player::set_playing(true);
    fake_player::clear_calls();
    press(&mut rt, Key::Escape);
    assert!(review(&rt).death_summary_open());
    assert!(fake_player::calls().contains(&Call::Pause));
    assert!(rt.find("death-summary").is_some());
    assert!(rt.find("death-bar-no-vision").is_some());
    assert!(rt.find("death-bar-ganked").is_some());
    assert!(rt.find("death-bar-teamfight").is_none());
    assert_eq!(rt.state.detail(), Detail::Game);
    press(&mut rt, Key::Escape);
    assert!(review(&rt).death.is_none());
    assert_eq!(rt.state.detail(), Detail::Game);
    press(&mut rt, Key::Escape);
    assert_eq!(rt.state.detail(), Detail::Pick);
}

#[test]
fn the_summary_buttons_close_or_restart_the_review() {
    let mut rt = entered();
    tap(&mut rt, "death-finish");
    tap(&mut rt, "death-again");
    assert_eq!(index(&rt), Some(0));
    assert!(!review(&rt).death_summary_open());
    tap(&mut rt, "death-finish");
    tap(&mut rt, "death-done");
    assert!(review(&rt).death.is_none());
}

#[test]
fn a_dot_on_the_summary_timeline_reopens_that_death() {
    let mut rt = entered();
    tap(&mut rt, "death-finish");
    rt.frame(1.0 / 60.0);
    rt.frame(2.0 / 60.0);
    tap(&mut rt, "death-dot-5");
    assert_eq!(index(&rt), Some(5));
    assert!(!review(&rt).death_summary_open());
}

#[test]
fn entering_from_a_clip_switches_to_the_full_game() {
    let mut rt = death_runtime();
    rt.update(|ui, _| {
        if let Some(review) = &mut ui.review {
            review
                .entry
                .meta
                .clips
                .push(greyscreen_core::library::ClipEntry {
                    file: "deaths.mp4".into(),
                    start_secs: 0.0,
                    end_secs: 10.0,
                    label: "MyKill".into(),
                });
        }
        ui.play_clip(0);
    });
    assert_ne!(
        review(&rt).source,
        crate::ui::Source::Game,
        "precondition: playing a clip"
    );
    press_char(&mut rt, "d");
    assert_eq!(review(&rt).source, crate::ui::Source::Game);
    assert_eq!(index(&rt), Some(0));
}

#[test]
fn the_hint_counts_deaths_and_reviews() {
    let mut entry = death_entry();
    assert_eq!(death_hint(&entry).as_deref(), Some("7 deaths"));
    entry.meta.review.toggle_tag(2, DeathTag::Ganked);
    entry.meta.review.set_note(9, "greedy");
    assert_eq!(death_hint(&entry).as_deref(), Some("7 deaths · 2 reviewed"));
    entry
        .meta
        .events
        .retain(|e| e.id != 3 && e.id != 4 && e.id != 6 && e.id != 7 && e.id != 8 && e.id != 9);
    assert_eq!(death_hint(&entry).as_deref(), Some("1 death · 1 reviewed"));
    entry.meta.events.retain(|e| e.id != 2);
    assert_eq!(death_hint(&entry), None);
}

#[test]
fn review_progress_reads_k_of_n_and_hides_without_deaths() {
    let mut entry = death_entry();
    assert_eq!(review_progress(&entry).as_deref(), Some("0 of 7 reviewed"));
    entry.meta.review.toggle_tag(2, DeathTag::Ganked);
    assert_eq!(review_progress(&entry).as_deref(), Some("1 of 7 reviewed"));
    entry.meta.events.retain(|e| e.id == 2);
    assert_eq!(review_progress(&entry).as_deref(), Some("1 of 1 reviewed"));
    entry.meta.events.clear();
    assert_eq!(review_progress(&entry), None);
}
