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
