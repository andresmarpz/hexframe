use greyscreen_core::settings::STORAGE_LIMITS_GB;
use rune::{El, FileDialogOptions, col, row, text};
use rune_kit::{Button, ButtonVariant, ControlSize, Kit, Progress, Select, SelectAppearance};

use super::catalog::Row;
use super::{setting_row, stacked_row};
use crate::ui::{Ui, format_size};

const STORAGE_WIDTH: f32 = 140.0;
const STORAGE_OFF: &str = "Off";
const STORAGE_LIMIT_NOTE: &str = "Oldest full-game videos are deleted permanently when recordings pass this size. Clips are kept as long as possible.";
const BYTES_PER_GB: f64 = 1_000_000_000.0;

pub(super) fn render(s: &Ui, k: &Kit, layer: &mut Vec<El>, which: Row) -> El {
    match which {
        Row::Folder => folder(s, k),
        _ => limit(s, k, layer),
    }
}

fn folder(s: &Ui, k: &Kit) -> El {
    let path = row().grow(1.0).min_w(0.0).child(
        text(s.settings.output_dir.display().to_string())
            .key("output-dir")
            .font(k.ty.small)
            .color(k.c.text_muted)
            .truncate(),
    );
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
    let open = Button::new("Open")
        .variant(ButtonVariant::Ghost)
        .control_size(ControlSize::Large)
        .on_press(|ui: &mut Ui| ui.reveal_output_dir())
        .build(k)
        .key("open-output");
    let control = row()
        .w_full()
        .gap(8.0)
        .center_y()
        .child(path)
        .child(change)
        .child(open);
    stacked_row(
        k,
        "folder-row",
        "Recordings folder",
        Some("Games and clips are saved here."),
        control,
    )
}

fn limit(s: &Ui, k: &Kit, layer: &mut Vec<El>) -> El {
    let used = s.storage_used_bytes();
    let (summary, fraction) = match s.settings.storage_limit_gb {
        Some(gb) => (
            format!("{} of {gb} GB", format_size(used)),
            (used as f64 / (f64::from(gb) * BYTES_PER_GB)).min(1.0) as f32,
        ),
        None => (format!("{} used, no limit", format_size(used)), 0.0),
    };
    let meter = col()
        .key("storage-meter")
        .w_full()
        .gap(6.0)
        .child(
            text(summary)
                .key("storage-summary")
                .font(k.ty.body_medium)
                .color(k.c.text_base),
        )
        .child(Progress::new(Some(fraction)).build(k));
    let select = setting_row(
        k,
        "storage-limit-row",
        "Storage limit",
        Some(STORAGE_LIMIT_NOTE),
        select(s, k, layer),
    );
    col()
        .key("storage-usage")
        .w_full()
        .child(row().w_full().px(16.0).pt(12.0).pb(4.0).child(meter))
        .child(select)
}

fn limit_label(gb: Option<u32>) -> String {
    gb.map_or_else(|| STORAGE_OFF.to_owned(), |gb| format!("{gb} GB"))
}

fn select(s: &Ui, k: &Kit, layer: &mut Vec<El>) -> El {
    let labels: Vec<String> = std::iter::once(None)
        .chain(STORAGE_LIMITS_GB.map(Some))
        .map(limit_label)
        .collect();
    let names: Vec<&str> = labels.iter().map(String::as_str).collect();
    let current = limit_label(s.settings.storage_limit_gb);
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
                .find(|gb| limit_label(Some(*gb)) == value);
            ui.change(|s| s.storage_limit_gb = limit);
        })
        .build(k, layer)
}
