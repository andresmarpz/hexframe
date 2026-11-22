use crossbeam_channel::Receiver;
use greyscreen_core::export::{ExportDone, ExportEvent, ExportPreset, ExportRange};
use greyscreen_engine::EngineCmd;
use rune::Runtime;

use super::{live_runtime, nav_rect_of, settle, tap, visible_texts};
use crate::ui::fake_player;
use crate::ui::{ExportPhase, Ui, review_entry};

fn reviewing() -> (Runtime<Ui>, Receiver<EngineCmd>) {
    let (mut rt, rx) = live_runtime();
    rt.update(|ui, _| {
        ui.player_factory = fake_player::factory;
        ui.recordings = vec![review_entry()];
        ui.open_review_entry(review_entry());
    });
    settle(&mut rt);
    (rt, rx)
}

fn has_text(rt: &Runtime<Ui>, wanted: &str) -> bool {
    visible_texts(rt).iter().any(|text| text == wanted)
}

fn export_commands(rx: &Receiver<EngineCmd>) -> Vec<EngineCmd> {
    rx.try_iter()
        .filter(|cmd| matches!(cmd, EngineCmd::Export(_) | EngineCmd::CancelExport))
        .collect()
}

fn drag_by(rt: &mut Runtime<Ui>, key: &str, dx: f32) {
    let (x, y) = rt.find(key).unwrap_or_else(|| panic!("no element {key}"));
    rt.pointer_down(x, y);
    rt.pointer_move(x + dx / 2.0, y);
    rt.pointer_move(x + dx, y);
    rt.pointer_up(x + dx, y);
    settle(rt);
}

#[test]
fn a_clip_row_opens_the_dialog_and_cancel_closes_it() {
    let (mut rt, rx) = reviewing();
    tap(&mut rt, "tab-clips");
    settle(&mut rt);
    tap(&mut rt, "export-clip-1");
    settle(&mut rt);
    for key in [
        "export-dialog",
        "export-range",
        "export-preset",
        "export-destination",
        "export-start",
    ] {
        assert!(rt.find(key).is_some(), "{key} is missing");
    }
    assert!(has_text(&rt, "Export clip"));
    assert!(has_text(&rt, "Original quality"));
    tap(&mut rt, "export-cancel");
    settle(&mut rt);
    assert!(rt.find("export-dialog").is_none());
    assert!(export_commands(&rx).is_empty());
}

#[test]
fn dragging_a_handle_changes_the_range_that_is_exported() {
    let (mut rt, rx) = reviewing();
    tap(&mut rt, "tab-clips");
    tap(&mut rt, "export-clip-1");
    settle(&mut rt);
    let before = rt.state.export.range;
    drag_by(&mut rt, "export-end-handle", 60.0);
    let after = rt.state.export.range;
    assert_eq!(after.start, before.start);
    assert!(after.end > before.end, "{after:?} should end later");
    tap(&mut rt, "export-start");
    settle(&mut rt);
    let sent = export_commands(&rx);
    let [EngineCmd::Export(job)] = sent.as_slice() else {
        panic!("expected one export command");
    };
    assert_eq!(job.range, after);
    assert!(has_text(&rt, "Exporting…"));
    assert!(rt.find("export-progress").is_some());
}

#[test]
fn the_discord_preset_changes_the_job() {
    let (mut rt, rx) = reviewing();
    rt.update(|ui, _| ui.open_export_clip(&ui.recordings[0].meta.id.clone(), 1));
    rt.update(|ui, _| ui.export.can_reencode = true);
    settle(&mut rt);
    let (x, y) = rt.find("export-preset").expect("preset control");
    rt.pointer_down(x + 100.0, y);
    rt.pointer_up(x + 100.0, y);
    settle(&mut rt);
    assert_eq!(rt.state.export.preset, ExportPreset::Discord);
    tap(&mut rt, "export-start");
    let sent = export_commands(&rx);
    let [EngineCmd::Export(job)] = sent.as_slice() else {
        panic!("expected one export command");
    };
    assert_eq!(job.preset, ExportPreset::Discord);
    assert!(job.stem.ends_with("_Discord"));
}

