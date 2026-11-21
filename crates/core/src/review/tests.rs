use super::*;
use crate::game::Team;

fn player(name: &str, champion: &str, is_bot: bool) -> Player {
    Player {
        name: name.into(),
        champion: champion.into(),
        team: Team::Order,
        is_bot,
    }
}

fn me() -> Player {
    player("Me#EUW", "Ahri", false)
}

fn kill(id: u32, rec_secs: f64, killer: &str, victim: &str, assisters: &[&str]) -> EventEntry {
    EventEntry {
        id,
        game_secs: rec_secs - 8.0,
        rec_secs,
        kind: GameEvent::ChampionKill {
            killer: killer.into(),
            victim: victim.into(),
            assisters: assisters.iter().map(|a| (*a).to_owned()).collect(),
        },
        highlights: Vec::new(),
    }
}

fn death(id: u32, rec_secs: f64) -> Death {
    Death {
        event_id: id,
        game_secs: rec_secs - 8.0,
        rec_secs,
        killer: "Zed".into(),
        assisters: Vec::new(),
    }
}

#[test]
fn deaths_are_the_champion_kills_with_me_as_victim_in_time_order() {
    let events = vec![
        kill(9, 400.0, "Zed", "Me", &["LeeSin"]),
        kill(2, 100.0, "Me", "Zed", &[]),
        kill(5, 250.0, "Jinx", "Lux", &["Me"]),
        kill(3, 120.0, "Minion_T200L0S3", "Me#EUW", &[]),
        EventEntry {
            id: 4,
            game_secs: 130.0,
            rec_secs: 138.0,
            kind: GameEvent::MinionsSpawning,
            highlights: Vec::new(),
        },
    ];
    let found = deaths(&events, &me());
    let ids: Vec<u32> = found.iter().map(|d| d.event_id).collect();
    assert_eq!(ids, [3, 9]);
    assert_eq!(found[1].killer, "Zed");
    assert_eq!(found[1].assisters, ["LeeSin"]);
    assert_eq!(found[1].game_secs, 392.0);
}

#[test]
fn no_deaths_when_i_never_die() {
    let events = vec![kill(1, 10.0, "Me", "Zed", &[])];
    assert!(deaths(&events, &me()).is_empty());
}

#[test]
fn bots_are_matched_by_champion_name() {
    let bot = player("Bot 1", "Ahri", true);
    let events = vec![kill(1, 10.0, "Zed", "Ahri", &[])];
    assert_eq!(deaths(&events, &bot).len(), 1);
}

#[test]
fn the_window_runs_twelve_seconds_before_to_three_after() {
    let d = death(1, 100.0);
    assert_eq!(d.window(600.0), (88.0, 103.0));
    assert_eq!(d.slow_mo_from(600.0), 99.0);
}

#[test]
fn the_window_is_clamped_to_the_recording() {
    assert_eq!(death(1, 5.0).window(600.0), (0.0, 8.0));
    assert_eq!(death(1, 598.0).window(600.0), (586.0, 600.0));
    assert_eq!(death(1, 5.0).slow_mo_from(600.0), 4.0);
    assert_eq!(death(1, 5.0).window(0.0), (0.0, 8.0));
}

#[test]
fn slow_mo_never_starts_before_the_window() {
    let d = death(1, 1.0);
    let (start, _) = d.window(2.0);
    assert_eq!(d.slow_mo_from(2.0), start);
}

#[test]
fn killers_resolve_to_champions_or_a_plain_description() {
    let players = vec![
        me(),
        player("Zed Main#EUW", "Zed", false),
        player("Bot 2", "LeeSin", true),
    ];
    assert_eq!(killer_label("Zed Main#EUW", &players), "Zed");
    assert_eq!(killer_label("zed main", &players), "Zed");
    assert_eq!(killer_label("LeeSin", &players), "LeeSin");
    assert_eq!(killer_label("Minion_T200L0S3", &players), "a minion");
    assert_eq!(killer_label("Turret_T1_C_05_A", &players), "a turret");
    assert_eq!(killer_label("SRU_Baron", &players), "a jungle monster");
    assert_eq!(killer_label("", &players), "the environment");
    assert_eq!(killer_label("Someone", &players), "Someone");
}

