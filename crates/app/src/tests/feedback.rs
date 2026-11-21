#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::{Arc, Mutex};
use std::time::Duration;

use crossbeam_channel::{Receiver, Sender, unbounded};
use greyscreen_core::feedback::{
    DraftProblem, FeedbackDraft, FeedbackKind, FeedbackReport, FeedbackResult, MAX_MESSAGE_CHARS,
};
use greyscreen_core::keybinds::Action;
use greyscreen_core::settings::{NavStyle, Settings};
use greyscreen_engine::{
    Engine, EngineCmd, EngineConfig, FakeMediaSource, FeedbackConfig, FeedbackTransport,
    LeagueFactory, UiMsg,
};
use greyscreen_league::{FeedbackError, LeagueMsg};
use rune::{Key, KeyEvent, Mods, Runtime, TextEdit};

use super::{live_runtime, settle, tap, visible_texts};
use crate::ui::{FeedbackPhase, Ui};

fn has_text(rt: &Runtime<Ui>, wanted: &str) -> bool {
    visible_texts(rt).iter().any(|t| t == wanted)
}

fn write(rt: &mut Runtime<Ui>, message: &str) {
    let message = message.to_owned();
    rt.update(move |ui, _| ui.feedback.message = TextEdit::new(message));
    settle(rt);
}

fn open(rt: &mut Runtime<Ui>) {
    tap(rt, "nav-feedback");
    settle(rt);
}

fn sent_commands(rx: &Receiver<EngineCmd>) -> Vec<FeedbackDraft> {
    rx.try_iter()
        .filter_map(|cmd| match cmd {
            EngineCmd::SendFeedback(draft) => Some(draft),
            _ => None,
        })
        .collect()
}

#[test]
fn the_nav_row_opens_the_dialog_with_the_logs_box_ticked() {
    let (mut rt, _rx) = live_runtime();
    assert!(rt.find("feedback-dialog").is_none());
    open(&mut rt);
    assert!(rt.find("feedback-dialog").is_some());
    for key in [
        "feedback-kind",
        FEEDBACK_MESSAGE,
        "feedback-email",
        "feedback-logs",
        "feedback-send",
        "feedback-cancel",
    ] {
        assert!(rt.find(key).is_some(), "{key}");
    }
    assert!(rt.state.feedback.include_logs);
    assert_eq!(rt.state.feedback.kind, FeedbackKind::Bug);
    assert!(has_text(&rt, "Include recent logs"));
    assert!(has_text(&rt, "Email (optional)"));
}

const FEEDBACK_MESSAGE: &str = "feedback-message";

#[test]
fn the_rail_and_the_dock_have_a_feedback_button_with_a_tooltip() {
    let (mut rt, _rx) = live_runtime();
    rt.update(|ui, _| ui.settings.sidebar_collapsed = true);
    settle(&mut rt);
    open(&mut rt);
    assert!(rt.state.feedback.open);

    let (mut rt, _rx) = live_runtime();
    rt.update(|ui, _| ui.settings.nav_style = NavStyle::Dock);
    settle(&mut rt);
    tap(&mut rt, "dock-feedback");
    settle(&mut rt);
    assert!(rt.state.feedback.open);
    assert!(rt.find("nav").is_none());
}

#[test]
fn the_about_page_has_a_send_feedback_button_that_replaces_the_settings_sheet() {
    let (mut rt, _rx) = live_runtime();
    tap(&mut rt, "nav-settings");
    tap(&mut rt, "settings-nav-about");
    tap(&mut rt, "send-feedback");
    settle(&mut rt);
    assert!(rt.state.feedback.open);
    assert!(!rt.state.settings_open);
    assert!(rt.find("feedback-dialog").is_some());
}

#[test]
fn the_action_has_no_default_key_and_opens_the_dialog_when_bound() {
    assert!(Action::SendFeedback.spec().defaults.is_empty());
    let (mut rt, _rx) = live_runtime();
    rt.on_key = Some(Ui::on_key);
    rt.update(|ui, _| {
        ui.settings
            .keybinds
            .insert("send_feedback".into(), "Alt+Shift+K".into());
    });
    rt.key(KeyEvent {
        key: Key::Text("k".into()),
        mods: Mods {
            alt: true,
            shift: true,
            ..Mods::default()
        },
    });
    settle(&mut rt);
    assert!(rt.state.feedback.open);
}

#[test]
fn an_empty_message_is_not_sent() {
    let (mut rt, rx) = live_runtime();
    open(&mut rt);
    tap(&mut rt, "feedback-send");
    assert!(sent_commands(&rx).is_empty());
    assert_eq!(rt.state.feedback.problem, Some(DraftProblem::EmptyMessage));
    assert!(has_text(&rt, DraftProblem::EmptyMessage.text()));
    assert_eq!(rt.state.feedback.phase, FeedbackPhase::Editing);
}

