mod range;

use greyscreen_core::export::{DISCORD_LIMIT_BYTES, ExportDone, ExportPreset};
use rune::{El, Tx, col, row, text};
use rune_kit::{
    Button, ButtonVariant, ControlSize, Dialog, Field, Kit, Progress, SegmentedControl, State,
};

use crate::ui::{CopyState, Destination, ExportPhase, ExportUi, Ui, format_clock};

const DIALOG_WIDTH: f32 = 480.0;
const PADDING: f32 = 16.0;
const NO_RE_ENCODE: &str = "Re-encoding needs Windows, so only Original quality works here.";

pub(crate) fn dialog(s: &Ui, k: &Kit, layer: &mut Vec<El>) {
    let export = &s.export;
    let Some(target) = export.target.as_ref().filter(|_| export.open) else {
        return;
    };
    let (title, subtitle, body) = match &export.phase {
        ExportPhase::Editing => (
            target.title.clone(),
            target.subtitle.clone(),
            form(export, k),
        ),
        ExportPhase::Working {
            fraction,
            shrinking,
            cancelling,
        } => (
            target.title.clone(),
            target.subtitle.clone(),
            working(*fraction, *shrinking, *cancelling, k),
        ),
        ExportPhase::Done {
            done,
            wanted_start,
            copy,
        } => (
            "Exported".to_owned(),
            file_name(done),
            finished(done, *wanted_start, copy, k),
        ),
        ExportPhase::Failed(why) => (
            "Export failed".to_owned(),
            target.subtitle.clone(),
            failed(why, k),
        ),
    };
    Dialog::new("export", title)
        .description(subtitle)
        .width(DIALOG_WIDTH)
        .fit()
        .open(true)
        .on_close(|ui: &mut Ui| ui.close_export())
        .child(body)
        .build(k, layer);
}

fn file_name(done: &ExportDone) -> String {
    done.path
        .file_name()
        .map_or_else(String::new, |name| name.to_string_lossy().into_owned())
}

fn body() -> El {
    col()
        .key("export-body")
        .w_full()
        .px(PADDING)
        .pb(PADDING)
        .gap(PADDING)
}

fn note(key: &'static str, message: String, k: &Kit) -> El {
    text(message)
        .key(key)
        .font(k.ty.body)
        .color(k.c.text_muted)
        .wrap()
        .w_full()
}

fn problem(key: &'static str, message: String, k: &Kit) -> El {
    text(message)
        .key(key)
        .font(k.ty.body)
        .color(k.c.danger.fg)
        .wrap()
        .w_full()
}

fn buttons(items: Vec<El>) -> El {
    row().w_full().gap(8.0).justify_end().children(items)
}

fn action(label: &str, variant: ButtonVariant) -> Button {
    Button::new(label)
        .variant(variant)
        .control_size(ControlSize::Large)
}

fn form(export: &ExportUi, k: &Kit) -> El {
    let presets: Vec<&str> = ExportPreset::ALL.iter().map(|p| p.label()).collect();
    let picked = ExportPreset::ALL.iter().position(|p| *p == export.preset);
    let mut quality = SegmentedControl::new(&presets, picked)
        .label("Quality")
        .width(None)
        .on_pick(|ui: &mut Ui, index, _: &mut Tx| ui.set_export_preset(index));
    if !export.can_reencode {
        quality = quality.segment_state(1, State::Disabled);
    }
    let places: Vec<&str> = Destination::ALL.iter().map(|d| d.label()).collect();
    let chosen = Destination::ALL
        .iter()
        .position(|d| *d == export.destination);
    let mut save_to = SegmentedControl::new(&places, chosen)
        .label("Save to")
        .width(None)
        .on_pick(|ui: &mut Ui, index, _: &mut Tx| ui.set_export_destination(index));
    if export.downloads.is_none() {
        save_to = save_to.segment_state(1, State::Disabled);
    }
    let can_start = export.can_start();
    body()
        .child(
            col()
                .w_full()
                .gap(4.0)
                .child(range::bar(export, k))
                .child(range::readout(export, k)),
        )
        .child(
            Field::new()
                .label("Quality")
                .control(quality.build(k).key("export-preset"))
                .build(k),
        )
        .child(note("export-note", export.preset.note().to_owned(), k))
        .children(
            (!export.can_reencode).then(|| note("export-windows-only", NO_RE_ENCODE.to_owned(), k)),
        )
        .child(
            Field::new()
                .label("Save to")
                .control(save_to.build(k).key("export-destination"))
                .build(k),
        )
        .children(
            export
                .plan_problem()
                .map(|why| problem("export-problem", why, k)),
        )
        .child(buttons(vec![
            action("Cancel", ButtonVariant::Neutral)
                .on_press(|ui: &mut Ui| ui.close_export())
                .build(k)
                .key("export-cancel"),
            action("Export", ButtonVariant::Contrast)
                .disabled(!can_start)
                .on_press(|ui: &mut Ui| ui.start_export())
                .build(k)
                .key("export-start"),
        ]))
}

