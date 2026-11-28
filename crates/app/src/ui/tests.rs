use greyscreen_core::settings::Settings;
use greyscreen_engine::{NoticeAction, NoticeLevel, UiMsg};
use rune::Tx;

use super::{Ui, preview};

fn ui() -> Ui {
    Ui::blank(Settings::default())
}

#[test]
fn a_notice_with_an_action_stays_long_enough_to_press_it() {
    let mut ui = ui();
    let mut tx = Tx::default();
    ui.apply(
        UiMsg::NoticeWithAction {
            level: NoticeLevel::Warn,
            text: "Not enough disk space to record (1.4 GB free).".into(),
            action: NoticeAction::Link("https://example.com".into()),
        },
        &mut tx,
    );
    assert!(!ui.poll(4.0, &mut tx));
    let (notice, _) = ui.notice.clone().expect("notice");
    assert_eq!(
        notice.action,
        Some(NoticeAction::Link("https://example.com".into()))
    );
    assert!(ui.poll(12.0, &mut tx));
    assert!(ui.notice.is_none());
}

#[test]
fn home_shows_the_newest_game_after_the_library_loads() {
    let mut ui = ui();
    let mut tx = Tx::default();
    assert!(ui.latest_game().is_none());
    let mut library = preview::demo_recordings();
    library.reverse();
    let newest = library
        .iter()
        .max_by_key(|rec| rec.meta.started_at.timestamp())
        .map(|rec| rec.meta.id.clone());
    ui.apply(UiMsg::Library(library), &mut tx);
    assert_eq!(ui.latest_game().map(|rec| rec.meta.id.clone()), newest);
}
