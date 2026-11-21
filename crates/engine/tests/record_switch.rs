#![allow(clippy::expect_used)]

mod common;

use std::sync::Arc;

use crossbeam_channel::{Sender, unbounded};
use greyscreen_core::game::{GameResult, TimedEvent};
use greyscreen_core::library::{self, RecordingMeta};
use greyscreen_core::settings::Settings;
use greyscreen_core::time::mono_now;
use greyscreen_engine::{
    EngineCmd, EngineStatus, FakeMediaSource, LeagueFactory, NoticeLevel, UiMsg,
};
use greyscreen_league::LeagueMsg;

use common::{Harness, WAIT, assert_playable_from_key_frame, info, kill, start_engine, temp_dir};

struct Match {
    harness: Harness,
    league: Sender<LeagueMsg>,
}

fn start(tag: &str, auto_record: bool) -> Match {
    let settings = Settings {
        output_dir: temp_dir(tag),
        auto_record,
        ..Settings::default()
    };
    let (out_tx, out_rx) = unbounded();
    let league: LeagueFactory = Box::new(move |out: Sender<LeagueMsg>| {
        out_tx.send(out).ok();
        Box::new(())
    });
    let harness = start_engine(settings, Arc::new(FakeMediaSource::new()), league, None);
    let league = out_rx.recv_timeout(WAIT).expect("league sender");
    Match { harness, league }
}

impl Match {
    fn send(&self, msg: LeagueMsg) {
        self.league.send(msg).expect("engine listening");
    }

    fn load_game(&self) {
        self.send(LeagueMsg::WindowFound { hwnd: 7, pid: 42 });
        self.send(LeagueMsg::Ready(info()));
    }

    fn event(&self, id: u32, game_secs: f64) {
        self.send(LeagueMsg::Event(TimedEvent {
            id,
            game_secs,
            at: mono_now(),
            kind: kill("Hero", "Villain"),
        }));
    }

    fn set_auto_record(&self, auto_record: bool) {
        let settings = Settings {
            auto_record,
            output_dir: self.harness.dir.clone(),
            ..Settings::default()
        };
        self.harness
            .engine
            .as_ref()
            .expect("engine running")
            .cmd
            .send(EngineCmd::ApplySettings(settings))
            .expect("engine listening");
    }

    fn wait_status(&self, want: fn(&EngineStatus) -> bool) {
        let found = common::wait_for(&self.harness.ui, |msg| match msg {
            UiMsg::Status(status) => want(status).then_some(()),
            _ => None,
        });
        assert!(found.is_some(), "status not reached");
    }

    fn wait_saved(&self) -> (RecordingMeta, Vec<EngineStatus>) {
        let mut statuses = Vec::new();
        let folder = common::wait_for(&self.harness.ui, |msg| match msg {
            UiMsg::Notice { level, text } if *level == NoticeLevel::Error => {
                panic!("engine error: {text}")
            }
            UiMsg::Status(status) => {
                statuses.push(status.clone());
                None
            }
            UiMsg::RecordingSaved(entry) => Some(entry.folder.clone()),
            _ => None,
        })
        .expect("recording saved");
        (library::read_meta(&folder).expect("sidecar"), statuses)
    }
}

fn recording(status: &EngineStatus) -> bool {
    matches!(status, EngineStatus::Recording { .. })
}

#[test]
fn turning_the_switch_off_mid_game_saves_the_recording_like_a_game_end() {
    let m = start("switch-off", true);
    m.load_game();
    m.wait_status(recording);
    m.event(1, 61.0);
    m.event(2, 75.0);
    std::thread::sleep(std::time::Duration::from_millis(600));

    m.set_auto_record(false);
    let (meta, statuses) = m.wait_saved();
    assert_eq!(statuses.last(), Some(&EngineStatus::GameRunning));
    assert_eq!(meta.game.result, GameResult::Unknown);
    assert_eq!(meta.events.len(), 2);
    assert_eq!(meta.game.info.me.champion, "Ahri");
    assert!(!meta.video.is_empty());
    let folder = m.harness.dir.join(&meta.id);
    assert_playable_from_key_frame(&folder.join(&meta.video));

    m.set_auto_record(true);
    m.wait_status(recording);
    m.send(LeagueMsg::Ended(GameResult::Win));
    let (second, _) = m.wait_saved();
    assert_eq!(second.game.result, GameResult::Win);
    assert_ne!(second.id, meta.id);
}

#[test]
fn turning_the_switch_on_mid_game_starts_recording_right_away() {
    let m = start("switch-on", false);
    m.load_game();
    m.wait_status(|s| matches!(s, EngineStatus::GameRunning));

    m.set_auto_record(true);
    m.wait_status(recording);
    std::thread::sleep(std::time::Duration::from_millis(400));
    m.event(1, 30.0);
    m.send(LeagueMsg::Ended(GameResult::Loss));
    let (meta, _) = m.wait_saved();
    assert_eq!(meta.game.result, GameResult::Loss);
    assert_eq!(meta.game.info.me.champion, "Ahri");
}
