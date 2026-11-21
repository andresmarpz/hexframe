use greyscreen_core::paths;
use greyscreen_core::settings::{Hotkey, Quality, Resolution, STORAGE_LIMITS_GB, resolution_options};
use greyscreen_engine::reveal;
use rune::{El, FileDialogOptions, Icon, col, row, text};
use rune_kit::{
    Button, ButtonVariant, ControlSize, Kit, SegmentedControl, Select, SelectAppearance, Switch,
    TextInput,
};

mod updates;

use super::{card, hairline, page_header};

use crate::ui::{Ui, format_size_estimate, parse_clip_secs};

const QUALITIES: [(Quality, &str); 3] = [
    (Quality::Balanced, "Balanced"),
    (Quality::High, "High"),
    (Quality::Max, "Max"),
];
const FALLBACK_NATIVE_HEIGHT: u32 = 1440;
const FPS_OPTIONS: [u32; 2] = [30, 60];
const CONTROL_WIDTH: f32 = 220.0;
const RESOLUTION_WIDTH: f32 = 200.0;
const STORAGE_WIDTH: f32 = 140.0;
const STORAGE_OFF: &str = "Off";
const STORAGE_LIMIT_NOTE: &str = "Oldest full-game videos are deleted permanently when recordings pass this size. Clips are kept as long as possible.";
const PRESS_A_KEY: &str = "Press a key…";
const RIOT_NOTICE: &str = "Greyscreen isn't endorsed by Riot Games and doesn't reflect the views or opinions of Riot Games or anyone officially involved in producing or managing Riot Games properties. Riot Games, and all associated properties are trademarks or registered trademarks of Riot Games, Inc.";

static INFO: Icon = Icon {
    name: "greyscreen-info",
    view: 16.0,
    body: r#"<circle cx="8" cy="8" r="6.5" stroke="currentColor" fill="none"/><path d="M8 7.2V11.2" stroke="currentColor" stroke-linecap="round"/><circle cx="8" cy="4.9" r="0.9" fill="currentColor"/>"#,
};

pub(super) fn page(s: &Ui, k: &Kit, layer: &mut Vec<El>) -> El {
    col()
        .key("settings")
        .w_full()
        .gap(16.0)
        .child(page_header(
            k,
            "Settings",
            "Changes are saved automatically.",
            None,
        ))
        .child(
            col()
                .w_full()
                .gap(20.0)
                .child(recording_section(s, k, layer))
                .child(clips_section(s, k))
                .child(storage_section(s, k, layer))
                .child(about_section(s, k)),
        )
}

fn section(k: &Kit, key: &str, title: &str, rows: Vec<El>) -> El {
    let mut body = card(k).key(key);
    let last = rows.len().saturating_sub(1);
    for (i, row) in rows.into_iter().enumerate() {
        body = body.child(row);
        if i < last {
            body = body.child(hairline(k));
        }
    }
    col()
        .w_full()
        .gap(8.0)
        .child(
            text(title.to_owned())
                .font(k.ty.ui_medium)
                .color(k.c.text_muted)
                .ml(4.0),
        )
        .child(body)
}

fn setting_row(k: &Kit, key: &str, label: &str, description: Option<&str>, control: El) -> El {
    let mut copy = col().gap(2.0).grow(1.0).child(
        text(label.to_owned())
            .font(k.ty.body_medium)
            .color(k.c.text_base),
    );
    if let Some(description) = description {
        copy = copy.child(
            text(description.to_owned())
                .wrap()
                .w_full()
                .font(k.ty.small)
                .color(k.c.text_muted),
        );
    }
    row()
        .key(key)
        .w_full()
        .min_h(56.0)
        .px(16.0)
        .py(10.0)
        .gap(16.0)
        .center_y()
        .justify_between()
        .child(copy)
        .child(control)
}

fn toggle_row(
    k: &Kit,
    key: &'static str,
    label: &str,
    description: Option<&str>,
    on: bool,
    flip: fn(&mut Ui),
) -> El {
    let switch = Switch::new(on)
        .label(label.to_owned())
        .hide_label()
        .on_toggle(flip)
        .build(k)
        .key(format!("{key}-switch"));
    setting_row(k, key, label, description, switch)
}

