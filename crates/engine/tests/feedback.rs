#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::{Arc, Mutex};
use std::thread::{self, ThreadId};
use std::time::Duration;

use crossbeam_channel::{Sender, unbounded};
use greyscreen_core::feedback::{FeedbackDraft, FeedbackKind, FeedbackReport, FeedbackResult};
use greyscreen_core::settings::Settings;
use greyscreen_engine::{
    Engine, EngineCmd, EngineConfig, FakeMediaSource, FeedbackConfig, FeedbackTransport,
    LeagueFactory, UiMsg,
};
use greyscreen_league::{FeedbackError, LeagueMsg};

type Outcome = Box<dyn Fn() -> Result<(), FeedbackError> + Send + Sync>;

struct FakeTransport {
    sent: Mutex<Vec<(ThreadId, FeedbackReport)>>,
    thread_names: Mutex<Vec<Option<String>>>,
    outcome: Outcome,
}

impl FeedbackTransport for FakeTransport {
    fn send(&self, report: &FeedbackReport) -> Result<(), FeedbackError> {
        self.sent
            .lock()
            .unwrap()
            .push((thread::current().id(), report.clone()));
        self.thread_names
            .lock()
            .unwrap()
            .push(thread::current().name().map(str::to_owned));
        (self.outcome)()
    }
}

fn draft(include_logs: bool) -> FeedbackDraft {
    FeedbackDraft {
        kind: FeedbackKind::Idea,
        message: "Please add a dark mode".into(),
        email: "me@example.com".into(),
        include_logs,
        client_id: "client-1".into(),
    }
}

struct Rig {
    transport: Arc<FakeTransport>,
    cmd: Sender<EngineCmd>,
    ui: crossbeam_channel::Receiver<UiMsg>,
    _engine: greyscreen_engine::EngineHandle,
    logs: tempfile::TempDir,
}

fn rig(outcome: Outcome) -> Rig {
    let logs = tempfile::tempdir().unwrap();
    std::fs::write(
        logs.path().join("greyscreen.log.2026-11-21"),
        "opened /Users/ana/Movies\nsummoner=Faker\n",
    )
    .unwrap();
    let transport = Arc::new(FakeTransport {
        sent: Mutex::default(),
        thread_names: Mutex::default(),
        outcome,
    });
    let (ui_tx, ui) = unbounded();
    let league: LeagueFactory = Box::new(|_: Sender<LeagueMsg>| Box::new(()));
    let engine = Engine::start_with(EngineConfig {
        settings: Settings {
            output_dir: logs.path().join("rec"),
            ..Settings::default()
        },
        media: Arc::new(FakeMediaSource::new()),
        league,
        sink: Box::new(move |msg| {
            ui_tx.send(msg).ok();
        }),
        settings_path: None,
        updates: None,
        feedback: Some(FeedbackConfig {
            transport: transport.clone(),
            app_version: "0.1.9".into(),
            os: Some("macOS 15.4".into()),
            logs_dir: logs.path().to_owned(),
            home_dirs: vec!["/Users/ana".into()],
        }),
        hotkeys: None,
    });
    Rig {
        transport,
        cmd: engine.cmd.clone(),
        ui,
        _engine: engine,
        logs,
    }
}

fn next_feedback(rig: &Rig) -> FeedbackResult {
    while let Ok(msg) = rig.ui.recv_timeout(Duration::from_secs(5)) {
        if let UiMsg::Feedback(result) = msg {
            return result;
        }
    }
    panic!("no feedback result arrived");
}

#[test]
fn the_report_is_sent_from_a_background_thread_and_the_ui_hears_back() {
    let rig = rig(Box::new(|| Ok(())));
    rig.cmd.send(EngineCmd::SendFeedback(draft(true))).unwrap();
    assert_eq!(next_feedback(&rig), FeedbackResult::Sent);
    let sent = rig.transport.sent.lock().unwrap();
    assert_eq!(sent.len(), 1);
    assert_ne!(sent[0].0, thread::current().id());
    let report = &sent[0].1;
    assert_eq!(report.kind, FeedbackKind::Idea);
    assert_eq!(report.email.as_deref(), Some("me@example.com"));
    assert_eq!(report.app_version, "0.1.9");
    assert_eq!(report.os, "macOS 15.4");
    assert_eq!(report.client_id, "client-1");
    let log = report.log_tail.as_deref().unwrap();
    assert!(log.contains("opened ~/Movies"), "{log}");
    assert!(!log.contains("Faker") && !log.contains("ana"), "{log}");
    drop(rig.logs);
}

