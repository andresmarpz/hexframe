use rune::{Key, KeyEvent, Mods};

use super::{nav_rect_of, runtime, runtime_sized, settle, tap, visible_texts};
use crate::ui::SettingsSection;

fn open_sheet() -> rune::Runtime<crate::ui::Ui> {
    let mut rt = runtime();
    tap(&mut rt, "nav-settings");
    settle(&mut rt);
    rt
}

fn type_search(rt: &mut rune::Runtime<crate::ui::Ui>, text: &str) {
    rt.update(|ui, _| ui.settings_search = rune::TextEdit::new(text));
    settle(rt);
}

fn has_text(rt: &rune::Runtime<crate::ui::Ui>, wanted: &str) -> bool {
    visible_texts(rt).iter().any(|t| t == wanted)
}

#[test]
fn the_gear_opens_a_sheet_over_the_current_detail() {
    let rt = open_sheet();
    assert!(rt.state.settings_open);
    assert_eq!(rt.state.detail(), crate::ui::Detail::Live);
    assert!(rt.find("settings-sheet").is_some());
    assert!(rt.find("nav-settings").is_some());
}

#[test]
fn escape_closes_the_sheet_and_keeps_the_screen() {
    let mut rt = open_sheet();
    rt.key(KeyEvent {
        key: Key::Escape,
        mods: Mods::default(),
    });
    settle(&mut rt);
    assert!(!rt.state.settings_open);
    assert!(rt.find("settings-sheet").is_none());
}

#[test]
fn the_close_button_closes_the_sheet() {
    let mut rt = open_sheet();
    tap(&mut rt, "close");
    assert!(!rt.state.settings_open);
}

#[test]
fn toggling_settings_closes_an_open_sheet() {
    let mut rt = open_sheet();
    rt.update(|ui, _| ui.toggle_settings());
    settle(&mut rt);
    assert!(!rt.state.settings_open);
}

#[test]
fn each_section_shows_only_its_rows() {
    let mut rt = open_sheet();
    assert_eq!(rt.state.settings_section, SettingsSection::General);
    assert!(rt.find("nav-style-row").is_some());
    assert!(rt.find("sound-effects").is_some());
    assert!(rt.find("update-row").is_some());
    assert!(rt.find("recording-diagnostics").is_some());
    assert!(rt.find("riot-notice").is_some());
    assert!(rt.find("quality-row").is_none());
    assert!(rt.find("clips-enabled").is_none());
    tap(&mut rt, "settings-nav-recording");
    assert_eq!(rt.state.settings_section, SettingsSection::Recording);
    assert!(rt.find("quality-row").is_some());
    assert!(rt.find("fps-row").is_some());
    assert!(rt.find("resolution-row").is_some());
    assert!(rt.find("nav-style-row").is_none());
    tap(&mut rt, "settings-nav-clips");
    assert_eq!(rt.state.settings_section, SettingsSection::Clips);
    assert!(rt.find("clips-enabled").is_some());
    assert!(rt.find("moments").is_some());
    assert!(rt.find("review-events-row").is_some());
    assert!(rt.find("review-events-reset").is_some());
    assert!(rt.find("quality-row").is_none());
    tap(&mut rt, "settings-nav-storage");
    assert!(rt.find("folder-row").is_some());
    assert!(rt.find("storage-limit-row").is_some());
    tap(&mut rt, "settings-nav-keybinds");
    assert!(rt.find("bind-save_clip").is_some());
}

#[test]
fn the_quality_row_carries_the_size_estimate() {
    let mut rt = open_sheet();
    tap(&mut rt, "settings-nav-recording");
    assert!(
        visible_texts(&rt)
            .iter()
            .any(|t| t.contains("MB per minute") && t.ends_with('.'))
    );
}

#[test]
fn searching_fps_shows_the_frame_rate_row_across_sections() {
    let mut rt = open_sheet();
    tap(&mut rt, "settings-nav-storage");
    type_search(&mut rt, "fps");
    assert!(rt.find("fps-row").is_some());
    assert!(rt.find("folder-row").is_none());
    assert!(rt.find("quality-row").is_none());
}

