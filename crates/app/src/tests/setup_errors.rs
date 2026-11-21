use greyscreen_core::problem::{FOLDER_UNUSABLE, NO_FRAMES, Problem};
use greyscreen_engine::{EngineStatus, UiMsg};

use super::{live_runtime, runtime, settle, tap, visible_texts};
use crate::tray::Link;
use crate::ui::Screen;

fn fail(rt: &mut rune::Runtime<crate::ui::Ui>, raw: &str) {
    let raw = raw.to_owned();
    rt.update(move |ui, _| {
        ui.status = EngineStatus::Error(raw);
        ui.live_selected = false;
        ui.go(Screen::Home);
    });
    settle(rt);
}

fn shows(rt: &rune::Runtime<crate::ui::Ui>, needle: &str) -> bool {
    visible_texts(rt).iter().any(|t| t.contains(needle))
}

#[test]
fn each_problem_shows_what_happened_and_the_one_thing_to_do() {
    let mut rt = runtime();
    let folder = format!("{FOLDER_UNUSABLE}: D:\\Games");
    let cases = [
        (NO_FRAMES, Problem::ExclusiveFullscreen),
        (
            "the captured window closed during startup",
            Problem::LeagueNotFound,
        ),
        (folder.as_str(), Problem::FolderUnusable),
        (
            "There is not enough space on the disk. (os error 112)",
            Problem::LowDisk,
        ),
        (
            "video encoder failed: no usable hardware H.264 encoder",
            Problem::NoEncoder,
        ),
        (
            "windows graphics capture is not supported",
            Problem::WindowsTooOld,
        ),
        ("Access is denied. (0x80070005)", Problem::CaptureBlocked),
        ("something odd", Problem::Other),
    ];
    for (raw, problem) in cases {
        fail(&mut rt, raw);
        assert!(shows(&rt, problem.title()), "{problem:?} title");
        assert!(shows(&rt, problem.action()), "{problem:?} action");
        assert!(shows(&rt, problem.status_title()), "{problem:?} sidebar");
        assert!(rt.find("error-learn-more").is_some(), "{problem:?} link");
        assert_eq!(
            rt.find("error-choose-folder").is_some(),
            problem.offers_folder()
        );
    }
}

#[test]
fn an_unknown_failure_still_shows_the_raw_message() {
    let mut rt = runtime();
    fail(&mut rt, "something odd");
    assert!(shows(&rt, "something odd"));
    assert!(rt.find("error-open-logs").is_some());
}

#[test]
fn a_known_failure_hides_the_raw_message() {
    let mut rt = runtime();
    fail(&mut rt, NO_FRAMES);
    assert!(rt.find("error-message").is_none());
    assert!(rt.find("error-open-logs").is_none());
}

#[test]
fn learn_more_opens_the_faq_at_the_anchor_for_the_problem() {
    let mut rt = runtime();
    fail(&mut rt, NO_FRAMES);
    tap(&mut rt, "error-learn-more");
    assert_eq!(
        rt.take_url_requests(),
        ["https://greyscreen-lake.vercel.app/faq#fullscreen"]
    );
}

#[test]
fn retry_asks_the_engine_to_try_again() {
    let (mut rt, rx) = live_runtime();
    fail(&mut rt, NO_FRAMES);
    tap(&mut rt, "retry-capture");
    assert_eq!(rx.try_recv(), Ok(greyscreen_engine::EngineCmd::Retry));
}

#[test]
fn the_dock_marks_the_error() {
    let mut rt = runtime();
    rt.update(|ui, _| ui.settings.nav_style = greyscreen_core::settings::NavStyle::Dock);
    fail(&mut rt, NO_FRAMES);
    assert!(rt.find("dock-armed").is_some());
    assert!(shows(&rt, "Error"));
}

#[test]
fn status_and_settings_changes_reach_the_tray() {
    let (link, rx) = Link::detached();
    let mut rt = runtime();
    rt.update(move |ui, _| ui.tray = Some(link));
    rt.update(|ui, tx| ui.apply(UiMsg::Status(EngineStatus::Waiting), tx));
    let state = rx.try_iter().last().expect("a tray state");
    assert_eq!(state.status, EngineStatus::Waiting);
    assert!(state.auto_record);
    rt.update(|ui, _| ui.change(|s| s.auto_record = false));
    let state = rx.try_iter().last().expect("a tray state");
    assert!(!state.auto_record);
}
