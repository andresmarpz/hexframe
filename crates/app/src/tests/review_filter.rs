use crossbeam_channel::{Receiver, unbounded};
use greyscreen_core::categories::{CategorySet, EventCategory};
use greyscreen_core::library::{read_meta, write_meta};
use greyscreen_core::settings::Settings;
use greyscreen_engine::{EngineCmd, RecordingEntry};
use rune::{Key, KeyEvent, Mods, Runtime};

use super::{
    assert_seeks_to, card_text, press, refresh, review_runtime, review_runtime_showing,
    runtime_with, settle, tap, visible_texts,
};
use crate::ui::fake_player;
use crate::ui::{Ui, review_entry};

const TEAMMATE_KILLS_ITEM: usize = 5;
const SAVE_DEFAULT_ITEM: usize = 18;
const USE_DEFAULT_ITEM: usize = 19;

fn recommended() -> Runtime<Ui> {
    review_runtime_showing(CategorySet::recommended())
}

fn shown_ids(rt: &Runtime<Ui>) -> Vec<u32> {
    let review = rt.state.review.as_ref().expect("review");
    review.shown_events().map(|e| e.id).collect()
}

fn override_of(rt: &Runtime<Ui>) -> Option<CategorySet> {
    rt.state.review.as_ref()?.entry.meta.review.filter
}

fn item(index: usize) -> String {
    format!("filter-menu-item{index}")
}

fn saved_entry() -> (tempfile::TempDir, RecordingEntry) {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut entry = review_entry();
    std::fs::write(dir.path().join(&entry.meta.video), b"").expect("video");
    entry.folder = dir.path().to_owned();
    write_meta(dir.path(), &entry.meta).expect("meta");
    (dir, entry)
}

fn saved_runtime(entry: &RecordingEntry, shown: CategorySet) -> Runtime<Ui> {
    let mut ui = Ui::preview();
    ui.settings.review_events = shown;
    ui.player_factory = fake_player::factory;
    ui.recordings = vec![entry.clone()];
    let mut rt = runtime_with(ui);
    let entry = entry.clone();
    rt.update(move |ui, _| ui.open_review_entry(entry));
    rt.frame(1.0 / 60.0);
    rt.frame(2.0 / 60.0);
    rt
}

fn live_review() -> (Runtime<Ui>, Receiver<EngineCmd>) {
    let (cmd, rx) = unbounded();
    let settings = Settings {
        review_events: CategorySet::recommended(),
        ..Settings::default()
    };
    let mut ui = Ui::live(settings, cmd);
    ui.player_factory = fake_player::factory;
    let mut rt = runtime_with(ui);
    rt.update(|ui, _| ui.open_review_entry(review_entry()));
    rt.frame(1.0 / 60.0);
    while rx.try_recv().is_ok() {}
    (rt, rx)
}

#[test]
fn the_default_hides_other_players_kills_and_minor_flow_events() {
    let rt = recommended();
    assert_eq!(shown_ids(&rt), vec![3, 4, 5, 6, 8, 9, 10]);
    assert!(rt.find("event-3").is_some());
    assert!(rt.find("event-7").is_none());
    assert!(rt.find("event-1").is_none());
}

#[test]
fn the_tab_and_the_button_count_what_is_hidden() {
    let rt = recommended();
    assert!(card_text(&rt).contains("Events 7/10"));
    assert!(card_text(&rt).contains("3 hidden"));
    let all = review_runtime();
    assert!(card_text(&all).contains("Events 10"));
    assert!(!card_text(&all).contains("hidden"));
}

#[test]
fn filter_button_keeps_its_width_and_reset_restores_the_default() {
    let mut rt = recommended();
    let initial_width = super::rect(&rt, "event-filter")[2];
    tap(&mut rt, "event-filter");
    tap(&mut rt, &item(TEAMMATE_KILLS_ITEM));
    press(&mut rt, Key::Escape);
    let changed_width = super::rect(&rt, "event-filter")[2];
    assert!(
        (changed_width - initial_width).abs() < 0.5,
        "{initial_width} -> {changed_width}"
    );
    assert!(rt.find("event-filter-custom").is_some());
    tap(&mut rt, "event-filter-custom");
    assert_eq!(override_of(&rt), None);
    assert_eq!(shown_ids(&rt), vec![3, 4, 5, 6, 8, 9, 10]);
    assert!(rt.find("event-filter-custom").is_none());
    let reset_width = super::rect(&rt, "event-filter")[2];
    assert!(
        (reset_width - initial_width).abs() < 0.5,
        "{initial_width} -> {reset_width}"
    );
}

#[test]
fn the_filter_drives_the_timeline_markers_too() {
    let rt = recommended();
    for id in [3, 4, 5, 6, 8, 9, 10] {
        assert!(rt.find(&format!("marker-{id}")).is_some(), "marker {id}");
    }
    for id in [1, 2, 7] {
        assert!(rt.find(&format!("marker-{id}")).is_none(), "marker {id}");
    }
}