fn recording_section(s: &Ui, k: &Kit, layer: &mut Vec<El>) -> El {
    let settings = &s.settings;
    let quality = SegmentedControl::new(
        &QUALITIES.map(|(_, label)| label),
        QUALITIES.iter().position(|(q, _)| *q == settings.quality),
    )
    .label("Quality")
    .fit()
    .on_pick(|ui: &mut Ui, i: usize, _: &mut rune::Tx| ui.change(|s| s.quality = QUALITIES[i].0))
    .build(k)
    .key("quality");
    let displays = connected_displays();
    let options = resolution_options(&displays, settings.resolution);
    let labels: Vec<&str> = options.iter().map(|o| o.label.as_str()).collect();
    let current = options
        .iter()
        .find(|o| o.resolution == settings.resolution)
        .map(|o| o.label.as_str());
    let resolution = Select::new("resolution", &labels)
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
        .build(k, layer);
    let fps = SegmentedControl::new(
        &FPS_OPTIONS.map(|f| f.to_string()),
        FPS_OPTIONS.iter().position(|f| *f == settings.fps),
    )
    .label("Frame rate")
    .fit()
    .on_pick(|ui: &mut Ui, i: usize, _: &mut rune::Tx| ui.change(|s| s.fps = FPS_OPTIONS[i]))
    .build(k)
    .key("fps");
    section(
        k,
        "recording-card",
        "Recording",
        vec![
            toggle_row(
                k,
                "auto-record",
                "Record games",
                Some("Start recording when a League game starts."),
                settings.auto_record,
                |ui| ui.change(|s| s.auto_record = !s.auto_record),
            ),
            toggle_row(
                k,
                "full-game",
                "Record the full game",
                Some("Off keeps only highlight clips."),
                settings.record_full_game,
                |ui| ui.change(|s| s.record_full_game = !s.record_full_game),
            ),
            setting_row(
                k,
                "quality-row",
                "Quality",
                Some("Higher quality uses more disk space."),
                quality,
            ),
            setting_row(
                k,
                "resolution-row",
                "Resolution",
                Some("Larger games are scaled down to this."),
                resolution,
            ),
            setting_row(
                k,
                "fps-row",
                "Frame rate",
                Some("60 is smoother, 30 uses less disk space."),
                fps,
            ),
            size_estimate(s, k, native_height(&displays)),
            toggle_row(
                k,
                "audio",
                "Record game audio",
                Some("Records League's own audio. Discord and your microphone aren't recorded."),
                settings.audio,
                |ui| ui.change(|s| s.audio = !s.audio),
            ),
            toggle_row(
                k,
                "capture-cursor",
                "Show cursor",
                Some("Include the mouse cursor, as the game draws it."),
                settings.capture_cursor,
                |ui| ui.change(|s| s.capture_cursor = !s.capture_cursor),
            ),
        ],
    )
}

fn size_estimate(s: &Ui, k: &Kit, display_height: Option<u32>) -> El {
    let native_height = display_height.unwrap_or(FALLBACK_NATIVE_HEIGHT);
    let native = (s.settings.resolution == Resolution::Native).then_some(native_height);
    let line = format_size_estimate(s.settings.estimated_mb_per_minute(native_height), native);
    note_row(k, "size-estimate", &format!("{line}."))
}

fn native_height(displays: &[(u32, u32)]) -> Option<u32> {
    displays.iter().map(|&(_, h)| h).filter(|&h| h > 0).max()
}

#[cfg(windows)]
fn connected_displays() -> Vec<(u32, u32)> {
    greyscreen_capture::displays::connected_sizes()
}

#[cfg(not(windows))]
fn connected_displays() -> Vec<(u32, u32)> {
    Vec::new()
}

fn secs_input(
    k: &Kit,
    key: &'static str,
    label: &str,
    edit: &rune::TextEdit,
    pick: for<'a> fn(&'a mut Ui) -> &'a mut rune::TextEdit,
) -> El {
    let invalid = parse_clip_secs(&edit.text).is_none();
    TextInput::new(edit)
        .label(label.to_owned())
        .width(Some(88.0))
        .invalid(invalid)
        .on_edit(pick)
        .on_change(|ui: &mut Ui, _: &mut rune::Tx| ui.edit_clip_secs())
        .build(k)
        .key(key)
}

