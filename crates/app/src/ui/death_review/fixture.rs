use greyscreen_core::game::{GameEvent, Player, Team};
use greyscreen_core::library::EventEntry;
use greyscreen_engine::RecordingEntry;

const DEATH_VIDEO: &str = "deaths.mp4";
const DEATH_VIDEO_SECS: f64 = 60.0;
const GAME_OFFSET_SECS: f64 = 8.0;

fn enemy(champion: &str) -> Player {
    Player {
        name: champion.into(),
        champion: champion.into(),
        team: Team::Chaos,
        is_bot: false,
    }
}

fn kill(killer: &str, victim: &str, assisters: &[&str]) -> GameEvent {
    GameEvent::ChampionKill {
        killer: killer.into(),
        victim: victim.into(),
        assisters: assisters.iter().map(|a| (*a).to_owned()).collect(),
    }
}

pub(crate) fn death_entry() -> RecordingEntry {
    let mut entry = crate::ui::preview::review_entry();
    let meta = &mut entry.meta;
    meta.video = DEATH_VIDEO.into();
    meta.duration_secs = DEATH_VIDEO_SECS;
    meta.clips.clear();
    meta.review = Default::default();
    meta.game
        .info
        .players
        .extend(["Zed", "Lee Sin", "Jinx", "Lux", "Caitlyn"].map(enemy));
    let rows: [(f64, GameEvent, &[&str]); 9] = [
        (9.0, kill("Summoner", "Zed", &[]), &["MyKill"]),
        (14.0, kill("Zed", "Summoner", &["Lee Sin"]), &["MyDeath"]),
        (22.0, kill("Lee Sin", "Summoner", &[]), &["MyDeath"]),
        (
            31.0,
            kill("Jinx", "Summoner", &["Lux", "Caitlyn"]),
            &["MyDeath"],
        ),
        (36.0, kill("Summoner", "Lux", &[]), &["MyKill"]),
        (40.0, kill("Minion_T200L0S3", "Summoner", &[]), &["MyDeath"]),
        (46.0, kill("Zed", "Summoner", &[]), &["MyDeath"]),
        (52.0, kill("Lux", "Summoner", &["Jinx"]), &["MyDeath"]),
        (57.0, kill("Jinx", "Summoner", &[]), &["MyDeath"]),
    ];
    meta.events = rows
        .into_iter()
        .zip(1u32..)
        .map(|((rec_secs, kind, highlights), id)| EventEntry {
            id,
            game_secs: rec_secs - GAME_OFFSET_SECS + 600.0,
            rec_secs,
            kind,
            highlights: highlights.iter().map(|h| (*h).to_owned()).collect(),
        })
        .collect();
    entry
}
