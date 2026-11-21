use greyscreen_core::settings::{NavStyle, Quality, Resolution, resolution_options};
use greyscreen_engine::EngineStatus;
use rune::{Drag, El, Phase, row, text};
use rune_kit::{Kit, SegmentedControl, Select, SelectAppearance};

use super::catalog::Row;
use super::{setting_row, toggle_row};
use crate::ui::{Ui, format_size_rate};
use crate::view::widgets::{percent, volume_at, volume_slider};

const QUALITIES: [(Quality, &str); 3] = [
    (Quality::Balanced, "Balanced"),
    (Quality::High, "High"),
    (Quality::Max, "Max"),
];
const NAV_STYLES: [(NavStyle, &str); 2] =
    [(NavStyle::Sidebar, "Sidebar"), (NavStyle::Dock, "Dock")];
const NAV_STYLE_WIDTH: f32 = 160.0;
const QUALITY_NOTE: &str = "Higher quality uses more disk space.";
const FALLBACK_NATIVE_HEIGHT: u32 = 1440;
const FPS_OPTIONS: [u32; 2] = [30, 60];
const QUALITY_WIDTH: f32 = 252.0;
const FPS_WIDTH: f32 = 120.0;
const SOUND_VOLUME_WIDTH: f32 = 200.0;
const PERCENT_WIDTH: f32 = 32.0;
const RESOLUTION_WIDTH: f32 = 180.0;
const NEXT_GAME: &str =
    "Recording now. Quality, resolution and frame rate changes apply to your next game.";

pub(super) fn notice(s: &Ui, k: &Kit) -> Option<El> {
    matches!(s.status, EngineStatus::Recording { .. }).then(|| {
        row()
            .key("settings-next-game")
            .w_full()
            .px(12.0)
            .py(8.0)
            .radius(8.0)
            .fill(k.c.info.bg)
            .child(
                text(NEXT_GAME)
                    .wrap()
                    .w_full()
                    .font(k.ty.small)
                    .color(k.c.info.fg),
            )
    })
}

pub(super) fn render(s: &Ui, k: &Kit, layer: &mut Vec<El>, which: Row) -> El {
    let settings = &s.settings;
    match which {
        Row::RecordGames => toggle_row(
            k,
            "auto-record",
            "Record games",
            Some("Start recording when a League game starts."),
            settings.auto_record,
            |ui| ui.change(|s| s.auto_record = !s.auto_record),
        ),
        Row::FullGame => toggle_row(
            k,
            "full-game",
            "Record the full game",
            Some("Off keeps only highlight clips."),
            settings.record_full_game,
            |ui| ui.change(|s| s.record_full_game = !s.record_full_game),
        ),
        Row::Quality => {
            let note = quality_note(s, native_height(&(s.display_sizes)()));
            setting_row(k, "quality-row", "Quality", Some(&note), quality(s, k))
        }
        Row::Resolution => setting_row(
            k,
            "resolution-row",
            "Resolution",
            Some("Larger games are scaled down to this."),
            resolution(s, k, layer),
        ),
        Row::FrameRate => setting_row(
            k,
            "fps-row",
            "Frame rate",
            Some("60 is smoother, 30 uses less disk space."),
            fps(s, k),
        ),
        Row::Audio => toggle_row(
            k,
            "audio",
            "Record game audio",
            Some("Records League's own audio. Discord and your microphone aren't recorded."),
            settings.audio,
            |ui| ui.change(|s| s.audio = !s.audio),
        ),
        Row::Cursor => toggle_row(
            k,
            "capture-cursor",
            "Show cursor",
            Some("Include the mouse cursor, as the game draws it."),
            settings.capture_cursor,
            |ui| ui.change(|s| s.capture_cursor = !s.capture_cursor),
        ),
        Row::NavStyle => setting_row(
            k,
            "nav-style-row",
            "Navigation style",
            Some("A side panel, or a floating dock at the bottom."),
            nav_style(s, k),
        ),
        Row::SoundEffects => toggle_row(
            k,
            "sound-effects",
            "Sound effects",
            Some(
                "A soft sound when a recording starts or ends, a clip is saved or something fails.",
            ),
            settings.sound_effects,
            |ui| ui.change(|s| s.sound_effects = !s.sound_effects),
        ),
        _ => setting_row(
            k,
            "sound-volume-row",
            "Sound volume",
            None,
            sound_volume(s, k),
        ),
    }
}

