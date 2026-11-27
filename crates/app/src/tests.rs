use greyscreen_core::categories::CategorySet;
use rune::{Env, Key, KeyEvent, Mods, Runtime};

use crate::ui::Ui;
use crate::ui::fake_player::{self};
use crate::view;

mod playback;
mod review_volume;
mod scrub;
mod seek_preview;
mod video_click;
mod windows_playback;

fn runtime_with(ui: Ui) -> Runtime<Ui> {
    runtime_sized(ui, (1280.0, 720.0))
}

fn runtime_sized(ui: Ui, size: (f32, f32)) -> Runtime<Ui> {
    view::champion::without_icons();
    let mut rt = Runtime::new(ui, view::root, size, Env::new(true, 1.0));
    rt.frame(0.0);
    rt
}

fn runtime() -> Runtime<Ui> {
    runtime_with(Ui::preview())
}

fn tap(rt: &mut Runtime<Ui>, key: &str) {
    if rt.state.settings_open && key != "nav-settings" && key != "dock-settings" {
        let target = key.to_owned();
        rt.update(move |_, tx| tx.scroll_into_view(target));
        for _ in 0..40 {
            rt.frame(1.0 / 60.0);
        }
    }
    let (x, y) = rt.find(key).unwrap_or_else(|| panic!("no element {key}"));
    rt.pointer_down(x, y);
    rt.pointer_up(x, y);
    rt.frame(1.0 / 60.0);
}

fn review_runtime() -> Runtime<Ui> {
    review_runtime_showing(CategorySet::all())
}

fn review_runtime_showing(shown: CategorySet) -> Runtime<Ui> {
    let mut ui = Ui::preview();
    ui.settings.review_events = shown;
    ui.player_factory = fake_player::factory;
    let mut rt = runtime_sized(ui, (1400.0, 680.0));
    rt.update(|ui, _| ui.open_review_entry(crate::ui::review_entry()));
    rt.frame(1.0 / 60.0);
    rt.frame(2.0 / 60.0);
    fake_player::clear_calls();
    rt
}

#[test]
fn the_resolution_menu_opens_under_its_trigger_and_picks_a_value() {
    let mut rt = runtime();
    tap(&mut rt, "nav-settings");
    tap(&mut rt, "settings-nav-recording");
    tap(&mut rt, "resolution-trigger");
    for _ in 0..30 {
        rt.frame(1.0 / 60.0);
    }
    assert!(rt.state.resolution_open);
    let (trigger, _) = rt.presented("resolution-trigger").expect("trigger");
    let (menu, _) = rt.presented("resolution-content").expect("menu");
    let trigger_bottom = trigger[1] + trigger[3];
    assert!(
        (menu[1] - trigger_bottom).abs() < 12.0,
        "menu top {} should sit under the trigger bottom {trigger_bottom}",
        menu[1]
    );
    let trigger_right = trigger[0] + trigger[2];
    let menu_right = menu[0] + menu[2];
    assert!(
        (menu_right - trigger_right).abs() < 12.0,
        "menu right edge {menu_right} should line up with the trigger {trigger_right}"
    );
    tap(&mut rt, "option3");
    assert!(!rt.state.resolution_open);
}

fn press(rt: &mut Runtime<Ui>, key: Key) {
    rt.key(KeyEvent {
        key,
        mods: Mods::default(),
    });
    rt.frame(1.0 / 60.0);
}

fn press_char(rt: &mut Runtime<Ui>, c: &str) {
    press(rt, Key::Text(c.into()));
}

fn rect(rt: &Runtime<Ui>, key: &str) -> [f32; 4] {
    rt.presented(key)
        .unwrap_or_else(|| panic!("no presented {key}"))
        .0
}

fn refresh(rt: &mut Runtime<Ui>) {
    rt.update(|_, _| {});
    rt.frame(1.0 / 60.0);
}