#[test]
fn searching_gathers_matches_from_several_sections() {
    let mut rt = open_sheet();
    type_search(&mut rt, "clip");
    assert!(rt.find("clips-enabled").is_some());
    assert!(rt.find("bind-save_clip").is_some());
    assert!(rt.find("fps-row").is_none());
}

#[test]
fn a_search_with_no_match_says_so() {
    let mut rt = open_sheet();
    type_search(&mut rt, "zzzz");
    assert!(rt.find("settings-no-results").is_some());
    assert!(has_text(&rt, "No settings match “zzzz”"));
}

#[test]
fn clearing_the_search_returns_to_the_section() {
    let mut rt = open_sheet();
    type_search(&mut rt, "fps");
    type_search(&mut rt, "");
    assert!(rt.find("nav-style-row").is_some());
}

#[test]
fn picking_a_section_clears_the_search() {
    let mut rt = open_sheet();
    type_search(&mut rt, "fps");
    rt.update(|ui, _| ui.pick_settings_section(SettingsSection::Storage));
    settle(&mut rt);
    assert!(rt.state.settings_search.text.is_empty());
    assert!(rt.find("folder-row").is_some());
}

#[test]
fn turning_clips_off_dims_the_moments_and_disables_the_checklist() {
    let mut rt = open_sheet();
    tap(&mut rt, "settings-nav-clips");
    tap(&mut rt, "clips-enabled-switch");
    assert!(!rt.state.settings.clips.enabled);
    let before = rt.state.settings.clips.kills;
    tap(&mut rt, "clip-kills");
    assert_eq!(rt.state.settings.clips.kills, before);
}

#[test]
fn a_moment_checkbox_flips_its_clip_setting() {
    let mut rt = open_sheet();
    tap(&mut rt, "settings-nav-clips");
    let before = rt.state.settings.clips.deaths;
    tap(&mut rt, "clip-deaths");
    assert_eq!(rt.state.settings.clips.deaths, !before);
}

#[test]
fn the_save_clip_row_links_to_keybinds() {
    let mut rt = open_sheet();
    tap(&mut rt, "settings-nav-clips");
    tap(&mut rt, "hotkey-change");
    assert_eq!(rt.state.settings_section, SettingsSection::Keybinds);
}

#[test]
fn the_storage_meter_reads_used_of_limit() {
    let mut rt = open_sheet();
    rt.update(|ui, _| ui.settings.storage_limit_gb = Some(50));
    tap(&mut rt, "settings-nav-storage");
    let used = crate::ui::format_size(rt.state.storage_used_bytes());
    assert!(has_text(&rt, &format!("{used} of 50 GB")));
}

#[test]
fn the_storage_meter_without_a_limit_says_so() {
    let mut rt = open_sheet();
    tap(&mut rt, "settings-nav-storage");
    assert!(
        visible_texts(&rt)
            .iter()
            .any(|t| t.ends_with("used, no limit"))
    );
}

#[test]
fn storage_progress_bar_only_shows_when_a_limit_is_set() {
    let mut rt = open_sheet();
    tap(&mut rt, "settings-nav-storage");
    assert!(rt.find("storage-limit-progress").is_none());
    rt.update(|ui, _| ui.settings.storage_limit_gb = Some(50));
    settle(&mut rt);
    assert!(rt.find("storage-limit-progress").is_some());
}

#[test]
fn opening_a_game_closes_the_sheet() {
    let mut rt = open_sheet();
    rt.update(|ui, _| ui.open_review_entry(crate::ui::review_entry()));
    settle(&mut rt);
    assert!(!rt.state.settings_open);
}

#[test]
fn recording_shows_the_next_game_notice() {
    let mut rt = open_sheet();
    tap(&mut rt, "settings-nav-recording");
    assert!(rt.find("settings-next-game").is_some());
}

