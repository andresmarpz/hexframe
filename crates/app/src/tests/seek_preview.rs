use crossbeam_channel::unbounded;
use greyscreen_core::settings::Settings;
use greyscreen_engine::{EngineCmd, UiMsg};
use rune::Runtime;

use super::{rect, runtime_sized};
use crate::ui::fake_player;
use crate::ui::{Ui, review_entry};

const TICK: f64 = 1.0 / 60.0;

fn review(entry: greyscreen_engine::RecordingEntry) -> Runtime<Ui> {
    let mut ui = Ui::preview();
    ui.player_factory = fake_player::factory;
    fake_player::set_duration(10.0);
    let mut rt = runtime_sized(ui, (1400.0, 680.0));
    rt.update(|ui, _| ui.open_review_entry(entry));
    for _ in 0..3 {
        rt.frame(TICK);
    }
    rt
}

fn at(rt: &Runtime<Ui>, fraction: f32) -> (f32, f32) {
    let [x, y, w, h] = rect(rt, "timeline");
    (x + w * fraction, y + h / 2.0)
}

fn hover(rt: &mut Runtime<Ui>, fraction: f32) {
    let (x, y) = at(rt, fraction);
    rt.pointer_move(x, y);
    rt.frame(TICK);
    rt.frame(TICK);
}

fn previewed(rt: &Runtime<Ui>) -> Option<f64> {
    rt.state.review.as_ref().and_then(|r| r.seek_preview())
}

fn card(rt: &Runtime<Ui>) -> Option<[f32; 4]> {
    rt.presented("seek-preview").map(|presented| presented.0)
}

#[test]
fn hovering_the_bar_shows_the_tile_and_the_time_above_the_pointer() {
    let mut rt = review(review_entry());
    assert!(card(&rt).is_none());
    hover(&mut rt, 0.5);
    let card = card(&rt).expect("a preview card");
    let tile = rect(&rt, "seek-preview-tile");
    assert_eq!((tile[2], tile[3]), (160.0, 90.0));
    assert!((previewed(&rt).unwrap() - 5.0).abs() < 0.2);
    let [bar_x, bar_y, bar_w, _] = rect(&rt, "timeline");
    assert!(card[1] + card[3] <= bar_y, "the card sits above the bar");
    let center = card[0] + card[2] / 2.0;
    assert!(
        (center - (bar_x + bar_w / 2.0)).abs() < 2.0,
        "the card follows the pointer: {center}"
    );
}

#[test]
fn the_card_follows_the_pointer_and_changes_the_time() {
    let mut rt = review(review_entry());
    hover(&mut rt, 0.2);
    let first = card(&rt).expect("card")[0];
    assert!((previewed(&rt).unwrap() - 2.0).abs() < 0.2);
    hover(&mut rt, 0.8);
    let second = card(&rt).expect("card")[0];
    assert!(second > first + 100.0, "{first} -> {second}");
    assert!((previewed(&rt).unwrap() - 8.0).abs() < 0.2);
}

#[test]
fn leaving_the_bar_hides_the_card() {
    let mut rt = review(review_entry());
    hover(&mut rt, 0.5);
    assert!(card(&rt).is_some());
    rt.pointer_move(5.0, 5.0);
    for _ in 0..60 {
        rt.frame(TICK);
    }
    assert!(card(&rt).is_none());
}

#[test]
fn without_a_storyboard_the_card_shows_only_the_time() {
    let mut rt = review(review_entry());
    rt.update(|ui, _| {
        if let Some(review) = &mut ui.review {
            review.storyboard = None;
        }
    });
    hover(&mut rt, 0.5);
    assert!(card(&rt).is_some(), "the time still shows");
    assert!(rt.find("seek-preview-tile").is_none());
    assert!((previewed(&rt).unwrap() - 5.0).abs() < 0.2);
}

#[test]
fn dragging_the_thumb_moves_the_card_with_no_decoding_and_the_time_follows() {
    let mut rt = review(review_entry());
    let (x, y) = at(&rt, 0.1);
    rt.pointer_down(x, y);
    let mut last = card(&rt).map_or(0.0, |c| c[0]);
    for step in 1..=6 {
        let (x, y) = at(&rt, 0.1 + 0.1 * step as f32);
        rt.pointer_move(x, y);
        rt.frame(TICK);
        let now = card(&rt).expect("card while dragging")[0];
        assert!(now > last, "the card follows the drag at step {step}");
        last = now;
    }
    assert!((previewed(&rt).unwrap() - 7.0).abs() < 0.2);
    assert!(rt.find("seek-preview-tile").is_some());
    let (x, y) = at(&rt, 0.7);
    rt.pointer_up(x, y);
    rt.frame(TICK);
}