fn working(fraction: f32, shrinking: bool, cancelling: bool, k: &Kit) -> El {
    let label = if cancelling {
        "Cancelling…"
    } else if shrinking {
        "Making it smaller…"
    } else {
        "Exporting…"
    };
    body()
        .child(
            Progress::new(Some(fraction))
                .label(label)
                .show_value()
                .build(k)
                .key("export-progress"),
        )
        .child(buttons(vec![
            action("Cancel", ButtonVariant::Neutral)
                .disabled(cancelling)
                .on_press(|ui: &mut Ui| ui.cancel_export())
                .build(k)
                .key("export-cancel"),
        ]))
}

pub(crate) fn size_label(bytes: u64) -> String {
    let mb = bytes as f64 / 1_000_000.0;
    if mb >= 1.0 {
        format!("{mb:.1} MB")
    } else {
        format!("{:.0} KB", (bytes as f64 / 1_000.0).max(1.0))
    }
}

fn summary(done: &ExportDone) -> String {
    let length = format_clock(done.range.duration().round() as u32);
    let size = size_label(done.bytes);
    if done.preset == ExportPreset::Discord && done.bytes <= DISCORD_LIMIT_BYTES {
        format!("{size} · {length} · under the 10 MB limit")
    } else {
        format!("{size} · {length}")
    }
}

fn early_note(done: &ExportDone, wanted_start: f64) -> Option<String> {
    let early = wanted_start - done.range.start;
    (done.preset == ExportPreset::Original && early >= 0.05).then(|| {
        format!(
            "It starts {early:.1} s early, at the nearest keyframe, so no re-encoding was needed."
        )
    })
}

fn finished(done: &ExportDone, wanted_start: f64, copy: &CopyState, k: &Kit) -> El {
    let copy_label = if *copy == CopyState::Copied {
        "Copied"
    } else {
        "Copy file"
    };
    let green = k.c.success.fg;
    let status = match copy {
        CopyState::Copied => Some(note(
            "export-copied",
            "Copied. Paste it into Discord or a folder.".into(),
            k,
        )),
        CopyState::Failed(why) => Some(problem(
            "export-copy-error",
            format!("Couldn't copy the file: {why}"),
            k,
        )),
        CopyState::Idle => None,
    };
    let mut lines = col().w_full().gap(4.0).child(
        text(summary(done))
            .key("export-summary")
            .font(k.ty.body_medium)
            .color(green),
    );
    if done.shrunk {
        lines = lines.child(note(
            "export-shrunk",
            "It was a little big at first, so it was made smaller.".into(),
            k,
        ));
    }
    if let Some(early) = early_note(done, wanted_start) {
        lines = lines.child(note("export-early", early, k));
    }
    body().child(lines).children(status).child(buttons(vec![
        action("Show in folder", ButtonVariant::Neutral)
            .on_press(|ui: &mut Ui| ui.export_reveal())
            .build(k)
            .key("export-reveal"),
        action(copy_label, ButtonVariant::Neutral)
            .on_press(|ui: &mut Ui| ui.export_copy())
            .build(k)
            .key("export-copy"),
        action("Done", ButtonVariant::Contrast)
            .on_press(|ui: &mut Ui| ui.close_export())
            .build(k)
            .key("export-close"),
    ]))
}

fn failed(why: &str, k: &Kit) -> El {
    body()
        .child(problem("export-error", why.to_owned(), k))
        .child(buttons(vec![
            action("Close", ButtonVariant::Neutral)
                .on_press(|ui: &mut Ui| ui.close_export())
                .build(k)
                .key("export-close"),
            action("Try again", ButtonVariant::Contrast)
                .on_press(|ui: &mut Ui| ui.export_again())
                .build(k)
                .key("export-again"),
        ]))
}