#[test]
fn a_bad_email_is_not_sent() {
    let (mut rt, rx) = live_runtime();
    open(&mut rt);
    write(&mut rt, "It crashed");
    rt.update(|ui, _| ui.feedback.email = TextEdit::new("nope"));
    tap(&mut rt, "feedback-send");
    assert!(sent_commands(&rx).is_empty());
    assert_eq!(rt.state.feedback.problem, Some(DraftProblem::BadEmail));
}

#[test]
fn the_message_is_capped_at_5000_characters() {
    let (mut rt, _rx) = live_runtime();
    open(&mut rt);
    rt.update(|ui, _| {
        ui.feedback.message = TextEdit::new("é".repeat(MAX_MESSAGE_CHARS + 40));
        ui.feedback_message_edited();
    });
    assert_eq!(rt.state.feedback.message_chars(), MAX_MESSAGE_CHARS);
}

#[test]
fn sending_stores_a_client_id_once_and_sends_the_picked_options() {
    let (mut rt, rx) = live_runtime();
    open(&mut rt);
    write(&mut rt, "Add a dark mode");
    rt.update(|ui, _| {
        ui.set_feedback_kind(1);
        ui.feedback.email = TextEdit::new(" me@example.com ");
        ui.toggle_feedback_logs();
    });
    tap(&mut rt, "feedback-send");
    let commands: Vec<EngineCmd> = rx.try_iter().collect();
    let saved = commands
        .iter()
        .find_map(|cmd| match cmd {
            EngineCmd::ApplySettings(settings) => settings.client_id.clone(),
            _ => None,
        })
        .expect("the client id is saved through the engine");
    let draft = commands
        .iter()
        .find_map(|cmd| match cmd {
            EngineCmd::SendFeedback(draft) => Some(draft.clone()),
            _ => None,
        })
        .expect("the report is sent");
    assert_eq!(draft.client_id, saved);
    assert_eq!(draft.kind, FeedbackKind::Idea);
    assert_eq!(draft.message, "Add a dark mode");
    assert_eq!(draft.email.trim(), "me@example.com");
    assert!(!draft.include_logs);
    assert_eq!(rt.state.settings.client_id.as_deref(), Some(saved.as_str()));

    rt.update(|ui, _| {
        ui.apply_feedback(FeedbackResult::Failed {
            error: "x".into(),
            report: "{}".into(),
        })
    });
    settle(&mut rt);
    tap(&mut rt, "feedback-send");
    let again = sent_commands(&rx);
    assert_eq!(again.len(), 1);
    assert_eq!(again[0].client_id, saved);
}

#[test]
fn while_sending_the_buttons_are_disabled_and_a_spinner_shows() {
    let (mut rt, rx) = live_runtime();
    open(&mut rt);
    write(&mut rt, "It crashed");
    tap(&mut rt, "feedback-send");
    settle(&mut rt);
    assert_eq!(sent_commands(&rx).len(), 1);
    assert!(rt.state.feedback.sending());
    assert!(rt.find("feedback-spinner").is_some());
    assert!(has_text(&rt, "Sending…"));
    tap(&mut rt, "feedback-send");
    tap(&mut rt, "feedback-cancel");
    assert!(sent_commands(&rx).is_empty());
    assert!(rt.state.feedback.open, "cancel is disabled while sending");
    rt.update(|ui, _| ui.close_feedback());
    assert!(rt.state.feedback.open, "closing waits for the answer");
}

#[test]
fn success_says_thanks_and_the_next_open_starts_clean() {
    let (mut rt, _rx) = live_runtime();
    open(&mut rt);
    write(&mut rt, "It crashed");
    tap(&mut rt, "feedback-send");
    rt.update(|ui, _| ui.apply_feedback(FeedbackResult::Sent));
    settle(&mut rt);
    assert!(has_text(&rt, "Thanks, we got it."));
    assert!(rt.find("feedback-send").is_none());
    tap(&mut rt, "feedback-close");
    settle(&mut rt);
    assert!(rt.find("feedback-dialog").is_none());
    open(&mut rt);
    assert!(rt.state.feedback.message.text.is_empty());
    assert_eq!(rt.state.feedback.phase, FeedbackPhase::Editing);
}

