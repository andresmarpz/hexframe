use jiff::ToSpan;

use super::*;

fn day(n: i64) -> Timestamp {
    "2026-11-01T12:00:00Z".parse::<Timestamp>().unwrap() + (n * 24).hours()
}

fn game(id: &str, day_n: i64, video: u64, other: u64) -> GameUsage {
    GameUsage {
        id: id.into(),
        started_at: day(day_n),
        video_bytes: video,
        other_bytes: other,
    }
}

fn video(id: &str) -> Deletion {
    Deletion::Video { id: id.into() }
}

fn whole(id: &str) -> Deletion {
    Deletion::Game { id: id.into() }
}

#[test]
fn nothing_is_planned_under_the_limit() {
    let games = [game("a", 1, 800, 100), game("b", 2, 800, 100)];
    assert_eq!(plan(&games, 1800, None), vec![]);
    assert_eq!(plan(&games, 5000, None), vec![]);
}

#[test]
fn nothing_is_planned_without_games() {
    assert_eq!(plan(&[], 0, None), vec![]);
}

#[test]
fn the_oldest_video_goes_first() {
    let games = [
        game("new", 3, 800, 100),
        game("old", 1, 800, 100),
        game("mid", 2, 800, 100),
    ];
    assert_eq!(plan(&games, 2400, None), vec![video("old")]);
}

#[test]
fn videos_go_oldest_first_until_the_total_fits() {
    let games = [
        game("a", 1, 800, 100),
        game("b", 2, 800, 100),
        game("c", 3, 800, 100),
    ];
    assert_eq!(plan(&games, 1100, None), vec![video("a"), video("b")]);
}

#[test]
fn a_total_exactly_at_the_limit_is_kept() {
    let games = [game("a", 1, 800, 100), game("b", 2, 800, 100)];
    assert_eq!(plan(&games, 1800, None), vec![]);
    assert_eq!(plan(&games, 1799, None), vec![video("a")]);
}

#[test]
fn clips_are_kept_while_videos_can_pay() {
    let games = [game("a", 1, 500, 400), game("b", 2, 500, 400)];
    assert_eq!(plan(&games, 1300, None), vec![video("a")]);
}

#[test]
fn whole_games_go_only_after_every_video_is_gone() {
    let games = [
        game("a", 1, 100, 500),
        game("b", 2, 100, 500),
        game("c", 3, 100, 500),
    ];
    let planned = plan(&games, 1000, None);
    assert_eq!(planned, vec![video("b"), video("c"), whole("a")]);
}

#[test]
fn a_whole_game_replaces_its_video_deletion() {
    let games = [game("a", 1, 100, 500), game("b", 2, 100, 500)];
    let planned = plan(&games, 0, None);
    assert_eq!(planned, vec![whole("a"), whole("b")]);
}

#[test]
fn whole_games_stop_when_the_total_fits() {
    let games = [
        game("a", 1, 100, 500),
        game("b", 2, 100, 500),
        game("c", 3, 100, 500),
    ];
    assert_eq!(
        plan(&games, 1100, None),
        vec![video("b"), video("c"), whole("a")]
    );
    assert_eq!(
        plan(&games, 600, None),
        vec![video("c"), whole("a"), whole("b")]
    );
}

#[test]
fn games_without_a_video_are_skipped_in_the_first_pass() {
    let games = [game("clips", 1, 0, 400), game("full", 2, 600, 100)];
    assert_eq!(plan(&games, 500, None), vec![video("full")]);
}

#[test]
fn a_clips_only_game_can_go_whole_when_nothing_else_helps() {
    let games = [game("clips", 1, 0, 400), game("full", 2, 600, 100)];
    assert_eq!(plan(&games, 200, None), vec![video("full"), whole("clips")]);
}

#[test]
fn the_protected_game_is_never_planned_but_still_counts() {
    let games = [game("old", 1, 500, 100), game("live", 2, 800, 100)];
    assert_eq!(plan(&games, 1000, Some("live")), vec![video("old")]);
    assert_eq!(plan(&games, 0, Some("live")), vec![whole("old")]);
}

#[test]
fn the_protected_game_can_leave_the_total_over_the_limit() {
    let games = [game("live", 1, 5000, 100)];
    assert_eq!(plan(&games, 10, Some("live")), vec![]);
}

#[test]
fn equal_dates_break_ties_by_id() {
    let games = [
        game("b", 1, 500, 0),
        game("a", 1, 500, 0),
        game("c", 1, 500, 0),
    ];
    assert_eq!(plan(&games, 1000, None), vec![video("a")]);
}

#[test]
fn a_zero_limit_empties_everything_but_the_protected_game() {
    let games = [
        game("a", 1, 10, 5),
        game("b", 2, 10, 5),
        game("live", 3, 10, 5),
    ];
    let planned = plan(&games, 0, Some("live"));
    assert_eq!(planned, vec![whole("a"), whole("b")]);
}

fn remaining(games: &[GameUsage], planned: &[Deletion]) -> u64 {
    games
        .iter()
        .map(
            |game| match planned.iter().find(|deletion| deletion.id() == game.id) {
                Some(Deletion::Game { .. }) => 0,
                Some(Deletion::Video { .. }) => game.other_bytes,
                None => game.total_bytes(),
            },
        )
        .sum()
}

#[test]
fn the_plan_fits_the_limit_and_the_last_step_was_needed() {
    let games = [
        game("a", 1, 400, 50),
        game("b", 2, 400, 50),
        game("c", 3, 400, 50),
    ];
    for limit in [0, 100, 150, 450, 500, 900, 950, 1000, 1349, 1350] {
        let planned = plan(&games, limit, None);
        assert!(remaining(&games, &planned) <= limit, "limit {limit}");
        if let Some((_, earlier)) = planned.split_last() {
            assert!(
                remaining(&games, earlier) > limit,
                "limit {limit}: deleted too much"
            );
        }
    }
}

#[test]
fn freeing_space_takes_the_oldest_videos_until_enough() {
    let games = [
        game("c", 3, 500, 10),
        game("a", 1, 300, 10),
        game("b", 2, 400, 10),
    ];
    assert_eq!(plan_free_space(&games, 250, None), vec![video("a")]);
    assert_eq!(
        plan_free_space(&games, 301, None),
        vec![video("a"), video("b")]
    );
    assert_eq!(plan_free_space(&games, 1200, None).len(), 3);
}

#[test]
fn freeing_space_never_deletes_whole_games_or_the_protected_one() {
    let games = [game("a", 1, 300, 50), game("b", 2, 400, 50)];
    assert_eq!(
        plan_free_space(&games, u64::MAX, Some("a")),
        vec![video("b")]
    );
}

#[test]
fn freeing_space_skips_games_without_a_video_and_needs_nothing_for_zero() {
    let games = [game("a", 1, 0, 900), game("b", 2, 400, 0)];
    assert_eq!(plan_free_space(&games, 100, None), vec![video("b")]);
    assert_eq!(plan_free_space(&games, 0, None), vec![]);
    assert_eq!(plan_free_space(&[], 100, None), vec![]);
}
