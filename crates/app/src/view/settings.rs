use greyscreen_core::paths;
use greyscreen_core::settings::{Quality, Resolution};
use greyscreen_engine::reveal;
use rune::{El, FileDialogOptions, ScrollSpec, col, row, text};
use rune_kit::{
    Button, ButtonVariant, ControlSize, Kit, SegmentedControl, Select, Switch, TextInput,
};

use super::{card, hairline, page_header};
use crate::ui::{Ui, parse_clip_secs};

const QUALITIES: [(Quality, &str); 3] = [
    (Quality::Balanced, "Balanced"),
    (Quality::High, "High"),
    (Quality::Max, "Max"),
];
const RESOLUTIONS: [(Resolution, &str); 4] = [
    (Resolution::Native, "Native"),
    (Resolution::P1440, "1440p"),
    (Resolution::P1080, "1080p"),
    (Resolution::P720, "720p"),
];
const FPS_OPTIONS: [u32; 2] = [30, 60];
const CONTROL_WIDTH: f32 = 220.0;

pub(super) fn page(s: &Ui, k: &Kit, layer: &mut Vec<El>) -> El {
    col()
        .key("settings")
        .w_full()
        .h_full()
        .min_h(0.0)
        .gap(16.0)
        .child(page_header(
            k,
            "Settings",
            "Changes are saved automatically.",
            None,
        ))
        .child(
            col()
                .key("settings-scroll")
                .w_full()
                .grow(1.0)
                .min_h(0.0)
                .scroll(ScrollSpec {
                    stick_bottom: false,
                })
                .child(
                    col()
                        .w_full()
                        .gap(20.0)
                        .child(recording_section(s, k, layer))
                        .child(clips_section(s, k))
                        .child(storage_section(s, k))
                        .child(about_section(k)),
                ),
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
    let mut copy = col().gap(2.0).child(
        text(label.to_owned())
            .font(k.ty.body_medium)
            .color(k.c.text_base),
    );
    if let Some(description) = description {
        copy = copy.child(
            text(description.to_owned())
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
    let current = RESOLUTIONS
        .iter()
        .find(|(r, _)| *r == settings.resolution)
        .map(|(_, label)| *label);
    let resolution = Select::new("resolution", &RESOLUTIONS.map(|(_, label)| label))
        .current(current)
        .open(s.resolution_open)
        .width(140.0)
        .label("Resolution")
        .on_toggle(|ui: &mut Ui| ui.resolution_open = !ui.resolution_open)
        .on_close(|ui: &mut Ui| ui.resolution_open = false)
        .on_choose(|ui: &mut Ui, value: &str, _: &mut rune::Tx| {
            ui.resolution_open = false;
            if let Some((resolution, _)) = RESOLUTIONS.iter().find(|(_, label)| *label == value) {
                ui.change(|s| s.resolution = *resolution);
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
                "Record automatically",
                Some("Start when a League game starts, stop when it ends."),
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
            toggle_row(
                k,
                "audio",
                "Record game audio",
                None,
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
    section(
        k,
        "clips-card",
        "Clips",
        vec![
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
        ],
    )
}

fn storage_section(s: &Ui, k: &Kit) -> El {
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
        vec![setting_row(
            k,
            "folder-row",
            "Recordings folder",
            Some("Games and clips are saved here."),
            control,
        )],
    )
}

fn about_section(k: &Kit) -> El {
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
    section(
        k,
        "about-card",
        "About",
        vec![
            setting_row(k, "version-row", "Greyscreen", None, version),
            setting_row(
                k,
                "logs-row",
                "Logs",
                Some("Attach these when you report a problem."),
                logs,
            ),
        ],
    )
}
