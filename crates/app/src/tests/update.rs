#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use crossbeam_channel::{Receiver, Sender, unbounded};
use greyscreen_core::settings::Settings;
use greyscreen_core::update::{Asset, Release, UpdateInfo, UpdateState};
use greyscreen_engine::updater::{UpdateConfig, UpdateSource};
use greyscreen_engine::{
    Engine, EngineCmd, EngineConfig, EngineStatus, FakeMediaSource, LeagueFactory, UiMsg,
};
use greyscreen_league::{LeagueMsg, UpdateError};
use rune::{Env, Runtime};

use super::{settle, tap, visible_texts};
use crate::ui::Ui;
use crate::view;

fn info(tag: &str) -> UpdateInfo {
    UpdateInfo::from_release(&release(tag))
}

fn release(tag: &str) -> Release {
    Release {
        tag: tag.into(),
        prerelease: tag.contains('-'),
        published: None,
        exe: Some(Asset {
            url: format!("https://example.test/{tag}"),
            size: 2,
            sha256: None,
        }),
    }
}

fn settings_runtime() -> (Runtime<Ui>, Receiver<EngineCmd>) {
    let (cmd, rx) = unbounded();
    let ui = Ui::live(Settings::default(), cmd);
    let mut rt = Runtime::new(ui, view::root, (960.0, 2600.0), Env::new(true, 1.0));
    rt.frame(0.0);
    tap(&mut rt, "nav-settings");
    (rt, rx)
}

fn show(rt: &mut Runtime<Ui>, state: UpdateState) {
    rt.update(move |ui, _| ui.apply_update(state));
    settle(rt);
}

fn has_text(rt: &Runtime<Ui>, wanted: &str) -> bool {
    visible_texts(rt).iter().any(|t| t == wanted)
}

#[test]
fn the_about_card_offers_a_check() {
    let (rt, _rx) = settings_runtime();
    assert!(has_text(&rt, "Check for updates"));
    assert!(rt.find("check-updates-button").is_some());
    assert!(rt.find("install-update").is_none());
}

#[test]
fn checking_sends_the_command_and_waits() {
    let (mut rt, rx) = settings_runtime();
    tap(&mut rt, "check-updates-button");
    assert_eq!(rx.try_recv(), Ok(EngineCmd::CheckForUpdates));
    assert_eq!(rt.state.update.state, UpdateState::Checking);
    assert!(has_text(&rt, "Checking…"));
    tap(&mut rt, "check-updates-button");
    assert!(rx.try_recv().is_err());
}

#[test]
fn up_to_date_and_failures_read_as_plain_text() {
    let (mut rt, rx) = settings_runtime();
    show(&mut rt, UpdateState::UpToDate);
    assert!(has_text(&rt, "Greyscreen is up to date."));
    show(
        &mut rt,
        UpdateState::Failed("Couldn't check for updates. Try again later.".into()),
    );
    assert!(has_text(
        &rt,
        "Couldn't check for updates. Try again later."
    ));
    tap(&mut rt, "check-updates-button");
    assert_eq!(rx.try_recv(), Ok(EngineCmd::CheckForUpdates));
}

#[test]
fn a_ready_update_offers_install_and_restart() {
    let (mut rt, rx) = settings_runtime();
    show(&mut rt, UpdateState::Ready(info("v0.2.0")));
    assert!(
        visible_texts(&rt)
            .iter()
            .any(|t| t.starts_with("Update to v0.2.0 available"))
    );
    assert!(has_text(&rt, "Install and restart"));
    tap(&mut rt, "install-update");
    assert_eq!(rx.try_recv(), Ok(EngineCmd::InstallUpdate));
}

#[test]
fn downloading_and_restarting_have_no_actions() {
    let (mut rt, rx) = settings_runtime();
    show(&mut rt, UpdateState::Downloading(info("v0.2.0")));
    assert!(has_text(&rt, "Downloading v0.2.0…"));
    tap(&mut rt, "check-updates-button");
    assert!(rx.try_recv().is_err());
    show(&mut rt, UpdateState::Installing(info("v0.2.0")));
    assert!(has_text(&rt, "Restarting to finish the update…"));
    tap(&mut rt, "install-update");
    assert!(rx.try_recv().is_err());
}