fn clips_section(s: &Ui, k: &Kit) -> El {
    let clips = &s.settings.clips;
    let pre = secs_input(k, "pre-secs", "Seconds before", &s.pre_secs, |ui| {
        &mut ui.pre_secs
    });
    let post = secs_input(k, "post-secs", "Seconds after", &s.post_secs, |ui| {
        &mut ui.post_secs
    });
    let mut rows = vec![
        toggle_row(
            k,
            "clips-enabled",
            "Save highlight clips",
            Some("Cut short clips around the moments you pick below."),
            clips.enabled,
            |ui| ui.change(|s| s.clips.enabled = !s.clips.enabled),
        ),
        setting_row(
            k,
            "pre-row",
            "Before the moment",
            Some("1 to 120 seconds."),
            pre,
        ),
        setting_row(
            k,
            "post-row",
            "After the moment",
            Some("1 to 120 seconds."),
            post,
        ),
        toggle_row(k, "clip-kills", "Kills", None, clips.kills, |ui| {
            ui.change(|s| s.clips.kills = !s.clips.kills)
        }),
        toggle_row(k, "clip-deaths", "Deaths", None, clips.deaths, |ui| {
            ui.change(|s| s.clips.deaths = !s.clips.deaths)
        }),
        toggle_row(k, "clip-assists", "Assists", None, clips.assists, |ui| {
            ui.change(|s| s.clips.assists = !s.clips.assists)
        }),
        toggle_row(
            k,
            "clip-multikills",
            "Multikills",
            None,
            clips.multikills,
            |ui| ui.change(|s| s.clips.multikills = !s.clips.multikills),
        ),
        toggle_row(
            k,
            "clip-objectives",
            "Objectives",
            Some("Dragon, Herald, Baron and Voidgrubs."),
            clips.objectives,
            |ui| ui.change(|s| s.clips.objectives = !s.clips.objectives),
        ),
        toggle_row(
            k,
            "clip-structures",
            "Structures",
            Some("Turrets and inhibitors."),
            clips.structures,
            |ui| ui.change(|s| s.clips.structures = !s.clips.structures),
        ),
        toggle_row(k, "clip-aces", "Aces", None, clips.aces, |ui| {
            ui.change(|s| s.clips.aces = !s.clips.aces)
        }),
        hotkey_row(s, k),
    ];
    if let Some(line) = hotkey_note(s) {
        rows.push(note_row(k, "hotkey-note", line));
    }
    section(k, "clips-card", "Clips", rows)
}

fn hotkey_note(s: &Ui) -> Option<&str> {
    if s.hotkey_capture {
        return s.hotkey_hint;
    }
    s.hotkey_error.as_deref()
}

fn hotkey_row(s: &Ui, k: &Kit) -> El {
    let label = if s.hotkey_capture {
        PRESS_A_KEY.to_owned()
    } else {
        s.settings
            .save_clip_hotkey
            .as_ref()
            .map_or_else(|| "Not set".to_owned(), Hotkey::to_string)
    };
    let pick = Button::new(label)
        .variant(ButtonVariant::Neutral)
        .control_size(ControlSize::Large)
        .on_press_tx(|ui: &mut Ui, tx: &mut rune::Tx| ui.toggle_hotkey_capture(tx))
        .build(k)
        .key("hotkey-button");
    let mut control = row().gap(8.0).center_y().child(pick);
    if s.settings.save_clip_hotkey.is_some() && !s.hotkey_capture {
        control = control.child(
            Button::new("Turn off")
                .variant(ButtonVariant::Ghost)
                .control_size(ControlSize::Large)
                .on_press(|ui: &mut Ui| ui.clear_hotkey())
                .build(k)
                .key("hotkey-off"),
        );
    }
    let description = if s.hotkey_capture {
        "Press the key or combination to use. Esc cancels."
    } else {
        "Saves the last 30 seconds while you play. Works while League is in front."
    };
    setting_row(
        k,
        "hotkey-row",
        "Save clip hotkey",
        Some(description),
        control,
    )
}

fn note_row(k: &Kit, key: &str, line: &str) -> El {
    row()
        .key(key.to_owned())
        .w_full()
        .px(16.0)
        .py(8.0)
        .gap(6.0)
        .center_y()
        .fill(k.c.layer_01)
        .child(rune::icon(&INFO, 14.0).color(k.c.icon_muted).a11y_hidden())
        .child(text(line.to_owned()).font(k.ty.small).color(k.c.text_muted))
}