fn sound_volume(s: &Ui, k: &Kit) -> El {
    let share = s.settings.sound_volume as f32 / 100.0;
    let slider = volume_slider(k, "sound-volume", share)
        .on_step(|ui: &mut Ui, delta, _| ui.step_sound_volume(delta))
        .on_step_limit(|ui: &mut Ui, max, _| {
            ui.drag_sound_volume(if max { 100 } else { 0 });
            ui.finish_sound_volume();
        })
        .on_drag(|ui: &mut Ui, drag: &mut Drag, _| {
            if drag.phase == Phase::Up {
                ui.finish_sound_volume();
            } else {
                ui.drag_sound_volume(percent(volume_at(drag.x, drag.w)));
            }
        });
    row()
        .w(SOUND_VOLUME_WIDTH)
        .gap(12.0)
        .center_y()
        .child(slider)
        .child(
            text(format!("{}", s.settings.sound_volume))
                .key("sound-volume-percent")
                .font(k.ty.small_medium)
                .color(k.c.text_muted)
                .w(PERCENT_WIDTH),
        )
}

fn nav_style(s: &Ui, k: &Kit) -> El {
    SegmentedControl::new(
        &NAV_STYLES.map(|(_, label)| label),
        NAV_STYLES
            .iter()
            .position(|(style, _)| *style == s.settings.nav_style),
    )
    .label("Navigation style")
    .width(Some(NAV_STYLE_WIDTH))
    .on_pick(|ui: &mut Ui, i: usize, _: &mut rune::Tx| ui.change(|s| s.nav_style = NAV_STYLES[i].0))
    .build(k)
    .key("nav-style")
}

fn quality(s: &Ui, k: &Kit) -> El {
    SegmentedControl::new(
        &QUALITIES.map(|(_, label)| label),
        QUALITIES.iter().position(|(q, _)| *q == s.settings.quality),
    )
    .label("Quality")
    .width(Some(QUALITY_WIDTH))
    .on_pick(|ui: &mut Ui, i: usize, _: &mut rune::Tx| ui.change(|s| s.quality = QUALITIES[i].0))
    .build(k)
    .key("quality")
}

fn resolution(s: &Ui, k: &Kit, layer: &mut Vec<El>) -> El {
    let options = resolution_options(&(s.display_sizes)(), s.settings.resolution);
    let labels: Vec<&str> = options.iter().map(|o| o.label.as_str()).collect();
    let current = options
        .iter()
        .find(|o| o.resolution == s.settings.resolution)
        .map(|o| o.label.as_str());
    Select::new("resolution", &labels)
        .current(current)
        .open(s.resolution_open)
        .appearance(SelectAppearance::Field)
        .width(RESOLUTION_WIDTH)
        .label("Resolution")
        .on_toggle(|ui: &mut Ui| ui.resolution_open = !ui.resolution_open)
        .on_close(|ui: &mut Ui| ui.resolution_open = false)
        .on_choose(move |ui: &mut Ui, value: &str, _: &mut rune::Tx| {
            ui.resolution_open = false;
            if let Some(option) = options.iter().find(|o| o.label == value) {
                let resolution = option.resolution;
                ui.change(|s| s.resolution = resolution);
            }
        })
        .build(k, layer)
}

fn fps(s: &Ui, k: &Kit) -> El {
    SegmentedControl::new(
        &FPS_OPTIONS.map(|f| f.to_string()),
        FPS_OPTIONS.iter().position(|f| *f == s.settings.fps),
    )
    .label("Frame rate")
    .width(Some(FPS_WIDTH))
    .on_pick(|ui: &mut Ui, i: usize, _: &mut rune::Tx| ui.change(|s| s.fps = FPS_OPTIONS[i]))
    .build(k)
    .key("fps")
}

fn quality_note(s: &Ui, display_height: Option<u32>) -> String {
    let native_height = display_height.unwrap_or(FALLBACK_NATIVE_HEIGHT);
    let native = (s.settings.resolution == Resolution::Native).then_some(native_height);
    let rate = format_size_rate(s.settings.estimated_mb_per_minute(native_height), native);
    format!("{QUALITY_NOTE} {rate}")
}

fn native_height(displays: &[(u32, u32)]) -> Option<u32> {
    displays.iter().map(|&(_, h)| h).filter(|&h| h > 0).max()
}
