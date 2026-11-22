#![allow(clippy::expect_used)]

use std::path::Path;
use std::process::Command;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crossbeam_channel::{Sender, unbounded};
use greyscreen_core::game::{GameInfo, Player, Team};
use greyscreen_core::library;
use greyscreen_core::settings::Settings;
use greyscreen_engine::{Engine, EngineStatus, FakeMediaSource, LeagueFactory, UiMsg};
use greyscreen_league::LeagueMsg;

use crate::quit;

const WAIT: Duration = Duration::from_secs(20);

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

fn ffprobe_ok(path: &Path) {
    let Ok(output) = Command::new("ffprobe")
        .args(["-v", "error", "-show_entries", "stream=codec_name"])
        .arg(path)
        .output()
    else {
        eprintln!("ffprobe not found, skipping");
        return;
    };
    assert!(output.status.success(), "ffprobe failed: {output:?}");
}

#[test]
fn quitting_during_a_recording_finalizes_the_video_and_clears_the_check_marker() {
    let dir = tempfile::tempdir().expect("temp dir");
    let (go_tx, go_rx) = unbounded();
    let (ui_tx, ui_rx) = unbounded();
    let sink = Box::new(move |msg: UiMsg| {
        if matches!(msg, UiMsg::Status(EngineStatus::Recording { .. })) {
            go_tx.send(()).ok();
        }
        ui_tx.send(msg).ok();
    });
    let league: LeagueFactory = Box::new(move |out: Sender<LeagueMsg>| {
        let go = go_rx.clone();
        Box::new(thread::spawn(move || {
            out.send(LeagueMsg::WindowFound { hwnd: 1, pid: 1 }).ok();
            out.send(LeagueMsg::Ready(info())).ok();
            go.recv_timeout(WAIT).ok();
            thread::sleep(Duration::from_millis(300));
        }))
    });
    let engine = Engine::start(
        Settings {
            output_dir: dir.path().join("videos"),
            ..Settings::default()
        },
        Arc::new(FakeMediaSource::new().with_speed(20)),
        league,
        sink,
    );
    let recording = std::iter::from_fn(|| ui_rx.recv_timeout(WAIT).ok())
        .any(|msg| matches!(msg, UiMsg::Status(EngineStatus::Recording { .. })));
    assert!(recording);
    thread::sleep(Duration::from_millis(400));
    let cache = dir.path().join("encoder-check.json");
    let marker = cache.with_extension("running");
    std::fs::write(&marker, "{}").expect("marker");

    quit::finish(None, Some(engine), &cache);

    let msgs: Vec<UiMsg> = ui_rx.try_iter().collect();
    let folder = msgs
        .iter()
        .find_map(|msg| match msg {
            UiMsg::RecordingSaved(entry) => Some(entry.folder.clone()),
            _ => None,
        })
        .expect("recording saved");
    assert!(folder.join("game.mp4").is_file());
    ffprobe_ok(&folder.join("game.mp4"));
    assert!(!folder.join("game.mp4.part").exists());
    let meta = library::read_meta(&folder).expect("sidecar");
    assert!(meta.duration_secs > 0.0);
    assert!(meta.health.is_some());
    assert!(!marker.exists());
}