#[test]
fn the_transport_never_runs_on_the_engine_control_thread() {
    let rig = rig(Box::new(|| Ok(())));
    rig.cmd.send(EngineCmd::SendFeedback(draft(false))).unwrap();
    assert_eq!(next_feedback(&rig), FeedbackResult::Sent);
    let names = rig.transport.thread_names.lock().unwrap();
    assert_eq!(names.as_slice(), [Some("engine-feedback".to_owned())]);
}

#[test]
fn no_logs_leave_the_machine_when_the_box_is_not_ticked() {
    let rig = rig(Box::new(|| Ok(())));
    rig.cmd.send(EngineCmd::SendFeedback(draft(false))).unwrap();
    assert_eq!(next_feedback(&rig), FeedbackResult::Sent);
    assert_eq!(rig.transport.sent.lock().unwrap()[0].1.log_tail, None);
}

#[test]
fn a_failure_returns_a_plain_error_and_the_same_report_as_json() {
    let rig = rig(Box::new(|| {
        Err(FeedbackError::Request {
            url: "https://api.greyscreen.app/v1/feedback".into(),
            reason: "dns error".into(),
        })
    }));
    rig.cmd.send(EngineCmd::SendFeedback(draft(true))).unwrap();
    let FeedbackResult::Failed { error, report } = next_feedback(&rig) else {
        panic!("expected a failure");
    };
    assert!(error.starts_with("Couldn't reach Greyscreen"), "{error}");
    assert!(!error.contains("dns error"));
    let json: serde_json::Value = serde_json::from_str(&report).unwrap();
    let sent = rig.transport.sent.lock().unwrap()[0].1.clone();
    assert_eq!(json["message"], "Please add a dark mode");
    assert_eq!(json["kind"], "idea");
    assert_eq!(json["client_id"], "client-1");
    assert_eq!(json["log_tail"].as_str(), sent.log_tail.as_deref());
}

#[test]
fn server_refusals_and_rate_limits_read_as_plain_text() {
    let rig = rig(Box::new(|| {
        Err(FeedbackError::Rejected("message is required".into()))
    }));
    rig.cmd.send(EngineCmd::SendFeedback(draft(false))).unwrap();
    let FeedbackResult::Failed { error, .. } = next_feedback(&rig) else {
        panic!("expected a failure");
    };
    assert!(error.contains("message is required"), "{error}");

    let rig = self::rig(Box::new(|| Err(FeedbackError::RateLimited)));
    rig.cmd.send(EngineCmd::SendFeedback(draft(false))).unwrap();
    let FeedbackResult::Failed { error, .. } = next_feedback(&rig) else {
        panic!("expected a failure");
    };
    assert!(error.contains("a lot of feedback"), "{error}");
}

#[test]
fn a_build_without_feedback_says_so() {
    let (ui_tx, ui) = unbounded();
    let league: LeagueFactory = Box::new(|_: Sender<LeagueMsg>| Box::new(()));
    let dir = tempfile::tempdir().unwrap();
    let engine = Engine::start_with(EngineConfig {
        settings: Settings {
            output_dir: dir.path().join("rec"),
            ..Settings::default()
        },
        media: Arc::new(FakeMediaSource::new()),
        league,
        sink: Box::new(move |msg| {
            ui_tx.send(msg).ok();
        }),
        settings_path: None,
        updates: None,
        feedback: None,
        hotkeys: None,
    });
    engine
        .cmd
        .send(EngineCmd::SendFeedback(draft(false)))
        .unwrap();
    let found = loop {
        match ui.recv_timeout(Duration::from_secs(5)).unwrap() {
            UiMsg::Feedback(result) => break result,
            _ => continue,
        }
    };
    assert!(matches!(found, FeedbackResult::Failed { .. }));
}
