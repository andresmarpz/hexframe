use greyscreen_core::game::{GameEvent, GameResult, Player, Team};
use greyscreen_core::library::EventEntry;
use jiff::Zoned;

use super::*;
use crate::ui::Ui;

fn at(text: &str) -> Zoned {
    text.parse().unwrap()
}

fn me() -> Player {
    Player {
        name: "Summoner".into(),
        champion: "Ahri".into(),
        team: Team::Order,
        is_bot: false,
    }
}

fn kill(killer: &str, victim: &str, assisters: &[&str]) -> EventEntry {
    EventEntry {
        id: 1,
        game_secs: 1.0,
        rec_secs: 1.0,
        kind: GameEvent::ChampionKill {
            killer: killer.into(),
            victim: victim.into(),
            assisters: assisters.iter().map(|a| (*a).to_owned()).collect(),
        },
        highlights: Vec::new(),
    }
}

#[test]
fn modes_are_named_for_people() {
    assert_eq!(mode_label("CLASSIC"), "Summoner's Rift");
    assert_eq!(mode_label("ARAM"), "ARAM");
    assert_eq!(mode_label("PRACTICETOOL"), "Practice Tool");
    assert_eq!(mode_label("CHERRY"), "Arena");
    assert_eq!(mode_label("URF"), "URF");
    assert_eq!(mode_label("ONEFORALL"), "Oneforall");
    assert_eq!(mode_label("NEXUS_BLITZ"), "Nexus Blitz");
    assert_eq!(mode_label(""), "");
}

#[test]
fn dates_are_relative_to_now() {
    let now = at("2026-11-21T10:00:00+00:00[UTC]");
    let day = |s: &str| when(&at(s), &now).day;
    assert_eq!(
        when(&at("2026-11-21T19:55:00+00:00[UTC]"), &now).time,
        "19:55"
    );
    assert_eq!(day("2026-11-21T00:05:00+00:00[UTC]"), "Today");
    assert_eq!(day("2026-11-20T18:41:00+00:00[UTC]"), "Yesterday");
    assert_eq!(day("2026-11-19T22:10:00+00:00[UTC]"), "Thu");
    assert_eq!(day("2026-11-15T20:48:00+00:00[UTC]"), "Sun");
    assert_eq!(day("2026-11-14T20:48:00+00:00[UTC]"), "Nov 14");
    assert_eq!(day("2026-03-02T20:48:00+00:00[UTC]"), "Mar 2");
    assert_eq!(day("2025-11-15T20:48:00+00:00[UTC]"), "Nov 15, 2025");
    assert_eq!(day("2026-11-22T20:48:00+00:00[UTC]"), "Nov 22");
}

#[test]
fn a_game_late_last_night_in_local_time_is_yesterday() {
    let now = at("2026-11-21T10:00:00-03:00[America/Montevideo]");
    let game = at("2026-11-21T01:30:00+00:00[UTC]");
    let shown = when(&game, &now);
    assert_eq!(shown.day, "Yesterday");
    assert_eq!(shown.time, "22:30");
}

#[test]
fn kda_counts_kills_deaths_and_assists_for_me() {
    let events = vec![
        kill("Summoner", "Zed", &[]),
        kill("Summoner#EUW", "Lux", &["Jinx"]),
        kill("Lee Sin", "Summoner", &["Zed"]),
        kill("Jinx", "Caitlyn", &["summoner"]),
        kill("Jinx", "Lux", &["Leona", "Summoner#EUW"]),
        kill("Jinx", "Lux", &["Leona"]),
    ];
    assert_eq!(
        kda(&events, &me()),
        Kda {
            kills: 2,
            deaths: 1,
            assists: 2
        }
    );
}

#[test]
fn kda_of_a_bot_matches_by_champion_name() {
    let bot = Player {
        name: "Ahri Bot".into(),
        champion: "Ahri".into(),
        team: Team::Order,
        is_bot: true,
    };
    assert_eq!(kda(&[kill("Ahri", "Zed", &[])], &bot).kills, 1);
    assert_eq!(kda(&[kill("Ahri", "Zed", &[])], &me()).kills, 0);
}

#[test]
fn kda_ignores_other_events_and_empty_lists() {
    let other = EventEntry {
        kind: GameEvent::Multikill {
            killer: "Summoner".into(),
            streak: 2,
        },
        ..kill("x", "y", &[])
    };
    assert_eq!(kda(&[other], &me()), Kda::default());
    assert_eq!(kda(&[], &me()), Kda::default());
}

#[test]
fn total_time_uses_hours_and_minutes() {
    assert_eq!(total_time(0.0), "0m");
    assert_eq!(total_time(29.0 * 60.0), "29m");
    assert_eq!(total_time(60.0 * 60.0), "1h 0m");
    assert_eq!(total_time(12.0 * 3600.0 + 31.0 * 60.0), "12h 31m");
}

#[test]
fn summary_counts_games_results_time_and_size() {
    let mut recordings = Ui::preview().recordings;
    recordings.truncate(3);
    recordings[0].meta.game.result = GameResult::Win;
    recordings[1].meta.game.result = GameResult::Loss;
    recordings[2].meta.game.result = GameResult::Unknown;
    for r in &mut recordings {
        r.meta.duration_secs = 1800.0;
        r.size_bytes = 1_000_000_000;
    }
    assert_eq!(
        summary(&recordings),
        "3 games · 1W 1L · 1h 30m recorded · 3.0 GB"
    );
    recordings.truncate(1);
    assert_eq!(
        summary(&recordings),
        "1 game · 1W 0L · 30m recorded · 1.0 GB"
    );
}
