use greyscreen_core::game::{GameEvent, GameResult, Player, Team};
use greyscreen_core::library::EventEntry;
use greyscreen_engine::RecordingEntry;
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
    let all: Vec<&RecordingEntry> = recordings.iter().collect();
    assert_eq!(
        summary(&all, 3),
        "3 games · 1W 1L · 1h 30m recorded · 3.0 GB"
    );
    assert_eq!(
        summary(&all[..1], 1),
        "1 game · 1W 0L · 30m recorded · 1.0 GB"
    );
}

#[test]
fn summary_of_a_filtered_set_says_how_many_of_all() {
    let recordings = Ui::preview().recordings;
    let shown: Vec<&RecordingEntry> = recordings.iter().take(3).collect();
    let line = summary(&shown, recordings.len());
    assert!(line.starts_with("3 of 8 games · "), "{line}");
}

#[test]
fn the_week_counts_only_the_last_seven_days() {
    let recordings = Ui::preview().recordings;
    let stats = week_stats(&recordings, &Zoned::now());
    assert_eq!(stats.games, 6);
    assert_eq!((stats.wins, stats.losses), (4, 2));
    assert_eq!(stats.win_rate(), Some(67));
    let secs: f64 = recordings[..6].iter().map(|r| r.meta.duration_secs).sum();
    assert!((stats.secs - secs).abs() < 1e-6);
}

#[test]
fn an_empty_week_has_no_win_rate() {
    let stats = week_stats(&[], &Zoned::now());
    assert_eq!(stats, WeekStats::default());
    assert_eq!(stats.win_rate(), None);
}

#[test]
fn unknown_results_stay_out_of_the_win_rate() {
    let stats = WeekStats {
        games: 3,
        wins: 1,
        losses: 0,
        secs: 0.0,
    };
    assert_eq!(stats.win_rate(), Some(100));
}

#[test]
fn day_headings_name_recent_days_then_dates() {
    let now: Zoned = "2026-11-21T12:00:00[UTC]".parse().expect("now");
    let at = |stamp: &str| -> Zoned { stamp.parse().expect("zoned") };
    assert_eq!(day_heading(&at("2026-11-21T01:00:00[UTC]"), &now), "Today");
    assert_eq!(
        day_heading(&at("2026-11-20T23:00:00[UTC]"), &now),
        "Yesterday"
    );
    assert_eq!(
        day_heading(&at("2026-11-19T10:00:00[UTC]"), &now),
        "Thursday"
    );
    assert_eq!(
        day_heading(&at("2026-11-02T10:00:00[UTC]"), &now),
        "November 2"
    );
    assert_eq!(
        day_heading(&at("2025-03-02T10:00:00[UTC]"), &now),
        "March 2, 2025"
    );
}

#[test]
fn clip_titles_use_the_display_title_or_split_the_label() {
    assert_eq!(clip_title("MyKill"), "Kill");
    assert_eq!(clip_title("Multikill2"), "Double kill");
    assert_eq!(clip_title("SomethingNew"), "Something new");
}

#[test]
fn recent_clips_come_from_the_newest_game_first() {
    let recordings = Ui::preview().recordings;
    let clips = recent_clips(&recordings, 6);
    assert_eq!(clips.len(), 6);
    assert!(
        clips
            .iter()
            .all(|c| c.entry.meta.id == recordings[0].meta.id)
    );
    let starts: Vec<f64> = clips.iter().map(|c| c.clip.start_secs).collect();
    assert!(starts.windows(2).all(|pair| pair[0] >= pair[1]));
}

#[test]
fn game_time_is_recording_time_minus_the_loading_screen() {
    let recordings = Ui::preview().recordings;
    assert_eq!(game_secs_at(&recordings[0], 98.0), 90.0);
    assert_eq!(game_secs_at(&recordings[0], 2.0), 0.0);
}
