#![allow(clippy::expect_used)]

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crossbeam_channel::{Receiver, Sender, unbounded};
use greyscreen_core::game::{GameEvent, GameInfo, GameResult, Player, Team, TimedEvent};
use greyscreen_core::library::{self, RecordingMeta};
use greyscreen_core::settings::Settings;
use greyscreen_core::time::{Hns, hns_to_secs, mono_now};
use greyscreen_engine::{
    Engine, EngineHandle, EngineStatus, FakeMediaSource, LeagueFactory, NoticeLevel, UiMsg,
};
use greyscreen_league::LeagueMsg;

const WAIT: Duration = Duration::from_secs(20);

struct Harness {
    engine: Option<EngineHandle>,
    ui: Receiver<UiMsg>,
    dir: PathBuf,
}

impl Drop for Harness {
    fn drop(&mut self) {
        self.engine.take();
        std::fs::remove_dir_all(&self.dir).ok();
    }
}

fn temp_dir(tag: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    std::env::temp_dir().join(format!("greyscreen-e1-{tag}-{}-{nanos}", std::process::id()))
}

fn player(name: &str, champion: &str, team: Team) -> Player {
    Player {
        name: name.to_owned(),
        champion: champion.to_owned(),
        team,
        is_bot: false,
    }
}

fn info() -> GameInfo {
    GameInfo {
        mode: "CLASSIC".to_owned(),
        map: Some(11),
        me: player("Hero", "Ahri", Team::Order),
        players: vec![
            player("Hero", "Ahri", Team::Order),
            player("Villain", "Zed", Team::Chaos),
        ],
    }
}

fn event(id: u32, game_secs: f64, kind: GameEvent) -> LeagueMsg {
    LeagueMsg::Event(TimedEvent {
        id,
        game_secs,
        at: mono_now(),
        kind,
    })
}

fn kill(killer: &str, victim: &str) -> GameEvent {
    GameEvent::ChampionKill {
        killer: killer.to_owned(),
        victim: victim.to_owned(),
        assisters: Vec::new(),
    }
}

fn scripted_league(go: Receiver<()>, finish: Vec<LeagueMsg>) -> LeagueFactory {
    Box::new(move |out: Sender<LeagueMsg>| {
        let (go, finish) = (go.clone(), finish.clone());
        Box::new(thread::spawn(move || run_script(&out, &go, finish)))
    })
}

fn run_script(out: &Sender<LeagueMsg>, go: &Receiver<()>, finish: Vec<LeagueMsg>) {
    let send = |msg: LeagueMsg| out.send(msg).is_ok();
    if !send(LeagueMsg::WindowFound { hwnd: 7, pid: 42 }) || !send(LeagueMsg::Ready(info())) {
        return;
    }
    if go.recv_timeout(WAIT).is_err() {
        return;
    }
    thread::sleep(Duration::from_millis(150));
    send(event(1, 61.0, kill("Hero", "Villain")));
    thread::sleep(Duration::from_millis(150));
    send(event(2, 75.5, kill("Villain", "Hero")));
    thread::sleep(Duration::from_millis(150));
    send(event(
        3,
        90.0,
        GameEvent::DragonKill {
            dragon_type: "Fire".to_owned(),
            stolen: false,
            killer: "Hero".to_owned(),
            assisters: Vec::new(),
        },
    ));
    thread::sleep(Duration::from_millis(300));
    for msg in finish {
        send(msg);
    }
}

fn start(settings: Settings, finish_with: Vec<LeagueMsg>) -> Harness {
    let (go_tx, go_rx) = unbounded();
    let (ui_tx, ui_rx) = unbounded();
    let sink = Box::new(move |msg: UiMsg| {
        if matches!(msg, UiMsg::Status(EngineStatus::Recording { .. })) {
            go_tx.send(()).ok();
        }
        ui_tx.send(msg).ok();
    });
    let dir = settings.output_dir.clone();
    let league = scripted_league(go_rx, finish_with);
    let engine = Engine::start(settings, Arc::new(FakeMediaSource::fast()), league, sink);
    Harness {
        engine: Some(engine),
        ui: ui_rx,
        dir,
    }
}

fn settings(tag: &str) -> Settings {
    Settings {
        output_dir: temp_dir(tag),
        ..Settings::default()
    }
}

fn collect_until_idle(ui: &Receiver<UiMsg>) -> Vec<UiMsg> {
    let mut seen = Vec::new();
    let mut statuses = 0;
    while let Ok(msg) = ui.recv_timeout(WAIT) {
        if matches!(msg, UiMsg::Status(EngineStatus::Idle)) {
            statuses += 1;
        }
        let saved = matches!(msg, UiMsg::RecordingSaved(_));
        seen.push(msg);
        if statuses > 0 && saved {
            break;
        }
    }
    seen
}