fn short_sheet(open_menu: bool) -> rune::Runtime<crate::ui::Ui> {
    let mut rt = open_sheet();
    rt.set_viewport((1000.0, 480.0), 1.0);
    settle(&mut rt);
    if open_menu {
        tap(&mut rt, "settings-nav-recording");
        tap(&mut rt, "resolution-trigger");
        settle(&mut rt);
    }
    rt
}

fn bounds(rt: &rune::Runtime<crate::ui::Ui>, key: &str) -> [f32; 4] {
    rt.presented(key)
        .unwrap_or_else(|| panic!("no element {key}"))
        .0
}

#[test]
fn the_sheet_fits_a_short_window() {
    let rt = short_sheet(false);
    let sheet = bounds(&rt, "settings-dialog");
    assert!(sheet[1] >= 0.0 && sheet[1] + sheet[3] <= 480.0, "{sheet:?}");
    let body = bounds(&rt, "settings-sheet");
    assert!(body[3] > 200.0, "{body:?}");
}

#[test]
fn the_resolution_menu_stays_inside_the_sheet_in_a_short_window() {
    let rt = short_sheet(true);
    let sheet = bounds(&rt, "settings-dialog");
    let menu = bounds(&rt, "resolution-content");
    assert!(menu[1] >= sheet[1], "{menu:?} {sheet:?}");
    assert!(
        menu[1] + menu[3] <= sheet[1] + sheet[3],
        "{menu:?} {sheet:?}"
    );
}

#[test]
fn the_sheet_keeps_its_full_height_in_a_tall_window() {
    let rt = open_sheet();
    assert_eq!(bounds(&rt, "settings-sheet")[3], 532.0);
}

fn press_escape(rt: &mut rune::Runtime<crate::ui::Ui>) {
    rt.key(KeyEvent {
        key: Key::Escape,
        mods: Mods::default(),
    });
    settle(rt);
}

fn focused_search() -> rune::Runtime<crate::ui::Ui> {
    let mut rt = open_sheet();
    type_search(&mut rt, "fps");
    rt.update(|_, tx| tx.focus("settings-search", true));
    settle(&mut rt);
    rt
}

#[test]
fn escape_in_the_search_clears_the_text_and_keeps_the_sheet() {
    let mut rt = focused_search();
    press_escape(&mut rt);
    assert!(rt.state.settings_search.text.is_empty());
    assert!(rt.state.settings_open);
    assert!(rt.find("nav-style-row").is_some());
}

#[test]
fn a_second_escape_closes_the_sheet() {
    let mut rt = focused_search();
    press_escape(&mut rt);
    press_escape(&mut rt);
    assert!(!rt.state.settings_open);
}

fn pick_nav_style(rt: &mut rune::Runtime<crate::ui::Ui>, dock: bool) {
    rt.update(|_, tx| tx.scroll_into_view("nav-style".to_owned()));
    settle(rt);
    let (x, y) = rt.find("nav-style").expect("nav-style");
    let x = if dock { x + 30.0 } else { x - 30.0 };
    rt.pointer_down(x, y);
    rt.pointer_up(x, y);
    settle(rt);
}

#[test]
fn the_navigation_style_row_lives_in_general() {
    let mut rt = open_sheet();
    assert!(rt.find("nav-style-row").is_some());
    assert!(rt.find("nav-style").is_some());
    tap(&mut rt, "settings-nav-recording");
    assert!(rt.find("nav-style-row").is_none());
}

#[test]
fn searching_dock_finds_the_navigation_style_row() {
    let mut rt = open_sheet();
    tap(&mut rt, "settings-nav-storage");
    type_search(&mut rt, "dock");
    assert!(rt.find("nav-style-row").is_some());
    assert!(rt.find("quality-row").is_none());
}

