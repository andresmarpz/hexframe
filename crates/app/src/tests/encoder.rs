use greyscreen_core::encoder::{
    CheckOutcome, EncoderCheck, FAILED_TITLE, SOFTWARE_LABEL, SOFTWARE_NOTICE,
};
use greyscreen_core::problem::Problem;
use greyscreen_engine::{NoticeAction, NoticeLevel, UiMsg};
use rune::Runtime;

use super::{live_runtime, settle, tap, visible_texts};
use crate::ui::{SettingsSection, Ui};

fn check(outcome: CheckOutcome, encoder: &str) -> EncoderCheck {
    EncoderCheck {
        key: "k".into(),
        outcome,
        encoder: encoder.into(),
        detail: String::new(),
        first_failure: String::new(),
        noticed: false,
        verdicts: Vec::new(),
    }
}

fn receive(rt: &mut Runtime<Ui>, msg: UiMsg) {
    rt.update(move |ui, tx| ui.apply(msg, tx));
    settle(rt);
}

fn open_recording_settings(rt: &mut Runtime<Ui>) {
    rt.update(|ui, _| ui.open_settings(SettingsSection::Recording));
    settle(rt);
}

#[test]
fn a_failed_check_shows_what_to_try_on_home() {
    let (mut rt, _rx) = live_runtime();
    assert!(rt.find("encoder-error").is_none());
    receive(&mut rt, UiMsg::Encoder(check(CheckOutcome::Failed, "")));
    assert!(rt.find("encoder-error").is_some());
    assert!(visible_texts(&rt).contains(&FAILED_TITLE.to_owned()));
    tap(&mut rt, "encoder-faq");
    assert_eq!(rt.take_url_requests(), vec![Problem::NoEncoder.help_url()]);
}

#[test]
fn the_failed_card_opens_the_first_failure_line() {
    let (mut rt, _rx) = live_runtime();
    let mut failed = check(CheckOutcome::Failed, "");
    failed.first_failure = "NVIDIA NVENC (hardware) on NVIDIA GeForce GT 1030: E_FAIL".into();
    receive(&mut rt, UiMsg::Encoder(failed));
    assert!(rt.find("encoder-detail").is_some());
    assert!(rt.find("encoder-detail-line").is_none());
    tap(&mut rt, "encoder-detail");
    assert!(
        visible_texts(&rt)
            .contains(&"NVIDIA NVENC (hardware) on NVIDIA GeForce GT 1030: E_FAIL".to_owned())
    );
}

#[test]
fn a_failed_card_without_a_failure_line_has_no_details() {
    let (mut rt, _rx) = live_runtime();
    receive(&mut rt, UiMsg::Encoder(check(CheckOutcome::Failed, "")));
    assert!(rt.find("encoder-detail").is_none());
}

#[test]
fn a_working_encoder_shows_no_error_on_home() {
    let (mut rt, _rx) = live_runtime();
    receive(
        &mut rt,
        UiMsg::Encoder(check(CheckOutcome::Hardware, "NVIDIA NVENC (hardware)")),
    );
    assert!(rt.find("encoder-error").is_none());
}

#[test]
fn recording_settings_name_the_encoder_once_checked() {
    let (mut rt, _rx) = live_runtime();
    open_recording_settings(&mut rt);
    assert!(rt.find("encoder-row").is_none());
    receive(
        &mut rt,
        UiMsg::Encoder(check(CheckOutcome::Hardware, "NVIDIA NVENC (hardware)")),
    );
    assert!(visible_texts(&rt).contains(&"NVIDIA NVENC (hardware)".to_owned()));
    receive(
        &mut rt,
        UiMsg::Encoder(check(CheckOutcome::Software, SOFTWARE_LABEL)),
    );
    assert!(visible_texts(&rt).contains(&"Windows software encoder — uses more CPU".to_owned()));
}

#[test]
fn the_software_notice_is_shown_as_a_warning() {
    let (mut rt, _rx) = live_runtime();
    receive(
        &mut rt,
        UiMsg::NoticeWithAction {
            level: NoticeLevel::Warn,
            text: SOFTWARE_NOTICE.to_owned(),
            action: NoticeAction::Link(Problem::NoEncoder.help_url()),
        },
    );
    assert!(rt.find("notice").is_some());
    assert_eq!(
        rt.state.notice.as_ref().map(|(n, _)| n.text.as_str()),
        Some(SOFTWARE_NOTICE)
    );
}
