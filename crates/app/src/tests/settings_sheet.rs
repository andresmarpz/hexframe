use rune::{Key, KeyEvent, Mods};

use super::{runtime, settle, tap, visible_texts};
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
    assert!(rt.find("quality-row").is_some());
    assert!(rt.find("clips-enabled").is_none());
    tap(&mut rt, "settings-nav-clips");
    assert_eq!(rt.state.settings_section, SettingsSection::Clips);
    assert!(rt.find("clips-enabled").is_some());
    assert!(rt.find("moments").is_some());
    assert!(rt.find("quality-row").is_none());
    tap(&mut rt, "settings-nav-storage");
    assert!(rt.find("folder-row").is_some());
    assert!(rt.find("storage-limit-row").is_some());
    tap(&mut rt, "settings-nav-about");
    assert!(rt.find("update-row").is_some());
    assert!(rt.find("riot-notice").is_some());
}

#[test]
fn the_quality_row_carries_the_size_estimate() {
    let rt = open_sheet();
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
    assert!(rt.find("quality-row").is_some());
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
fn opening_a_game_closes_the_sheet() {
    let mut rt = open_sheet();
    rt.update(|ui, _| ui.open_review_entry(crate::ui::review_entry()));
    settle(&mut rt);
    assert!(!rt.state.settings_open);
}

#[test]
fn recording_shows_the_next_game_notice() {
    let rt = open_sheet();
    assert!(rt.find("settings-next-game").is_some());
}