#[test]
fn picking_dock_swaps_the_shell_live_and_picking_sidebar_swaps_it_back() {
    use greyscreen_core::settings::NavStyle;
    let mut rt = open_sheet();
    assert!(rt.find("nav").is_some());
    assert!(rt.find("dock").is_none());
    pick_nav_style(&mut rt, true);
    assert_eq!(rt.state.settings.nav_style, NavStyle::Dock);
    assert!(rt.state.settings_open);
    assert!(rt.find("dock").is_some());
    assert!(rt.find("nav").is_none());
    pick_nav_style(&mut rt, false);
    assert_eq!(rt.state.settings.nav_style, NavStyle::Sidebar);
    assert!(rt.find("nav").is_some());
    assert!(rt.find("dock").is_none());
}

#[test]
fn the_navigation_style_choice_reaches_the_engine() {
    use greyscreen_core::settings::NavStyle;
    use greyscreen_engine::EngineCmd;
    let (mut rt, rx) = super::live_runtime();
    tap(&mut rt, "nav-settings");
    pick_nav_style(&mut rt, true);
    let Ok(EngineCmd::ApplySettings(sent)) = rx.try_recv() else {
        panic!("no ApplySettings sent");
    };
    assert_eq!(sent.nav_style, NavStyle::Dock);
}

fn sheet_rects(window: (f32, f32)) -> ([f32; 4], [f32; 4]) {
    let mut rt = runtime_sized(crate::ui::Ui::preview(), window);
    tap(&mut rt, "nav-settings");
    settle(&mut rt);
    (
        nav_rect_of(&rt, "settings-dialog"),
        nav_rect_of(&rt, "settings-sheet"),
    )
}

#[test]
fn the_sheet_grows_with_the_window_in_steps() {
    let cases = [
        ((1000.0, 600.0), 760.0, 484.0),
        ((1280.0, 760.0), 880.0, 532.0),
        ((1600.0, 900.0), 1040.0, 652.0),
        ((1920.0, 1080.0), 1040.0, 652.0),
        ((2560.0, 1440.0), 1200.0, 760.0),
        ((1920.0, 640.0), 760.0, 484.0),
        ((1920.0, 500.0), 760.0, 392.0),
        ((1000.0, 400.0), 760.0, 292.0),
    ];
    for (window, width, body) in cases {
        let (dialog, sheet) = sheet_rects(window);
        assert_eq!(dialog[2], width, "dialog width at {window:?}");
        assert_eq!(sheet[3], body, "body height at {window:?}");
        assert!(dialog[0] >= 16.0, "left margin at {window:?}");
        assert!(
            dialog[0] + dialog[2] <= window.0 - 16.0,
            "right margin at {window:?}"
        );
        assert!(dialog[1] >= 16.0, "top margin at {window:?}");
        assert!(
            dialog[1] + dialog[3] <= window.1 - 16.0,
            "bottom margin at {window:?}"
        );
    }
}

#[test]
fn the_wide_sheet_gives_the_nav_and_the_search_the_extra_room() {
    let mut rt = runtime_sized(crate::ui::Ui::preview(), (1920.0, 1080.0));
    tap(&mut rt, "nav-settings");
    settle(&mut rt);
    let nav = nav_rect_of(&rt, "settings-nav");
    assert_eq!(nav[2], 224.0);
    let search = nav_rect_of(&rt, "settings-search");
    assert!(
        search[2] > 0.0 && search[2] <= nav[2],
        "search fits inside nav"
    );
}

#[test]
fn the_diagnostics_switch_is_off_by_default_and_applies_through_the_engine() {
    let (mut rt, rx) = super::live_runtime();
    assert!(!rt.state.settings.recording_diagnostics);
    super::tap(&mut rt, "nav-settings");
    super::tap(&mut rt, "recording-diagnostics-switch");
    assert!(rt.state.settings.recording_diagnostics);
    let Ok(greyscreen_engine::EngineCmd::ApplySettings(sent)) = rx.try_recv() else {
        panic!("no ApplySettings sent");
    };
    assert!(sent.recording_diagnostics);
}

#[test]
fn searching_for_stutter_finds_the_diagnostics_row() {
    let mut rt = super::runtime();
    super::tap(&mut rt, "nav-settings");
    type_search(&mut rt, "stutter");
    assert!(rt.find("recording-diagnostics").is_some());
    assert!(rt.find("auto-record").is_none());
}