fn status_names(msgs: &[UiMsg]) -> Vec<&'static str> {
    msgs.iter()
        .filter_map(|msg| match msg {
            UiMsg::Status(EngineStatus::Idle) => Some("Idle"),
            UiMsg::Status(EngineStatus::Waiting) => Some("Waiting"),
            UiMsg::Status(EngineStatus::Recording { .. }) => Some("Recording"),
            UiMsg::Status(EngineStatus::Finalizing) => Some("Finalizing"),
            UiMsg::Status(EngineStatus::Error(_)) => Some("Error"),
            _ => None,
        })
        .collect()
}

fn recording_folder(msgs: &[UiMsg]) -> PathBuf {
    msgs.iter()
        .find_map(|msg| match msg {
            UiMsg::RecordingSaved(entry) => Some(entry.folder.clone()),
            _ => None,
        })
        .expect("recording saved")
}

fn since(msgs: &[UiMsg]) -> Hns {
    msgs.iter()
        .find_map(|msg| match msg {
            UiMsg::Status(EngineStatus::Recording { since, .. }) => Some(*since),
            _ => None,
        })
        .expect("recording status")
}

fn ffprobe_ok(path: &Path) {
    let Ok(output) = Command::new("ffprobe")
        .args(["-v", "error", "-show_entries", "stream=codec_name"])
        .arg(path)
        .output()
    else {
        eprintln!("ffprobe not found, skipping");
        return;
    };
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "ffprobe failed: {output:?}");
    assert!(text.contains("h264"), "no h264 stream: {text}");
    assert!(text.contains("aac"), "no aac stream: {text}");
}

fn no_errors(msgs: &[UiMsg]) {
    for msg in msgs {
        if let UiMsg::Notice { level, text } = msg {
            assert_ne!(*level, NoticeLevel::Error, "{text}");
        }
    }
}

fn read(folder: &Path) -> RecordingMeta {
    library::read_meta(folder).expect("sidecar")
}

#[test]
fn records_game_with_events_and_finalizes() {
    let harness = start(
        settings("full"),
        vec![LeagueMsg::Ended(GameResult::Win), LeagueMsg::Gone],
    );
    let msgs = collect_until_idle(&harness.ui);
    no_errors(&msgs);
    assert_eq!(
        status_names(&msgs),
        ["Waiting", "Recording", "Finalizing", "Idle"]
    );
    let folder = recording_folder(&msgs);
    assert!(folder.join("game.mp4").is_file());
    assert!(!folder.join("game.mp4.part").exists());
    ffprobe_ok(&folder.join("game.mp4"));

    let feed: Vec<_> = msgs
        .iter()
        .filter_map(|m| match m {
            UiMsg::Feed(item) => Some(item),
            _ => None,
        })
        .collect();
    assert_eq!(feed.len(), 3);
    assert_eq!(feed[0].text, "MyKill");
    assert_eq!(feed[1].text, "MyDeath");
    assert_eq!(feed[2].text, "Dragon");

    let meta = read(&folder);
    assert_eq!(meta.game.result, GameResult::Win);
    assert_eq!(meta.video, "game.mp4");
    assert_eq!(meta.events.len(), 3);
    assert!(meta.duration_secs > 0.5);
    let t0 = since(&msgs);
    for (entry, item) in meta.events.iter().zip(&feed) {
        assert!(entry.rec_secs > 0.0);
        assert!((entry.rec_secs - item.rec_secs).abs() < 1e-9);
        assert!(entry.rec_secs < hns_to_secs(mono_now() - t0));
    }
    let ids: Vec<u32> = meta.events.iter().map(|e| e.id).collect();
    assert_eq!(ids, [1, 2, 3]);
    assert_eq!(meta.events[0].highlights, ["MyKill"]);
}