#[test]
fn toggling_a_tag_adds_then_removes_it() {
    let mut data = ReviewData::default();
    assert!(data.toggle_tag(4, DeathTag::NoVision));
    assert!(data.has_tag(4, DeathTag::NoVision));
    assert!(data.is_reviewed(4));
    assert!(!data.toggle_tag(4, DeathTag::NoVision));
    assert!(!data.has_tag(4, DeathTag::NoVision));
    assert!(data.is_empty());
}

#[test]
fn tags_keep_the_canonical_order() {
    let mut data = ReviewData::default();
    data.toggle_tag(1, DeathTag::Unavoidable);
    data.toggle_tag(1, DeathTag::OutOfPosition);
    data.toggle_tag(1, DeathTag::Ganked);
    assert_eq!(
        data.get(1).map(|d| d.tags.clone()),
        Some(vec![
            DeathTag::OutOfPosition,
            DeathTag::Ganked,
            DeathTag::Unavoidable
        ])
    );
}

#[test]
fn a_note_alone_marks_the_death_as_reviewed_and_clearing_it_undoes_that() {
    let mut data = ReviewData::default();
    data.set_note(7, "Walked into fog without a ward");
    assert!(data.is_reviewed(7));
    assert_eq!(data.note(7), "Walked into fog without a ward");
    data.set_note(7, "  ");
    assert!(!data.is_reviewed(7));
    assert_eq!(data.note(7), "");
}

#[test]
fn a_cleared_note_keeps_the_tags() {
    let mut data = ReviewData::default();
    data.toggle_tag(7, DeathTag::BadTrade);
    data.set_note(7, "greedy");
    data.set_note(7, "");
    assert!(data.is_reviewed(7));
    assert!(data.has_tag(7, DeathTag::BadTrade));
}

#[test]
fn entries_are_kept_sorted_by_event() {
    let mut data = ReviewData::default();
    data.toggle_tag(9, DeathTag::Ganked);
    data.toggle_tag(2, DeathTag::Ganked);
    data.toggle_tag(5, DeathTag::Ganked);
    let ids: Vec<u32> = data.deaths.iter().map(|d| d.event_id).collect();
    assert_eq!(ids, [2, 5, 9]);
}

#[test]
fn reviewed_counts_only_current_deaths() {
    let mut data = ReviewData::default();
    data.toggle_tag(1, DeathTag::Ganked);
    data.toggle_tag(99, DeathTag::Ganked);
    let list = [death(1, 50.0), death(2, 90.0)];
    assert_eq!(data.reviewed_among(&list), 1);
}

#[test]
fn the_summary_counts_tags_most_common_first() {
    let mut data = ReviewData::default();
    for id in [1, 2, 3] {
        data.toggle_tag(id, DeathTag::NoVision);
    }
    data.toggle_tag(1, DeathTag::Ganked);
    data.toggle_tag(2, DeathTag::Ganked);
    data.toggle_tag(3, DeathTag::Overextended);
    data.set_note(4, "just unlucky");
    let list = [
        death(1, 50.0),
        death(2, 90.0),
        death(3, 130.0),
        death(4, 170.0),
        death(5, 210.0),
    ];
    let summary = data.summary(&list);
    assert_eq!(summary.deaths, 5);
    assert_eq!(summary.reviewed, 4);
    assert_eq!(summary.notes, 1);
    assert_eq!(
        summary.tags,
        [
            (DeathTag::NoVision, 3),
            (DeathTag::Ganked, 2),
            (DeathTag::Overextended, 1)
        ]
    );
}

#[test]
fn review_data_round_trips_through_json() {
    let mut data = ReviewData::default();
    data.toggle_tag(3, DeathTag::OutOfPosition);
    data.toggle_tag(3, DeathTag::NoVision);
    data.set_note(3, "Backed off too late");
    data.toggle_tag(8, DeathTag::Unavoidable);
    let json = serde_json::to_string(&data).unwrap();
    assert!(json.contains("\"out_of_position\""));
    assert_eq!(serde_json::from_str::<ReviewData>(&json).unwrap(), data);
}

#[test]
fn missing_fields_fall_back_to_defaults() {
    let data: ReviewData = serde_json::from_str(r#"{"deaths":[{"event_id":2}]}"#).unwrap();
    assert!(data.is_reviewed(2));
    assert_eq!(data.note(2), "");
    let empty: ReviewData = serde_json::from_str("{}").unwrap();
    assert!(empty.is_empty());
}
