#![allow(clippy::expect_used)]

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crossbeam_channel::{Receiver, Sender, unbounded};
use greyscreen_core::game::{GameEvent, GameInfo, GameResult, Player, Team, TimedEvent};
use greyscreen_core::settings::Settings;
use greyscreen_core::time::mono_now;
use greyscreen_engine::{
    Engine, EngineCmd, EngineConfig, EngineHandle, EngineStatus, FakeMediaSource, LeagueFactory,
    UiMsg,
};
use greyscreen_league::LeagueMsg;
use rune::Runtime;

use super::{runtime_with, tap};
use crate::ui::fake_player;
use crate::ui::{Screen, Ui};

const WAIT: Duration = Duration::from_secs(30);

struct Match {
    rt: Runtime<Ui>,
    league: Sender<LeagueMsg>,
    ui: Receiver<UiMsg>,
    dir: PathBuf,
    engine: Option<EngineHandle>,
}

impl Drop for Match {
    fn drop(&mut self) {
        self.engine.take();
        std::fs::remove_dir_all(&self.dir).ok();
    }
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

fn start(tag: &str, auto_record: bool) -> Match {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let dir =
        std::env::temp_dir().join(format!("greyscreen-app-{tag}-{}-{nanos}", std::process::id()));
    let settings = Settings {
        output_dir: dir.clone(),
        auto_record,
        ..Settings::default()
    };
    let (out_tx, out_rx) = unbounded();
    let league: LeagueFactory = Box::new(move |out: Sender<LeagueMsg>| {
        out_tx.send(out).ok();
        Box::new(())
    });
    let (ui_tx, ui_rx) = unbounded();
    let engine = Engine::start_with(EngineConfig {
        settings: settings.clone(),
        media: Arc::new(FakeMediaSource::new()),
        league,
        sink: Box::new(move |msg| {
            ui_tx.send(msg).ok();
        }),
        settings_path: None,
        updates: None,
    });
    let mut ui = Ui::live(settings, engine.cmd.clone());
    ui.player_factory = fake_player::factory;
    Match {
        rt: runtime_with(ui),
        league: out_rx.recv_timeout(WAIT).expect("league sender"),
        ui: ui_rx,
        dir,
        engine: Some(engine),
    }
}

impl Match {
    fn send(&self, msg: LeagueMsg) {
        self.league.send(msg).expect("engine listening");
    }

    fn load_game(&self) {
        self.send(LeagueMsg::WindowFound { hwnd: 7, pid: 42 });
        self.send(LeagueMsg::Ready(info()));
    }

    fn kill(&self, id: u32) {
        self.send(LeagueMsg::Event(TimedEvent {
            id,
            game_secs: f64::from(id) * 10.0,
            at: mono_now(),
            kind: GameEvent::ChampionKill {
                killer: "Hero".to_owned(),
                victim: "Villain".to_owned(),
                assisters: Vec::new(),
            },
        }));
    }

    fn pump_until(&mut self, mut done: impl FnMut(&Ui) -> bool) {
        let deadline = Instant::now() + WAIT;
        loop {
            while let Ok(msg) = self.ui.try_recv() {
                self.rt.update(|ui, tx| ui.apply(msg, tx));
            }
            self.rt.frame(1.0 / 60.0);
            if done(&self.rt.state) {
                return;
            }
            assert!(Instant::now() < deadline, "timed out");
            std::thread::sleep(Duration::from_millis(20));
        }
    }
}

fn recording(ui: &Ui) -> bool {
    matches!(ui.status, EngineStatus::Recording { .. })
}

#[test]
fn after_a_match_the_feed_is_empty_and_the_last_game_card_opens_in_review() {
    let mut m = start("match", true);
    m.load_game();
    m.pump_until(recording);
    assert!(m.rt.find("feed-card").is_some());
    assert!(m.rt.find("last-game-card").is_none());

    m.kill(1);
    m.kill(2);
    m.pump_until(|ui| ui.feed.len() == 2);
    assert!(m.rt.find("feed-1").is_some());

    m.send(LeagueMsg::Ended(GameResult::Win));
    m.pump_until(|ui| ui.last_game.is_some());

    assert!(matches!(m.rt.state.status, EngineStatus::Idle));
    assert!(m.rt.state.feed.is_empty());
    assert!(m.rt.find("feed-card").is_none());
    assert!(m.rt.find("last-game-card").is_some());
    let last = m.rt.state.last_game.clone().expect("last game");
    assert_eq!(last.meta.game.result, GameResult::Win);
    assert_eq!(last.meta.events.len(), 2);

    tap(&mut m.rt, "open-last-game");
    assert_eq!(m.rt.state.screen, Screen::Review);
    assert_eq!(
        m.rt.state.review.as_ref().map(|r| r.entry.meta.id.clone()),
        Some(last.meta.id)
    );
}

#[test]
fn the_next_game_replaces_the_last_game_card_with_the_live_feed() {
    let mut m = start("next-game", true);
    m.load_game();
    m.pump_until(recording);
    m.send(LeagueMsg::Ended(GameResult::Loss));
    m.pump_until(|ui| ui.last_game.is_some());
    assert!(m.rt.find("last-game-card").is_some());

    m.send(LeagueMsg::Gone);
    m.load_game();
    m.pump_until(recording);
    assert!(m.rt.state.last_game.is_none());
    assert!(m.rt.find("last-game-card").is_none());
    assert!(m.rt.find("feed-card").is_some());
}

#[test]
fn record_games_switch_saves_the_setting_and_stops_a_running_recording() {
    let mut m = start("switch", true);
    m.load_game();
    m.pump_until(recording);
    m.kill(1);
    m.pump_until(|ui| ui.feed.len() == 1);

    tap(&mut m.rt, "record-games");
    assert!(!m.rt.state.settings.auto_record);
    m.pump_until(|ui| ui.last_game.is_some());
    assert!(matches!(m.rt.state.status, EngineStatus::GameRunning));
    assert!(m.rt.state.feed.is_empty());
    assert!(m.rt.find("last-game-card").is_some());

    tap(&mut m.rt, "record-games");
    assert!(m.rt.state.settings.auto_record);
    m.pump_until(recording);
    assert!(m.rt.state.last_game.is_none());
}

#[test]
fn record_games_switch_sends_the_new_settings_to_the_engine() {
    let (mut rt, rx) = super::live_runtime();
    tap(&mut rt, "record-games");
    let Ok(EngineCmd::ApplySettings(sent)) = rx.try_recv() else {
        panic!("no ApplySettings sent");
    };
    assert!(!sent.auto_record);
}
