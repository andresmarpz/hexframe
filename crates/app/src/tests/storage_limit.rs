use greyscreen_core::settings::Settings;
use greyscreen_engine::EngineCmd;
use rune::{Env, Runtime};

use super::{tap, visible_texts};
use crate::ui::Ui;
use crate::view;

fn settings_runtime() -> (Runtime<Ui>, crossbeam_channel::Receiver<EngineCmd>) {
    let (cmd, rx) = crossbeam_channel::unbounded();
    let ui = Ui::live(Settings::default(), cmd);
    let mut rt = Runtime::new(ui, view::root, (960.0, 2600.0), Env::new(true, 1.0));
    rt.frame(0.0);
    tap(&mut rt, "nav-settings");
    tap(&mut rt, "settings-nav-storage");
    (rt, rx)
}

fn has_text(rt: &Runtime<Ui>, wanted: &str) -> bool {
    visible_texts(rt).iter().any(|t| t == wanted)
}

fn open_storage_menu(rt: &mut Runtime<Ui>) {
    tap(rt, "storage-limit-trigger");
    for _ in 0..30 {
        rt.frame(1.0 / 60.0);
    }
}

#[test]
fn the_storage_limit_row_is_off_by_default_and_warns_about_permanent_deletes() {
    let (rt, _rx) = settings_runtime();
    assert!(has_text(&rt, "Storage limit"));
    assert!(has_text(&rt, "Off"));
    assert!(has_text(
        &rt,
        "Oldest full-game videos are deleted permanently when recordings pass this size. Clips are kept as long as possible."
    ));
}

#[test]
fn the_storage_menu_lists_off_and_the_four_limits() {
    let (mut rt, _rx) = settings_runtime();
    open_storage_menu(&mut rt);
    assert!(rt.state.storage_open);
    for label in ["50 GB", "100 GB", "250 GB", "500 GB"] {
        assert!(has_text(&rt, label), "missing {label}");
    }
}

#[test]
fn picking_a_limit_stores_it_and_tells_the_engine() {
    let (mut rt, rx) = settings_runtime();
    open_storage_menu(&mut rt);
    tap(&mut rt, "option3");
    assert!(!rt.state.storage_open);
    assert_eq!(rt.state.settings.storage_limit_gb, Some(250));
    match rx.try_iter().last() {
        Some(EngineCmd::ApplySettings(settings)) => {
            assert_eq!(settings.storage_limit_gb, Some(250));
        }
        other => panic!("expected the settings, got {other:?}"),
    }
    assert!(has_text(&rt, "250 GB"));
}

#[test]
fn picking_off_removes_the_limit() {
    let (mut rt, _rx) = settings_runtime();
    rt.update(|ui, _| ui.settings.storage_limit_gb = Some(100));
    rt.frame(1.0 / 60.0);
    assert!(has_text(&rt, "100 GB"));
    open_storage_menu(&mut rt);
    tap(&mut rt, "option0");
    assert_eq!(rt.state.settings.storage_limit_gb, None);
    assert!(has_text(&rt, "Off"));
}