fn storage_limit_label(gb: Option<u32>) -> String {
    gb.map_or_else(|| STORAGE_OFF.to_owned(), |gb| format!("{gb} GB"))
}

fn storage_limit_select(s: &Ui, k: &Kit, layer: &mut Vec<El>) -> El {
    let labels: Vec<String> = std::iter::once(None)
        .chain(STORAGE_LIMITS_GB.map(Some))
        .map(storage_limit_label)
        .collect();
    let names: Vec<&str> = labels.iter().map(String::as_str).collect();
    let current = storage_limit_label(s.settings.storage_limit_gb);
    Select::new("storage-limit", &names)
        .current(Some(current.as_str()))
        .open(s.storage_open)
        .appearance(SelectAppearance::Field)
        .width(STORAGE_WIDTH)
        .label("Storage limit")
        .on_toggle(|ui: &mut Ui| ui.storage_open = !ui.storage_open)
        .on_close(|ui: &mut Ui| ui.storage_open = false)
        .on_choose(|ui: &mut Ui, value: &str, _: &mut rune::Tx| {
            ui.storage_open = false;
            let limit = STORAGE_LIMITS_GB
                .into_iter()
                .find(|gb| storage_limit_label(Some(*gb)) == value);
            ui.change(|s| s.storage_limit_gb = limit);
        })
        .build(k, layer)
}

fn storage_section(s: &Ui, k: &Kit, layer: &mut Vec<El>) -> El {
    let change = Button::new("Change")
        .variant(ButtonVariant::Neutral)
        .control_size(ControlSize::Large)
        .on_press_tx(|ui: &mut Ui, tx: &mut rune::Tx| {
            let options = FileDialogOptions {
                title: "Choose the recordings folder".into(),
                directory: Some(ui.settings.output_dir.clone()),
                ..Default::default()
            };
            tx.open_folder(options, |ui: &mut Ui, answer, _| {
                if let Ok(Some(dir)) = answer {
                    ui.change(|s| s.output_dir = dir);
                }
            });
        })
        .build(k)
        .key("change-folder");
    let path = col().w(CONTROL_WIDTH).child(
        text(s.settings.output_dir.display().to_string())
            .key("output-dir")
            .font(k.ty.small)
            .color(k.c.text_muted)
            .truncate(),
    );
    let open = Button::new("Open")
        .variant(ButtonVariant::Ghost)
        .control_size(ControlSize::Large)
        .on_press(|ui: &mut Ui| ui.reveal_output_dir())
        .build(k)
        .key("open-output");
    let control = row()
        .gap(12.0)
        .center_y()
        .child(path)
        .child(change)
        .child(open);
    section(
        k,
        "storage-card",
        "Storage",
        vec![
            setting_row(
                k,
                "folder-row",
                "Recordings folder",
                Some("Games and clips are saved here."),
                control,
            ),
            setting_row(
                k,
                "storage-limit-row",
                "Storage limit",
                Some(STORAGE_LIMIT_NOTE),
                storage_limit_select(s, k, layer),
            ),
        ],
    )
}

fn about_section(s: &Ui, k: &Kit) -> El {
    let version = text(format!("Version {}", env!("CARGO_PKG_VERSION")))
        .key("version")
        .font(k.ty.body)
        .color(k.c.text_muted);
    let logs = Button::new("Open logs folder")
        .variant(ButtonVariant::Neutral)
        .control_size(ControlSize::Large)
        .on_press(|_: &mut Ui| reveal(&paths::logs_dir()))
        .build(k)
        .key("open-logs");
    let mut rows = vec![setting_row(k, "version-row", "Greyscreen", None, version)];
    rows.extend(updates::rows(s, k));
    rows.push(setting_row(
        k,
        "logs-row",
        "Logs",
        Some("Attach these when you report a problem."),
        logs,
    ));
    if cfg!(windows) {
        rows.push(toggle_row(
            k,
            "start-with-windows",
            "Start with Windows",
            Some("Open in the tray when you sign in."),
            s.start_with_windows,
            Ui::toggle_start_with_windows,
        ));
    }
    rows.push(
        row().key("riot-notice").w_full().px(16.0).py(10.0).child(
            text(RIOT_NOTICE)
                .wrap()
                .w_full()
                .font(k.ty.small)
                .color(k.c.text_muted),
        ),
    );
    section(k, "about-card", "About", rows)
}