#[test]
fn the_button_opens_a_menu_with_every_category_checked_by_the_filter() {
    let mut rt = recommended();
    assert!(rt.find(&item(0)).is_none());
    tap(&mut rt, "event-filter");
    assert!(rt.state.review.as_ref().is_some_and(|r| r.filter_menu));
    let text = card_text(&rt);
    for category in EventCategory::ALL {
        assert!(text.contains(category.label()), "{}", category.label());
    }
    assert!(text.contains("Save as my default"));
    assert!(text.contains("Use my default"));
}

#[test]
fn ticking_a_category_adds_its_events_to_the_list_and_the_timeline() {
    let mut rt = recommended();
    tap(&mut rt, "event-filter");
    tap(&mut rt, &item(TEAMMATE_KILLS_ITEM));
    assert_eq!(shown_ids(&rt), vec![3, 4, 5, 6, 7, 8, 9, 10]);
    assert!(rt.find("marker-7").is_some());
    assert!(rt.find("event-7").is_some());
    let review = rt.state.review.as_ref().expect("review");
    assert_eq!(review.hidden_events(), 2);
    assert!(review.filter_menu);
}

#[test]
fn a_changed_filter_shows_it_is_not_the_default() {
    let mut rt = recommended();
    assert!(rt.find("event-filter-custom").is_none());
    tap(&mut rt, "event-filter");
    tap(&mut rt, &item(TEAMMATE_KILLS_ITEM));
    assert!(rt.find("event-filter-custom").is_some());
    tap(&mut rt, &item(TEAMMATE_KILLS_ITEM));
    assert!(rt.find("event-filter-custom").is_none());
    assert_eq!(override_of(&rt), None);
}

#[test]
fn the_override_is_saved_with_the_recording_and_comes_back() {
    let (dir, entry) = saved_entry();
    let mut rt = saved_runtime(&entry, CategorySet::recommended());
    tap(&mut rt, "event-filter");
    tap(&mut rt, &item(TEAMMATE_KILLS_ITEM));
    let wanted = CategorySet::recommended().with(EventCategory::TeammateKills, true);

    let on_disk = read_meta(dir.path()).expect("meta");
    assert_eq!(on_disk.review.filter, Some(wanted));
    let global = rt.state.settings.review_events;
    assert_eq!(global, CategorySet::recommended());

    let reopened = RecordingEntry {
        meta: on_disk,
        ..entry
    };
    let rt = saved_runtime(&reopened, CategorySet::recommended());
    assert_eq!(shown_ids(&rt), vec![3, 4, 5, 6, 7, 8, 9, 10]);
    assert!(rt.find("event-filter-custom").is_some());
}

#[test]
fn another_game_keeps_following_the_global_default() {
    let (_dir, entry) = saved_entry();
    let mut rt = saved_runtime(&entry, CategorySet::recommended());
    tap(&mut rt, "event-filter");
    tap(&mut rt, &item(TEAMMATE_KILLS_ITEM));
    rt.update(|ui, _| ui.open_review_entry(review_entry()));
    rt.frame(1.0 / 60.0);
    assert_eq!(shown_ids(&rt), vec![3, 4, 5, 6, 8, 9, 10]);
}

#[test]
fn use_my_default_drops_the_override_and_closes_the_menu() {
    let (dir, entry) = saved_entry();
    let mut rt = saved_runtime(&entry, CategorySet::recommended());
    tap(&mut rt, "event-filter");
    tap(&mut rt, &item(TEAMMATE_KILLS_ITEM));
    tap(&mut rt, &item(USE_DEFAULT_ITEM));
    assert_eq!(shown_ids(&rt), vec![3, 4, 5, 6, 8, 9, 10]);
    assert_eq!(override_of(&rt), None);
    assert!(rt.state.review.as_ref().is_some_and(|r| !r.filter_menu));
    assert_eq!(read_meta(dir.path()).expect("meta").review.filter, None);
}

#[test]
fn save_as_my_default_updates_the_settings_and_sends_them() {
    let (mut rt, rx) = live_review();
    tap(&mut rt, "event-filter");
    tap(&mut rt, &item(TEAMMATE_KILLS_ITEM));
    tap(&mut rt, &item(SAVE_DEFAULT_ITEM));
    let wanted = CategorySet::recommended().with(EventCategory::TeammateKills, true);
    assert_eq!(rt.state.settings.review_events, wanted);
    assert_eq!(override_of(&rt), None);
    assert!(rt.find("event-filter-custom").is_none());
    assert_eq!(shown_ids(&rt), vec![3, 4, 5, 6, 7, 8, 9, 10]);
    let sent: Vec<EngineCmd> = rx.try_iter().collect();
    assert!(
        sent.iter()
            .any(|cmd| matches!(cmd, EngineCmd::ApplySettings(s) if s.review_events == wanted)),
        "{sent:?}"
    );
}