#[test]
fn working_shows_progress_and_cancel_asks_the_engine() {
    let (mut rt, rx) = reviewing();
    rt.update(|ui, _| {
        ui.open_export_clip(&ui.recordings[0].meta.id.clone(), 1);
        ui.start_export();
        ui.apply_export(ExportEvent::Progress(0.4));
    });
    settle(&mut rt);
    export_commands(&rx);
    assert!(has_text(&rt, "Exporting…"));
    assert!(rt.find("export-start").is_none());
    let dialog_height = nav_rect_of(&rt, "export-dialog")[3];
    assert!(
        dialog_height >= 220.0,
        "export-dialog height collapsed to {dialog_height}"
    );
    tap(&mut rt, "export-cancel");
    settle(&mut rt);
    assert_eq!(export_commands(&rx), vec![EngineCmd::CancelExport]);
    assert!(has_text(&rt, "Cancelling…"));
    rt.update(|ui, _| ui.apply_export(ExportEvent::Cancelled));
    settle(&mut rt);
    assert!(rt.find("export-dialog").is_none());
}

#[test]
fn a_finished_export_offers_show_in_folder_copy_and_done() {
    let (mut rt, _rx) = reviewing();
    rt.update(|ui, _| {
        ui.open_export_clip(&ui.recordings[0].meta.id.clone(), 1);
        ui.start_export();
        ui.apply_export(ExportEvent::Done(ExportDone {
            path: "exports/Ahri_Test.mp4".into(),
            bytes: 8_700_000,
            range: ExportRange::new(0.4, 6.0),
            preset: ExportPreset::Original,
            shrunk: false,
        }));
    });
    settle(&mut rt);
    assert!(has_text(&rt, "Exported"));
    assert!(has_text(&rt, "Ahri_Test.mp4"));
    assert!(has_text(&rt, "8.7 MB · 00:06"));
    assert!(has_text(
        &rt,
        "It starts 0.6 s early, at the nearest keyframe, so no re-encoding was needed."
    ));
    let before = nav_rect_of(&rt, "export-copy")[2];
    tap(&mut rt, "export-copy");
    settle(&mut rt);
    assert!(has_text(&rt, "Copied"));
    assert_eq!(nav_rect_of(&rt, "export-copy")[2], before);
    assert!(has_text(&rt, "Copied. Paste it into Discord or a folder."));
    tap(&mut rt, "export-reveal");
    tap(&mut rt, "export-close");
    settle(&mut rt);
    assert!(rt.find("export-dialog").is_none());
}

#[test]
fn a_failed_export_says_why_and_can_try_again() {
    let (mut rt, _rx) = reviewing();
    rt.update(|ui, _| {
        ui.open_export_clip(&ui.recordings[0].meta.id.clone(), 1);
        ui.start_export();
        ui.apply_export(ExportEvent::Failed(
            "Couldn't make the smaller file. Try Original quality.".into(),
        ));
    });
    settle(&mut rt);
    assert!(has_text(&rt, "Export failed"));
    assert!(has_text(
        &rt,
        "Couldn't make the smaller file. Try Original quality."
    ));
    tap(&mut rt, "export-again");
    settle(&mut rt);
    assert!(rt.find("export-start").is_some());
    assert!(matches!(rt.state.export.phase, ExportPhase::Editing));
}

#[test]
fn the_review_toolbar_exports_the_moment_but_not_while_a_clip_plays() {
    let (mut rt, _rx) = reviewing();
    assert!(rt.find("review-export").is_some());
    tap(&mut rt, "review-export");
    settle(&mut rt);
    assert!(has_text(&rt, "Export this moment"));
    tap(&mut rt, "export-cancel");
    settle(&mut rt);
    rt.update(|ui, _| ui.play_clip(0));
    settle(&mut rt);
    assert!(rt.find("review-export").is_none());
}

#[test]
fn the_clips_screen_cards_have_an_export_button() {
    let (mut rt, _rx) = live_runtime();
    rt.update(|ui, _| {
        ui.recordings = vec![review_entry()];
        ui.go(crate::ui::Screen::Clips);
    });
    settle(&mut rt);
    let id = rt.state.recordings[0].meta.id.clone();
    tap(&mut rt, &format!("export-clip-{id}-1"));
    settle(&mut rt);
    assert!(rt.find("export-dialog").is_some());
    assert_eq!(rt.state.export.range, ExportRange::new(1.0, 6.0));
    assert!(
        rt.state.review.is_none(),
        "the card button does not open the clip"
    );
}
