use std::path::PathBuf;

use greyscreen_core::game::{GameEvent, GameInfo, GameResult, Player, Team};
use greyscreen_core::highlight::{Highlight, Objective, Structure};
use greyscreen_core::library::{
    ClipEntry, EventEntry, GameRecord, META_VERSION, RecordingMeta, VideoInfo,
};
use greyscreen_core::settings::Settings;
use greyscreen_engine::{EngineStatus, FeedItem, RecordingEntry};
use jiff::Zoned;
use jiff::civil::date;
use jiff::tz::TimeZone;

use super::Ui;

pub(super) fn ui() -> Ui {
    let mut ui = Ui::blank(Settings::default());
    ui.status = EngineStatus::Recording {
        since: 0,
        game: Some(game_info("Ahri", "CLASSIC")),
    };
    ui.elapsed_secs = 12 * 60 + 34;
    ui.feed = feed().into();
    ui.recordings = recordings();
    ui
}

fn player(champion: &str) -> Player {
    Player {
        name: "Summoner".into(),
        champion: champion.into(),
        team: Team::Order,
        is_bot: false,
    }
}

fn game_info(champion: &str, mode: &str) -> GameInfo {
    GameInfo {
        mode: mode.into(),
        map: Some(11),
        me: player(champion),
        players: vec![player(champion)],
    }
}

fn feed() -> Vec<FeedItem> {
    use Highlight::*;
    let rows: [(f64, Vec<Highlight>, &str); 12] = [
        (95.0, vec![MyFirstBlood], "You took First Blood on Zed"),
        (214.0, vec![MyKill], "You killed Zed"),
        (
            251.0,
            vec![TeamStructure(Structure::Turret)],
            "Your team destroyed the outer mid turret",
        ),
        (
            366.0,
            vec![TeamObjective(Objective::Dragon("Infernal".into()))],
            "Your team slew the Infernal Drake",
        ),
        (402.0, vec![MyDeath], "You were slain by Lee Sin"),
        (
            455.0,
            vec![MyAssist],
            "You assisted on Jinx killing Caitlyn",
        ),
        (
            518.0,
            vec![TeamObjective(Objective::Herald)],
            "Your team took the Rift Herald",
        ),
        (587.0, vec![MyKill], "You killed Lux"),
        (
            601.0,
            vec![MyMultikill { streak: 2 }],
            "Double Kill: Lux and Leona",
        ),
        (
            655.0,
            vec![EnemyStructure(Structure::Inhibitor)],
            "The enemy team lost the top inhibitor",
        ),
        (
            702.0,
            vec![TeamObjective(Objective::Baron)],
            "Your team slew Baron Nashor",
        ),
        (754.0, Vec::new(), "Minions are spawning"),
    ];
    rows.into_iter()
        .zip(1u32..)
        .map(|((secs, highlights, text), id)| FeedItem {
            id,
            rec_secs: secs + 8.0,
            game_secs: secs,
            text: text.into(),
            highlights,
        })
        .collect()
}

struct Row {
    when: (i8, i8, i8),
    champion: &'static str,
    mode: &'static str,
    result: GameResult,
    minutes: f64,
    size_mb: u64,
    clips: usize,
}

fn entry(row: Row) -> RecordingEntry {
    let (day, hour, minute) = row.when;
    let started_at = date(2026, 11, day)
        .at(hour, minute, 0, 0)
        .to_zoned(TimeZone::UTC)
        .unwrap_or_else(|_| Zoned::now());
    let id = format!(
        "2026-11-{day:02}_{hour:02}-{minute:02}-00_{}_{}",
        row.champion, row.mode
    );
    let events = vec![EventEntry {
        id: 1,
        game_secs: 95.0,
        rec_secs: 103.0,
        kind: GameEvent::GameStart,
        highlights: vec!["MyKill".into()],
    }];
    let clips = (0..row.clips)
        .map(|i| ClipEntry {
            file: format!("clips/{i:02}-00_MyKill.mp4"),
            start_secs: 90.0 + i as f64 * 60.0,
            end_secs: 120.0 + i as f64 * 60.0,
            label: "MyKill".into(),
        })
        .collect();
    RecordingEntry {
        folder: PathBuf::from("Greyscreen").join(&id),
        size_bytes: row.size_mb * 1_000_000,
        meta: RecordingMeta {
            version: META_VERSION,
            id,
            started_at,
            duration_secs: row.minutes * 60.0,
            video: "game.mp4".into(),
            game: GameRecord {
                info: game_info(row.champion, row.mode),
                result: row.result,
            },
            events,
            clips,
            encoder: "preview".into(),
            video_format: VideoInfo {
                width: 1920,
                height: 1080,
                fps: 60,
            },
        },
    }
}

fn recordings() -> Vec<RecordingEntry> {
    use GameResult::*;
    let row = |when, champion, mode, result, minutes, size_mb, clips| {
        entry(Row {
            when,
            champion,
            mode,
            result,
            minutes,
            size_mb,
            clips,
        })
    };
    vec![
        row((20, 19, 55), "Ahri", "CLASSIC", Win, 33.2, 1_820, 6),
        row((20, 18, 41), "Ahri", "CLASSIC", Loss, 28.8, 1_540, 3),
        row((19, 22, 10), "Yasuo", "ARAM", Win, 19.1, 1_010, 9),
        row((19, 20, 32), "Lux", "CLASSIC", Win, 36.7, 1_960, 5),
        row((18, 21, 15), "Jinx", "CLASSIC", Loss, 41.1, 2_240, 0),
        row((17, 23, 2), "Ahri", "ARAM", Win, 17.9, 940, 4),
        row((16, 19, 20), "Viego", "CLASSIC", Unknown, 22.3, 1_190, 2),
        row((15, 20, 48), "Thresh", "CLASSIC", Loss, 31.5, 1_680, 1),
    ]
}
