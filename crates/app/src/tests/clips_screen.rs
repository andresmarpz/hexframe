use greyscreen_engine::EngineStatus;
use rune::{Key, KeyEvent, Runtime};

use super::keybinds::platform;
use super::{runtime, settle, tap};
use crate::ui::library::format::recent_clips;
use crate::ui::{Detail, Screen, Source, Ui};

fn on_clips() -> Runtime<Ui> {
    let mut rt = runtime();
    rt.on_key = Some(Ui::on_key);
    rt.update(|ui, _| {
        ui.status = EngineStatus::Idle;
        ui.go(Screen::Clips);
    });
    settle(&mut rt);
    rt
}

fn card_keys(rt: &Runtime<Ui>) -> Vec<String> {
    rt.elements()
        .into_iter()
        .filter_map(|e| e.key)
        .filter(|key| key.starts_with("clip-"))
        .collect()
}

#[test]
fn the_grid_shows_every_saved_clip() {
    let rt = on_clips();
    let total = rt.state.clip_count();
    assert!(
        total > 4,
        "the fixture should have more clips than Home shows"
    );
    assert_eq!(card_keys(&rt).len(), total);
    assert!(rt.find("clips-grid").is_some());
    assert!(rt.find("clips-empty").is_none());
}

#[test]
fn the_nav_row_counts_the_clips_and_opens_the_screen() {
    let mut rt = runtime();
    let count = rt.state.clip_count().to_string();
    let said = rt
        .accessibility_tree()
        .nodes
        .iter()
        .find(|(_, node)| node.label() == Some("Clips") && node.description().is_some())
        .and_then(|(_, node)| node.description().map(str::to_owned));
    assert_eq!(said, Some(count));
    tap(&mut rt, "nav-clips");
    assert_eq!(rt.state.screen, Screen::Clips);
    assert_eq!(rt.state.detail(), Detail::Clips);
}

#[test]
fn without_clips_the_screen_says_so() {
    let mut rt = on_clips();
    rt.update(|ui, _| {
        for rec in &mut ui.recordings {
            rec.meta.clips.clear();
        }
    });
    settle(&mut rt);
    assert!(rt.find("clips-empty").is_some());
    assert!(rt.find("clips-grid").is_none());
}

#[test]
fn clicking_a_clip_opens_its_game_in_games_and_plays_it() {
    let mut rt = on_clips();
    let (id, index) = {
        let first = recent_clips(&rt.state.recordings, 1)[0];
        (first.entry.meta.id.clone(), first.index)
    };
    tap(&mut rt, &format!("clip-{id}-{index}"));
    assert_eq!(rt.state.screen, Screen::Games);
    assert_eq!(rt.state.detail(), Detail::Game);
    assert_eq!(rt.state.selected_id(), Some(id.as_str()));
    assert_eq!(
        rt.state.review.as_ref().map(|review| review.source),
        Some(Source::Clip(index))
    );
}

#[test]
fn the_platform_modifier_with_3_opens_clips() {
    let mut rt = on_clips();
    rt.update(|ui, _| ui.go(Screen::Home));
    rt.key(KeyEvent {
        key: Key::Command('3'),
        mods: platform(),
    });
    settle(&mut rt);
    assert_eq!(rt.state.screen, Screen::Clips);
}

#[test]
fn arrow_keys_do_not_open_games_from_clips() {
    let mut rt = on_clips();
    super::press(&mut rt, Key::Down);
    assert!(rt.state.review.is_none());
}

#[test]
fn searching_clips_filters_by_champion_or_title_and_clears() {
    let mut rt = on_clips();
    let total = rt.state.clip_count();
    assert!(rt.find("clips-search").is_some());

    rt.update(|ui, _| ui.clips_search.replace_text("ahri"));
    settle(&mut rt);
    let ahri_cards = card_keys(&rt).len();
    assert!(ahri_cards > 0 && ahri_cards < total);

    rt.update(|ui, _| ui.clips_search.replace_text("double kill"));
    settle(&mut rt);
    let title_cards = card_keys(&rt).len();
    assert!(title_cards > 0 && title_cards < total);

    rt.update(|ui, _| ui.clips_search.replace_text("zzz-no-such-clip"));
    settle(&mut rt);
    assert!(rt.find("clips-grid").is_none());
    assert!(rt.find("clips-clear-search").is_some());
    assert!(
        super::visible_texts(&rt)
            .iter()
            .any(|t| t == "No clips match")
    );

    tap(&mut rt, "clips-clear-search");
    assert_eq!(card_keys(&rt).len(), total);
}

#[test]
fn clip_card_shows_duration_once_not_twice() {
    let mut rt = on_clips();
    rt.update(|ui, _| {
        ui.recordings.truncate(1);
        ui.recordings[0].meta.clips.truncate(1);
    });
    settle(&mut rt);
    let clip = &rt.state.recordings[0].meta.clips[0];
    let duration = crate::ui::format_clock((clip.end_secs - clip.start_secs).max(0.0) as u32);
    let count = super::visible_texts(&rt)
        .iter()
        .filter(|t| t.as_str() == duration)
        .count();
    assert_eq!(count, 1, "duration {duration} appeared {count} times");
}
