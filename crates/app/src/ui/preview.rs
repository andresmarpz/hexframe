mod death;
mod thumbs;

use std::path::PathBuf;

use greyscreen_core::game::{GameEvent, GameInfo, GameResult, Player, Team};
use greyscreen_core::highlight::{Highlight, Objective, Structure};
use greyscreen_core::library::thumb::clip_thumb_file;
use greyscreen_core::library::{
    ClipEntry, EventEntry, GameRecord, META_VERSION, RecordingMeta, VideoInfo,
};
use greyscreen_core::settings::Settings;
use greyscreen_engine::{EngineStatus, FeedItem, RecordingEntry};
use jiff::{SignedDuration, Unit};

use super::{Screen, Ui};

pub(super) use death::death_entry;
pub(super) use thumbs::copy_playable;

pub(super) fn ui() -> Ui {
    let mut ui = Ui::blank(Settings::default());
    ui.display_sizes = Vec::new;
    ui.status = EngineStatus::Recording {
        since: 0,
        game: Some(game_info("Ahri", "CLASSIC")),
    };
    ui.elapsed_secs = 12 * 60 + 34;
    ui.feed = feed().into();
    ui.recordings = recordings();
    ui.live_selected = true;
    ui.screen = Screen::Games;
    if let Some(dir) = crate::qa::library() {
        ui.recordings = greyscreen_engine::library_entries(dir);
        ui.settings.output_dir = dir.to_owned();
    } else if rune::is_headless() {
        seed_thumbs(&mut ui.recordings);
    }
    ui
}