#[test]
fn failure_shows_the_error_keeps_the_text_and_copies_the_report() {
    let (mut rt, _rx) = live_runtime();
    let clipboard = rune::clipboard::memory();
    rt.use_clipboard(&clipboard);
    open(&mut rt);
    write(&mut rt, "It crashed");
    tap(&mut rt, "feedback-send");
    rt.update(|ui, _| {
        ui.apply_feedback(FeedbackResult::Failed {
            error: "Couldn't reach Greyscreen. Check your connection and try again.".into(),
            report: "{\"message\":\"It crashed\"}".into(),
        });
    });
    settle(&mut rt);
    assert!(has_text(
        &rt,
        "Couldn't reach Greyscreen. Check your connection and try again."
    ));
    assert_eq!(rt.state.feedback.message.text, "It crashed");
    assert!(rt.find("feedback-send").is_some());
    tap(&mut rt, "feedback-copy");
    settle(&mut rt);
    assert_eq!(
        clipboard.text().as_deref().map(String::as_str),
        Some("{\"message\":\"It crashed\"}")
    );
    assert!(has_text(&rt, "Copied"));
}

#[test]
fn a_failure_without_a_report_offers_no_copy_button() {
    let (mut rt, _rx) = live_runtime();
    open(&mut rt);
    write(&mut rt, "It crashed");
    tap(&mut rt, "feedback-send");
    rt.update(|ui, _| {
        ui.apply_feedback(FeedbackResult::Failed {
            error: "Feedback is not available in this build.".into(),
            report: String::new(),
        });
    });
    settle(&mut rt);
    assert!(rt.find("feedback-copy").is_none());
}

#[test]
fn an_answer_that_nobody_waits_for_is_ignored() {
    let (mut rt, _rx) = live_runtime();
    rt.update(|ui, _| ui.apply_feedback(FeedbackResult::Sent));
    assert_eq!(rt.state.feedback.phase, FeedbackPhase::Editing);
}

struct FakeApi {
    outcome: Mutex<Result<(), String>>,
    seen: Mutex<Vec<(Option<String>, FeedbackReport)>>,
}

impl FeedbackTransport for FakeApi {
    fn send(&self, report: &FeedbackReport) -> Result<(), FeedbackError> {
        self.seen.lock().unwrap().push((
            std::thread::current().name().map(str::to_owned),
            report.clone(),
        ));
        self.outcome
            .lock()
            .unwrap()
            .clone()
            .map_err(|reason| FeedbackError::Request {
                url: "https://api.test".into(),
                reason,
            })
    }
}

fn pump(rt: &mut Runtime<Ui>, rx: &Receiver<UiMsg>, done: impl Fn(&Ui) -> bool) {
    while let Ok(msg) = rx.recv_timeout(Duration::from_secs(5)) {
        rt.update(move |ui, tx| ui.apply(msg, tx));
        settle(rt);
        if done(&rt.state) {
            return;
        }
    }
    panic!("no answer, at {:?}", rt.state.feedback.phase);
}

#[test]
fn the_dialog_walks_through_failure_and_success_with_a_fake_transport() {
    let api = Arc::new(FakeApi {
        outcome: Mutex::new(Err("offline".into())),
        seen: Mutex::default(),
    });
    let dir = tempfile::tempdir().unwrap();
    let (ui_tx, ui_rx) = unbounded();
    let league: LeagueFactory = Box::new(|_: Sender<LeagueMsg>| Box::new(()));
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
        feedback: Some(FeedbackConfig {
            transport: api.clone(),
            app_version: "0.1.9".into(),
            os: Some("macOS 15".into()),
            logs_dir: dir.path().to_owned(),
            home_dirs: Vec::new(),
        }),
        hotkeys: None,
    });
    let ui = Ui::live(Settings::default(), engine.cmd.clone());
    let mut rt = super::runtime_sized(ui, (960.0, 1400.0));
    open(&mut rt);
    write(&mut rt, "It crashed");

    tap(&mut rt, "feedback-send");
    assert!(rt.state.feedback.sending());
    pump(&mut rt, &ui_rx, |ui| {
        matches!(ui.feedback.phase, FeedbackPhase::Failed { .. })
    });
    assert!(rt.find("feedback-copy").is_some());

    *api.outcome.lock().unwrap() = Ok(());
    tap(&mut rt, "feedback-send");
    pump(&mut rt, &ui_rx, |ui| {
        ui.feedback.phase == FeedbackPhase::Sent
    });
    assert!(has_text(&rt, "Thanks, we got it."));

    let seen = api.seen.lock().unwrap();
    assert_eq!(seen.len(), 2);
    assert!(
        seen.iter()
            .all(|(thread, _)| thread.as_deref() == Some("engine-feedback"))
    );
    assert_eq!(seen[0].1.client_id, seen[1].1.client_id);
    assert_eq!(seen[0].1.message, "It crashed");
}