#[test]
fn rec_secs_match_event_instants_within_ten_ms() {
    let (at_tx, at_rx) = unbounded::<Hns>();
    let (go_tx, go_rx) = unbounded();
    let (ui_tx, ui_rx) = unbounded();
    let sink = Box::new(move |msg: UiMsg| {
        if matches!(msg, UiMsg::Status(EngineStatus::Recording { .. })) {
            go_tx.send(()).ok();
        }
        ui_tx.send(msg).ok();
    });
    let league: LeagueFactory = Box::new(move |out| {
        let (go, at_tx) = (go_rx.clone(), at_tx.clone());
        Box::new(thread::spawn(move || {
            out.send(LeagueMsg::WindowFound { hwnd: 1, pid: 1 }).ok();
            out.send(LeagueMsg::Ready(info())).ok();
            go.recv_timeout(WAIT).ok();
            for id in 1..=3u32 {
                thread::sleep(Duration::from_millis(100));
                let at = mono_now();
                at_tx.send(at).ok();
                out.send(LeagueMsg::Event(TimedEvent {
                    id,
                    game_secs: f64::from(id),
                    at,
                    kind: kill("Hero", "Villain"),
                }))
                .ok();
            }
            out.send(LeagueMsg::Gone).ok();
        }))
    });
    let dir = temp_dir("exact");
    let engine = Engine::start(
        Settings {
            output_dir: dir.clone(),
            ..Settings::default()
        },
        Arc::new(FakeMediaSource::fast()),
        league,
        sink,
    );
    let harness = Harness {
        engine: Some(engine),
        ui: ui_rx,
        dir,
    };
    let msgs = collect_until_idle(&harness.ui);
    let t0 = since(&msgs);
    let ats: Vec<Hns> = at_rx.try_iter().collect();
    let meta = read(&recording_folder(&msgs));
    assert_eq!(meta.game.result, GameResult::Unknown);
    assert_eq!(meta.events.len(), 3);
    for (entry, at) in meta.events.iter().zip(ats) {
        assert!((entry.rec_secs - hns_to_secs(at - t0)).abs() < 0.010);
    }
}

#[test]
fn events_before_recording_start_are_not_stored() {
    let harness = start(
        settings("early"),
        vec![
            LeagueMsg::Event(TimedEvent {
                id: 9,
                game_secs: 1.0,
                at: -10_000_000,
                kind: kill("Hero", "Villain"),
            }),
            LeagueMsg::Gone,
        ],
    );
    let msgs = collect_until_idle(&harness.ui);
    let meta = read(&recording_folder(&msgs));
    assert!(meta.events.iter().all(|e| e.id != 9));
    let fed = msgs
        .iter()
        .any(|m| matches!(m, UiMsg::Feed(item) if item.id == 9 && item.rec_secs < 0.0));
    assert!(fed);
}

#[test]
fn skips_video_when_full_game_recording_is_off() {
    let mut options = settings("novideo");
    options.record_full_game = false;
    let harness = start(options, vec![LeagueMsg::Gone]);
    let msgs = collect_until_idle(&harness.ui);
    no_errors(&msgs);
    assert_eq!(
        status_names(&msgs),
        ["Waiting", "Recording", "Finalizing", "Idle"]
    );
    let folder = recording_folder(&msgs);
    assert!(!folder.join("game.mp4").exists());
    assert!(!folder.join("game.mp4.part").exists());
    let meta = read(&folder);
    assert_eq!(meta.video, "");
    assert_eq!(meta.events.len(), 3);
}

#[test]
fn shutdown_during_recording_finalizes_the_video() {
    let (go_tx, go_rx) = unbounded();
    let (ui_tx, ui_rx) = unbounded();
    let sink = Box::new(move |msg: UiMsg| {
        if matches!(msg, UiMsg::Status(EngineStatus::Recording { .. })) {
            go_tx.send(()).ok();
        }
        ui_tx.send(msg).ok();
    });
    let league: LeagueFactory = Box::new(move |out| {
        let go = go_rx.clone();
        Box::new(thread::spawn(move || {
            out.send(LeagueMsg::WindowFound { hwnd: 1, pid: 1 }).ok();
            out.send(LeagueMsg::Ready(info())).ok();
            go.recv_timeout(WAIT).ok();
            thread::sleep(Duration::from_millis(300));
        }))
    });
    let dir = temp_dir("shutdown");
    let engine = Engine::start(
        Settings {
            output_dir: dir.clone(),
            ..Settings::default()
        },
        Arc::new(FakeMediaSource::fast()),
        league,
        sink,
    );
    let mut recording = false;
    while let Ok(msg) = ui_rx.recv_timeout(WAIT) {
        if matches!(msg, UiMsg::Status(EngineStatus::Recording { .. })) {
            recording = true;
            break;
        }
    }
    assert!(recording);
    thread::sleep(Duration::from_millis(400));
    drop(engine);
    let msgs: Vec<UiMsg> = ui_rx.try_iter().collect();
    let folder = recording_folder(&msgs);
    assert!(folder.join("game.mp4").is_file());
    ffprobe_ok(&folder.join("game.mp4"));
    assert_eq!(status_names(&msgs), ["Finalizing", "Idle"]);
    std::fs::remove_dir_all(dir).ok();
}