fn seed_thumbs(recordings: &mut [RecordingEntry]) {
    let root = thumbs::dir();
    for (game, rec) in recordings.iter_mut().enumerate() {
        rec.folder = root.join(&rec.meta.id);
        let seed = game as u32 * 31 + 5;
        thumbs::write(&rec.folder.join("thumb.jpg"), seed);
        for (index, clip) in rec.meta.clips.iter().enumerate() {
            let dest = rec.folder.join(clip_thumb_file(clip));
            thumbs::write(&dest, seed * 17 + index as u32 * 7 + 3);
        }
    }
    if let Some(first) = recordings.first() {
        thumbs::copy_playable(&first.folder, first);
    }
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
            "Your team lost the top inhibitor",
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

const CLIP_LABELS: [&str; 6] = [
    "MyKill",
    "Multikill2",
    "Baron",
    "FirstBlood",
    "Turret",
    "Multikill3",
];

struct Row {
    hours_ago: i64,
    champion: &'static str,
    mode: &'static str,
    result: GameResult,
    minutes: f64,
    size_mb: u64,
    clips: usize,
}

fn entry(row: Row) -> RecordingEntry {
    let now = crate::qa::now()
        .round(Unit::Hour)
        .unwrap_or_else(|_| crate::qa::now());
    let started_at = now
        .checked_sub(SignedDuration::from_hours(row.hours_ago))
        .unwrap_or(now);
    let id = format!(
        "{}_{}_{}",
        started_at.strftime("%Y-%m-%d_%H-%M-%S"),
        row.champion,
        row.mode
    );
    let (kills, deaths, assists) = (
        row.clips + 1,
        (row.size_mb / 500) as usize,
        row.minutes as usize / 3,
    );
    let me = || String::from("Summoner");
    let fights = (0..kills)
        .map(|_| (me(), "Zed".to_owned(), Vec::new()))
        .chain((0..deaths).map(|_| ("Lee Sin".to_owned(), me(), Vec::new())))
        .chain((0..assists).map(|_| ("Jinx".to_owned(), "Lux".to_owned(), vec![me()])));
    let events = fights
        .zip(1u32..)
        .map(|((killer, victim, assisters), id)| EventEntry {
            id,
            game_secs: f64::from(id) * 60.0,
            rec_secs: f64::from(id) * 60.0 + 8.0,
            kind: GameEvent::ChampionKill {
                killer,
                victim,
                assisters,
            },
            highlights: Vec::new(),
        })
        .collect();
    let clips = (0..row.clips)
        .map(|i| ClipEntry {
            file: format!("clips/{i:02}-00_MyKill.mp4"),
            start_secs: 90.0 + i as f64 * 60.0,
            end_secs: 120.0 + i as f64 * 60.0,
            label: CLIP_LABELS[(i + row.clips) % CLIP_LABELS.len()].into(),
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
            review: Default::default(),
            health: None,
        },
    }
}

fn recordings() -> Vec<RecordingEntry> {
    use GameResult::*;
    let row = |hours_ago, champion, mode, result, minutes, size_mb, clips| {
        entry(Row {
            hours_ago,
            champion,
            mode,
            result,
            minutes,
            size_mb,
            clips,
        })
    };
    let mut list = vec![
        row(2, "Ahri", "CLASSIC", Win, 33.2, 1_820, 6),
        row(4, "Ahri", "CLASSIC", Loss, 28.8, 1_540, 3),
        row(27, "Yasuo", "ARAM", Win, 19.1, 1_010, 9),
        row(29, "Lux", "CLASSIC", Win, 36.7, 1_960, 5),
        row(52, "Jinx", "CLASSIC", Loss, 41.1, 2_240, 0),
        row(76, "Ahri", "ARAM", Win, 17.9, 940, 4),
        row(190, "Viego", "CLASSIC", Unknown, 22.3, 1_190, 2),
        row(215, "Thresh", "CLASSIC", Loss, 31.5, 1_680, 1),
    ];
    for (at, reviewed) in [(0, 2), (3, 1)] {
        let info = list[at].meta.game.info.clone();
        let ids: Vec<u32> = greyscreen_core::review::deaths(&list[at].meta.events, &info)
            .iter()
            .take(reviewed)
            .map(|d| d.event_id)
            .collect();
        for id in ids {
            list[at]
                .meta
                .review
                .toggle_tag(id, greyscreen_core::review::DeathTag::NoVision);
        }
    }
    list
}

pub(super) fn demo_recordings() -> Vec<RecordingEntry> {
    let fixture = review_entry();
    recordings()
        .into_iter()
        .map(|mut rec| {
            let last = rec.meta.events.last().map_or(1.0, |e| e.rec_secs) + 10.0;
            let scale = fixture.meta.duration_secs / last;
            for event in &mut rec.meta.events {
                event.rec_secs *= scale;
                event.highlights = match &event.kind {
                    GameEvent::ChampionKill { killer, .. } if killer == "Summoner" => {
                        vec!["MyKill".to_owned()]
                    }
                    GameEvent::ChampionKill { victim, .. } if victim == "Summoner" => {
                        vec!["MyDeath".to_owned()]
                    }
                    _ => vec!["MyAssist".to_owned()],
                };
            }
            if !rec.meta.clips.is_empty() {
                rec.meta.clips = fixture.meta.clips.clone();
            }
            rec.folder = fixture.folder.clone();
            rec.meta.video = fixture.meta.video.clone();
            rec.meta.duration_secs = fixture.meta.duration_secs;
            rec
        })
        .collect()
}

pub(super) fn review_entry() -> RecordingEntry {
    let mut entry = recordings().remove(0);
    entry.folder = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures");
    entry.meta.video = "review.mp4".into();
    entry.meta.duration_secs = 10.0;
    let who = || String::from("Summoner");
    let mate = |name: &str, champion: &str, team: Team| Player {
        name: name.into(),
        champion: champion.into(),
        team,
        is_bot: false,
    };
    entry.meta.game.info.players.extend([
        mate("Jinx", "Jinx", Team::Order),
        mate("Lux", "Lux", Team::Chaos),
        mate("Lee Sin", "Lee Sin", Team::Chaos),
        mate("Zed", "Zed", Team::Chaos),
    ]);
    let kill = |killer: &str, victim: &str| GameEvent::ChampionKill {
        killer: killer.into(),
        victim: victim.into(),
        assisters: Vec::new(),
    };
    let rows: Vec<(f64, GameEvent, &[&str])> = vec![
        (0.3, GameEvent::MinionsSpawning, &[]),
        (
            1.2,
            GameEvent::FirstBlood { recipient: who() },
            &["FirstBlood"],
        ),
        (2.4, kill("Summoner", "Zed"), &["MyKill"]),
        (
            3.4,
            GameEvent::TurretKilled {
                turret: "Turret_T2_C_05".into(),
                killer: who(),
                assisters: Vec::new(),
            },
            &["Turret"],
        ),
        (4.6, kill("Lee Sin", "Summoner"), &["MyDeath"]),
        (
            5.5,
            GameEvent::DragonKill {
                dragon_type: "Fire".into(),
                stolen: false,
                killer: "Lee Sin".into(),
                assisters: Vec::new(),
            },
            &["EnemyDragon"],
        ),
        (6.4, kill("Jinx", "Lux"), &["AllyKill"]),
        (
            7.3,
            GameEvent::Multikill {
                killer: who(),
                streak: 2,
            },
            &["Multikill2"],
        ),
        (
            8.2,
            GameEvent::BaronKill {
                stolen: false,
                killer: who(),
                assisters: Vec::new(),
            },
            &["Baron"],
        ),
        (
            9.1,
            GameEvent::InhibKilled {
                inhib: "Barracks_T1_C1".into(),
                killer: "Lee Sin".into(),
                assisters: Vec::new(),
            },
            &["EnemyInhibitor"],
        ),
    ];
    entry.meta.events = rows
        .into_iter()
        .zip(1u32..)
        .map(|((rec_secs, kind, highlights), id)| EventEntry {
            id,
            game_secs: rec_secs - 8.0,
            rec_secs,
            kind,
            highlights: highlights.iter().map(|h| (*h).to_owned()).collect(),
        })
        .collect();
    entry.meta.clips = vec![
        ClipEntry {
            file: "review.mp4".into(),
            start_secs: 0.0,
            end_secs: 10.0,
            label: "FirstBlood".into(),
        },
        ClipEntry {
            file: "review.mp4".into(),
            start_secs: 1.0,
            end_secs: 6.0,
            label: "MyKill".into(),
        },
    ];
    entry
}

pub(super) fn update_info() -> greyscreen_core::update::UpdateInfo {
    greyscreen_core::update::UpdateInfo {
        tag: "v0.2.0".into(),
        prerelease: false,
        size: 24_000_000,
        notes_url: greyscreen_core::update::notes_url("v0.2.0"),
    }
}