#[test]
fn the_save_and_use_default_items_wait_until_the_filter_differs() {
    let mut rt = recommended();
    tap(&mut rt, "event-filter");
    tap(&mut rt, &item(SAVE_DEFAULT_ITEM));
    tap(&mut rt, &item(USE_DEFAULT_ITEM));
    assert_eq!(override_of(&rt), None);
    assert!(rt.state.review.as_ref().is_some_and(|r| r.filter_menu));
}

#[test]
fn death_review_keeps_every_death_whatever_the_filter_hides() {
    let mut rt = review_runtime_showing(CategorySet::NONE);
    assert!(shown_ids(&rt).is_empty());
    assert!(card_text(&rt).contains("No events match the filter"));
    rt.update(|ui, _| ui.review_deaths());
    rt.frame(1.0 / 60.0);
    let review = rt.state.review.as_ref().expect("review");
    assert_eq!(review.death_list().len(), 1);
    assert!(review.death_active());
}

#[test]
fn next_event_skips_the_events_the_filter_hides() {
    let mut rt = recommended();
    rt.on_key = Some(Ui::on_key);
    fake_player::set_time(0.6);
    refresh(&mut rt);
    fake_player::clear_calls();
    rt.key(KeyEvent {
        key: Key::Right,
        mods: Mods {
            shift: true,
            ..Mods::default()
        },
    });
    rt.frame(1.0 / 60.0);
    assert_seeks_to(2.3);
    fake_player::set_time(0.0);
}

#[test]
fn changing_the_global_default_updates_an_open_review_that_follows_it() {
    let mut rt = recommended();
    rt.update(|ui, _| ui.change(|s| s.review_events = CategorySet::all()));
    rt.frame(1.0 / 60.0);
    assert_eq!(shown_ids(&rt).len(), 10);
    assert_eq!(override_of(&rt), None);
}

#[test]
fn an_override_stays_when_the_global_default_changes() {
    let mut rt = recommended();
    tap(&mut rt, "event-filter");
    tap(&mut rt, &item(TEAMMATE_KILLS_ITEM));
    press(&mut rt, Key::Escape);
    rt.update(|ui, _| ui.change(|s| s.review_events = CategorySet::all()));
    rt.frame(1.0 / 60.0);
    assert_eq!(shown_ids(&rt), vec![3, 4, 5, 6, 7, 8, 9, 10]);
    assert!(rt.find("event-filter-custom").is_some());
}

fn open_review_settings() -> Runtime<Ui> {
    let mut rt = recommended();
    tap(&mut rt, "nav-settings");
    settle(&mut rt);
    tap(&mut rt, "settings-nav-clips");
    rt
}

fn type_search(rt: &mut Runtime<Ui>, text: &str) {
    rt.update(|ui, _| ui.settings_search = rune::TextEdit::new(text));
    settle(rt);
}

#[test]
fn the_review_section_lists_every_category_as_a_checkbox() {
    let rt = open_review_settings();
    assert!(rt.find("review-events-row").is_some());
    for category in EventCategory::ALL {
        let key = format!("review-event-{}", category.id());
        assert!(rt.find(&key).is_some(), "{key}");
    }
    assert!(visible_texts(&rt).iter().any(|t| t == "Teammates' kills"));
}

#[test]
fn ticking_a_checkbox_changes_the_global_default() {
    let mut rt = open_review_settings();
    assert!(
        !rt.state
            .settings
            .review_events
            .contains(EventCategory::EnemyKills)
    );
    tap(&mut rt, "review-event-enemy-kills");
    assert!(
        rt.state
            .settings
            .review_events
            .contains(EventCategory::EnemyKills)
    );
    tap(&mut rt, "review-event-my-kills");
    assert!(
        !rt.state
            .settings
            .review_events
            .contains(EventCategory::MyKills)
    );
}

#[test]
fn reset_to_default_restores_the_recommended_set() {
    let mut rt = open_review_settings();
    rt.update(|ui, _| ui.settings.review_events = CategorySet::NONE);
    settle(&mut rt);
    tap(&mut rt, "review-events-reset");
    assert_eq!(rt.state.settings.review_events, CategorySet::recommended());
}

#[test]
fn the_reset_button_waits_until_something_changed() {
    let mut rt = open_review_settings();
    rt.update(|ui, _| ui.settings.review_events = CategorySet::recommended());
    settle(&mut rt);
    let before = rt.state.settings.review_events;
    tap(&mut rt, "review-events-reset");
    assert_eq!(rt.state.settings.review_events, before);
}

#[test]
fn searching_finds_the_review_rows() {
    let mut rt = open_review_settings();
    for word in [
        "baron",
        "teammates",
        "towers",
        "multikills",
        "filter",
        "review",
    ] {
        type_search(&mut rt, word);
        assert!(rt.find("review-events-row").is_some(), "{word}");
    }
    type_search(&mut rt, "reset default");
    assert!(rt.find("review-events-row").is_some());
    type_search(&mut rt, "baron");
    assert!(rt.find("fps-row").is_none());
}