#[test]
fn a_loose_copy_links_to_the_release_page() {
    let (mut rt, rx) = settings_runtime();
    show(&mut rt, UpdateState::AvailableManual(info("v0.2.0")));
    assert!(
        visible_texts(&rt)
            .iter()
            .any(|t| t == "Update to v0.2.0 available — download from GitHub")
    );
    assert!(rt.find("install-update").is_none());
    tap(&mut rt, "open-release");
    assert_eq!(
        rt.take_url_requests(),
        ["https://github.com/andresmarpz/greyscreen/releases/tag/v0.2.0"]
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn installing_during_a_game_waits_for_the_end() {
    let (mut rt, rx) = settings_runtime();
    show(&mut rt, UpdateState::Ready(info("v0.2.0")));
    rt.update(|ui, _| ui.status = Ui::preview().status);
    settle(&mut rt);
    tap(&mut rt, "install-update");
    assert!(rx.try_recv().is_err());
    assert!(rt.state.update.install_when_idle);
    assert!(has_text(&rt, "Greyscreen will update after this game"));
    rt.update(|ui, tx| ui.apply(UiMsg::Status(EngineStatus::Finalizing), tx));
    assert!(rx.try_recv().is_err());
    rt.update(|ui, tx| ui.apply(UiMsg::Status(EngineStatus::Idle), tx));
    assert_eq!(rx.try_recv(), Ok(EngineCmd::InstallUpdate));
}

#[test]
fn a_new_update_state_cancels_a_pending_install() {
    let (mut rt, rx) = settings_runtime();
    show(&mut rt, UpdateState::Ready(info("v0.2.0")));
    rt.update(|ui, _| ui.status = Ui::preview().status);
    tap(&mut rt, "install-update");
    show(&mut rt, UpdateState::Checking);
    assert!(!rt.state.update.install_when_idle);
    rt.update(|ui, tx| ui.apply(UiMsg::Status(EngineStatus::Idle), tx));
    assert!(rx.try_recv().is_err());
}

#[test]
fn update_available_follows_the_state() {
    let (mut rt, _rx) = settings_runtime();
    assert!(rt.state.update_available.is_none());
    for state in [
        UpdateState::AvailableManual(info("v0.2.0")),
        UpdateState::Available(info("v0.2.0")),
        UpdateState::Downloading(info("v0.2.0")),
        UpdateState::Ready(info("v0.2.0")),
    ] {
        show(&mut rt, state);
        assert_eq!(rt.state.update_available, Some(info("v0.2.0")));
    }
    show(&mut rt, UpdateState::UpToDate);
    assert!(rt.state.update_available.is_none());
}

#[test]
fn the_automatic_check_switch_saves_the_setting() {
    let (mut rt, rx) = settings_runtime();
    assert!(has_text(&rt, "Check for updates automatically"));
    tap(&mut rt, "check-updates-switch");
    assert!(!rt.state.settings.check_updates);
    let Ok(EngineCmd::ApplySettings(settings)) = rx.try_recv() else {
        panic!("settings were not sent");
    };
    assert!(!settings.check_updates);
}

struct FakeSource {
    releases: Vec<Release>,
}

impl UpdateSource for FakeSource {
    fn releases(&self) -> Result<Vec<Release>, UpdateError> {
        Ok(self.releases.clone())
    }

    fn download(&self, _: &Asset, dest: &Path) -> Result<(), UpdateError> {
        std::fs::create_dir_all(dest.parent().unwrap_or(Path::new("."))).ok();
        std::fs::write(dest, b"MZ").ok();
        Ok(())
    }
}

fn pump_until(rt: &mut Runtime<Ui>, rx: &Receiver<UiMsg>, done: impl Fn(&UpdateState) -> bool) {
    while let Ok(msg) = rx.recv_timeout(Duration::from_secs(5)) {
        rt.update(move |ui, tx| ui.apply(msg, tx));
        settle(rt);
        if done(&rt.state.update.state) {
            return;
        }
    }
    panic!("update state never arrived, at {:?}", rt.state.update.state);
}

#[test]
fn the_card_walks_through_check_download_and_install_with_a_fake_source() {
    let dir = std::env::temp_dir().join(format!("greyscreen-app-update-{}", std::process::id()));
    let launched: Arc<std::sync::Mutex<Vec<PathBuf>>> = Arc::default();
    let launches = Arc::clone(&launched);
    let (ui_tx, ui_rx) = unbounded();
    let league: LeagueFactory = Box::new(|_: Sender<LeagueMsg>| Box::new(()));
    let engine = Engine::start_with(EngineConfig {
        settings: Settings {
            output_dir: dir.join("rec"),
            ..Settings::default()
        },
        media: Arc::new(FakeMediaSource::new()),
        league,
        sink: Box::new(move |msg| {
            ui_tx.send(msg).ok();
        }),
        settings_path: None,
        updates: Some(UpdateConfig {
            source: Arc::new(FakeSource {
                releases: vec![release("v0.2.0")],
            }),
            running: "0.1.6-preview.8".into(),
            can_install: true,
            dir: dir.join("downloads"),
            launch: Box::new(move |path| {
                launches.lock().unwrap().push(path.to_owned());
                Ok(())
            }),
            first_check: Duration::from_secs(3600),
            interval: Duration::from_secs(3600),
            busy_retry: Duration::from_secs(3600),
        }),
    });
    let ui = Ui::live(Settings::default(), engine.cmd.clone());
    let mut rt = Runtime::new(ui, view::root, (960.0, 2600.0), Env::new(true, 1.0));
    rt.frame(0.0);
    tap(&mut rt, "nav-settings");
    tap(&mut rt, "check-updates-button");
    pump_until(&mut rt, &ui_rx, |s| matches!(s, UpdateState::Ready(_)));
    assert!(has_text(&rt, "Install and restart"));
    tap(&mut rt, "install-update");
    pump_until(&mut rt, &ui_rx, |s| matches!(s, UpdateState::Installing(_)));
    assert!(has_text(&rt, "Restarting to finish the update…"));
    let launched = launched.lock().unwrap().clone();
    assert_eq!(launched.len(), 1);
    assert_eq!(launched[0].file_name().unwrap(), "greyscreen-v0.2.0.exe");
    drop(engine);
    std::fs::remove_dir_all(dir).ok();
}
