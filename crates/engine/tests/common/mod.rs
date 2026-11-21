#![allow(dead_code, clippy::expect_used)]

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crossbeam_channel::{Receiver, Sender, unbounded};
use greyscreen_core::game::{GameEvent, GameInfo, Player, Team};
use greyscreen_core::media::MediaSource;
use greyscreen_core::settings::Settings;
use greyscreen_core::time::Hns;
use greyscreen_engine::{Engine, EngineConfig, EngineHandle, EngineStatus, LeagueFactory, UiMsg};
use greyscreen_league::LeagueMsg;

pub(crate) const WAIT: Duration = Duration::from_secs(20);

pub(crate) struct Harness {
    pub engine: Option<EngineHandle>,
    pub ui: Receiver<UiMsg>,
    pub dir: PathBuf,
    pub settings_path: PathBuf,
}

impl Drop for Harness {
    fn drop(&mut self) {
        self.engine.take();
        std::fs::remove_dir_all(&self.dir).ok();
        std::fs::remove_file(&self.settings_path).ok();
    }
}

pub(crate) fn temp_dir(tag: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    std::env::temp_dir().join(format!("greyscreen-e2-{tag}-{}-{nanos}", std::process::id()))
}

pub(crate) fn player(name: &str, champion: &str, team: Team) -> Player {
    Player {
        name: name.to_owned(),
        champion: champion.to_owned(),
        team,
        is_bot: false,
    }
}

pub(crate) fn info() -> GameInfo {
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

pub(crate) fn kill(killer: &str, victim: &str) -> GameEvent {
    GameEvent::ChampionKill {
        killer: killer.to_owned(),
        victim: victim.to_owned(),
        assisters: Vec::new(),
    }
}

pub(crate) fn no_league() -> LeagueFactory {
    Box::new(|_out: Sender<LeagueMsg>| Box::new(()))
}

pub(crate) type Tap = Box<dyn Fn(&UiMsg) + Send + Sync>;

pub(crate) fn start_engine(
    settings: Settings,
    media: Arc<dyn MediaSource>,
    league: LeagueFactory,
    tap: Option<Tap>,
) -> Harness {
    let dir = settings.output_dir.clone();
    let settings_path = dir.with_extension("settings.json");
    let (tx, rx) = unbounded();
    let sink = Box::new(move |msg: UiMsg| {
        if let Some(tap) = &tap {
            tap(&msg);
        }
        tx.send(msg).ok();
    });
    let engine = Engine::start_with(EngineConfig {
        settings,
        media,
        league,
        sink,
        settings_path: Some(settings_path.clone()),
        updates: None,
        hotkeys: None,
    });
    Harness {
        engine: Some(engine),
        ui: rx,
        dir,
        settings_path,
    }
}

pub(crate) fn start_idle(settings: Settings, media: Arc<dyn MediaSource>) -> Harness {
    start_engine(settings, media, no_league(), None)
}

pub(crate) fn wait_for<T>(
    ui: &Receiver<UiMsg>,
    mut pick: impl FnMut(&UiMsg) -> Option<T>,
) -> Option<T> {
    while let Ok(msg) = ui.recv_timeout(WAIT) {
        if let Some(found) = pick(&msg) {
            return Some(found);
        }
    }
    None
}

pub(crate) fn recording_since(ui: &Receiver<UiMsg>) -> Hns {
    wait_for(ui, |msg| match msg {
        UiMsg::Status(EngineStatus::Recording { since, .. }) => Some(*since),
        _ => None,
    })
    .expect("recording status")
}

pub(crate) fn ffprobe(path: &Path, args: &[&str]) -> Option<String> {
    let Ok(output) = Command::new("ffprobe")
        .args(["-v", "error"])
        .args(args)
        .arg(path)
        .output()
    else {
        eprintln!("ffprobe not found, skipping");
        return None;
    };
    assert!(
        output.status.success(),
        "ffprobe failed on {}: {output:?}",
        path.display()
    );
    Some(String::from_utf8_lossy(&output.stdout).into_owned())
}

pub(crate) fn assert_playable_from_key_frame(path: &Path) {
    if let Some(streams) = ffprobe(path, &["-show_entries", "stream=codec_name"]) {
        assert!(streams.contains("h264"), "no h264 stream: {streams}");
        assert!(streams.contains("aac"), "no aac stream: {streams}");
    }
    let first = ffprobe(
        path,
        &[
            "-select_streams",
            "v",
            "-show_frames",
            "-read_intervals",
            "%+#1",
            "-show_entries",
            "frame=key_frame",
            "-of",
            "csv=p=0",
        ],
    );
    if let Some(first) = first {
        assert_eq!(first.trim(), "1", "first frame is not a key frame: {first}");
    }
}
