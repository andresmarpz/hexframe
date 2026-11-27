use greyscreen_core::settings::Settings;
use greyscreen_engine::{NoticeAction, NoticeLevel, UiMsg};
use rune::Tx;

use super::Ui;

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