#[test]
fn a_marker_shows_its_event_card_and_not_the_preview() {
    let mut rt = review(review_entry());
    let id = rt
        .state
        .review
        .as_ref()
        .and_then(|r| r.shown_events().next().map(|e| e.id))
        .expect("an event");
    let (x, y) = rt.find(&format!("marker-{id}")).expect("marker");
    rt.pointer_move(x, y);
    for _ in 0..30 {
        rt.frame(TICK);
    }
    assert!(rt.presented("event-card").is_some());
    assert!(card(&rt).is_none());
}

#[test]
fn a_game_without_a_storyboard_asks_the_engine_for_one_and_uses_it_when_it_lands() {
    let dir = tempfile::tempdir().unwrap();
    let mut entry = review_entry();
    std::fs::copy(
        entry.folder.join("review.mp4"),
        dir.path().join("review.mp4"),
    )
    .unwrap();
    let source = entry.folder.clone();
    entry.folder = dir.path().to_path_buf();

    let (cmd, rx) = unbounded();
    let mut ui = Ui::live(
        Settings {
            first_run_done: true,
            ..Settings::default()
        },
        cmd,
    );
    ui.player_factory = fake_player::factory;
    fake_player::set_duration(10.0);
    let mut rt = runtime_sized(ui, (1400.0, 680.0));
    rt.update({
        let entry = entry.clone();
        move |ui, _| ui.open_review_entry(entry)
    });
    for _ in 0..3 {
        rt.frame(TICK);
    }
    assert_eq!(
        rx.try_iter()
            .filter(|cmd| matches!(cmd, EngineCmd::BuildStoryboard { .. }))
            .collect::<Vec<_>>(),
        [EngineCmd::BuildStoryboard {
            folder: dir.path().to_path_buf()
        }]
    );
    hover(&mut rt, 0.5);
    assert!(rt.find("seek-preview-tile").is_none());

    for file in ["storyboard.json", "storyboard-0.jpg"] {
        std::fs::copy(source.join(file), dir.path().join(file)).unwrap();
    }
    let folder = dir.path().to_path_buf();
    rt.update(move |ui, tx| ui.apply(UiMsg::Storyboard(folder), tx));
    hover(&mut rt, 0.4);
    hover(&mut rt, 0.5);
    assert!(rt.find("seek-preview-tile").is_some());
}

#[test]
fn a_game_that_has_a_storyboard_does_not_ask_for_one() {
    let (cmd, rx) = unbounded();
    let mut ui = Ui::live(
        Settings {
            first_run_done: true,
            ..Settings::default()
        },
        cmd,
    );
    ui.player_factory = fake_player::factory;
    let mut rt = runtime_sized(ui, (1400.0, 680.0));
    rt.update(|ui, _| ui.open_review_entry(review_entry()));
    rt.frame(TICK);
    assert!(
        rx.try_iter()
            .all(|cmd| !matches!(cmd, EngineCmd::BuildStoryboard { .. }))
    );
}

#[test]
fn the_card_stays_inside_the_window_at_both_ends_of_the_bar() {
    let mut rt = review(review_entry());
    for end in [0.0, 1.0] {
        hover(&mut rt, end);
        let [x, _, w, _] = card(&rt).expect("card");
        assert!(x >= 0.0 && x + w <= 1400.0, "card at {x} wide {w}");
    }
}

#[test]
fn the_card_also_shows_in_theater_mode() {
    let mut rt = review(review_entry());
    rt.update(|ui, _| {
        if let Some(review) = &mut ui.review {
            review.theater = true;
        }
    });
    for _ in 0..3 {
        rt.frame(TICK);
    }
    let (x, y) = at(&rt, 0.3);
    rt.pointer_move(x, y);
    rt.frame(TICK);
    rt.frame(TICK);
    assert!(card(&rt).is_some());
    assert!(rt.find("seek-preview-tile").is_some());
}
