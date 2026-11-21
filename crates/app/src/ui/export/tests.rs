use crossbeam_channel::{Receiver, unbounded};
use greyscreen_core::export::{ExportDone, ExportEvent, ExportPreset, ExportRange};
use greyscreen_core::settings::Settings;
use greyscreen_engine::{EngineCmd, RecordingEntry};

use crate::ui::{CopyState, Destination, ExportPhase, RangeHandle, Source, Ui};
use crate::ui::{fake_player, review_entry};

fn live() -> (Ui, Receiver<EngineCmd>) {
    let (cmd, rx) = unbounded();
    let mut ui = Ui::live(Settings::default(), cmd);
    ui.player_factory = fake_player::factory;
    (ui, rx)
}

fn reviewing(entry: RecordingEntry) -> (Ui, Receiver<EngineCmd>) {
    let (mut ui, rx) = live();
    ui.open_review_entry(entry);
    (ui, rx)
}

fn id_of(ui: &Ui) -> String {
    ui.review
        .as_ref()
        .map(|review| review.entry.meta.id.clone())
        .unwrap_or_default()
}

fn exports(rx: &Receiver<EngineCmd>) -> Vec<EngineCmd> {
    rx.try_iter()
        .filter(|cmd| matches!(cmd, EngineCmd::Export(_) | EngineCmd::CancelExport))
        .collect()
}

fn done_event(bytes: u64) -> ExportEvent {
    ExportEvent::Done(ExportDone {
        path: "exports/Ahri.mp4".into(),
        bytes,
        range: ExportRange::new(0.5, 6.0),
        preset: ExportPreset::Original,
        shrunk: false,
    })
}

#[test]
fn a_clip_opens_with_its_own_range_and_padded_bounds() {
    let (mut ui, _rx) = reviewing(review_entry());
    let id = id_of(&ui);
    ui.open_export_clip(&id, 1);
    let export = &ui.export;
    assert!(export.open && export.editing());
    assert_eq!(export.range, ExportRange::new(1.0, 6.0));
    assert_eq!(export.default_range, export.range);
    assert_eq!(export.bounds, (0.0, 10.0));
    let target = export.target.as_ref().unwrap();
    assert_eq!(target.title, "Export clip");
    assert!(target.subtitle.contains("Kill"));
    assert!(target.source.ends_with("review.mp4"));
    assert_eq!(target.source_offset_secs, 0.0);
}

#[test]
fn a_clip_opens_from_the_clips_screen_without_a_review() {
    let (mut ui, _rx) = live();
    ui.recordings = vec![review_entry()];
    let id = ui.recordings[0].meta.id.clone();
    assert!(ui.review.is_none());
    ui.open_export_clip(&id, 0);
    assert!(ui.export.open);
    assert_eq!(ui.export.range, ExportRange::new(0.0, 10.0));
}

#[test]
fn a_deleted_game_video_exports_from_the_clip_file() {
    let mut entry = review_entry();
    entry.meta.video = String::new();
    let (mut ui, _rx) = live();
    ui.recordings = vec![entry];
    let id = ui.recordings[0].meta.id.clone();
    ui.open_export_clip(&id, 1);
    let export = &ui.export;
    let target = export.target.as_ref().unwrap();
    assert!(target.source.ends_with("review.mp4"));
    assert_eq!(target.source_offset_secs, 1.0);
    assert_eq!(export.bounds, (1.0, 6.0));
}

#[test]
fn nothing_opens_when_no_video_is_left() {
    let dir = tempfile::tempdir().unwrap();
    let mut entry = review_entry();
    entry.folder = dir.path().to_path_buf();
    let (mut ui, _rx) = live();
    ui.recordings = vec![entry];
    let id = ui.recordings[0].meta.id.clone();
    ui.open_export_clip(&id, 0);
    assert!(!ui.export.open);
}

#[test]
fn a_moment_is_fifteen_seconds_each_side_of_the_playhead() {
    let mut entry = review_entry();
    entry.meta.duration_secs = 600.0;
    let (mut ui, _rx) = reviewing(entry);
    fake_player::set_time(100.0);
    ui.open_export_moment();
    let export = &ui.export;
    assert_eq!(export.range, ExportRange::new(85.0, 115.0));
    assert_eq!(export.bounds, (55.0, 145.0));
    assert_eq!(export.target.as_ref().unwrap().title, "Export this moment");
    fake_player::set_time(0.0);
}

#[test]
fn a_moment_needs_the_full_game_on_screen() {
    let (mut ui, _rx) = reviewing(review_entry());
    ui.play_clip(0);
    assert_eq!(ui.review.as_ref().map(|r| r.source), Some(Source::Clip(0)));
    ui.open_export_moment();
    assert!(!ui.export.open);
}

#[test]
fn the_handles_move_in_whole_seconds_and_stay_a_second_apart() {
    let mut entry = review_entry();
    entry.meta.duration_secs = 600.0;
    let (mut ui, _rx) = reviewing(entry);
    ui.recordings = vec![ui.review.as_ref().unwrap().entry.clone()];
    ui.export.begin(
        super::ExportTarget {
            title: String::new(),
            subtitle: String::new(),
            champion: "Ahri".into(),
            game_folder: "g".into(),
            source: "g/game.mp4".into(),
            source_offset_secs: 0.0,
            video: ui.recordings[0].meta.video_format,
        },
        ExportRange::new(100.0, 120.0),
        (70.0, 150.0),
    );
    ui.drag_export_handle(RangeHandle::Start, 0.5);
    assert_eq!(ui.export.range, ExportRange::new(110.0, 120.0));
    ui.drag_export_handle(RangeHandle::End, 0.0);
    assert_eq!(ui.export.range, ExportRange::new(110.0, 111.0));
    ui.drag_export_handle(RangeHandle::Start, 1.0);
    assert_eq!(ui.export.range, ExportRange::new(110.0, 111.0));
    ui.drag_export_handle(RangeHandle::End, 1.0);
    assert_eq!(ui.export.range, ExportRange::new(110.0, 150.0));
    ui.drag_export_handle(RangeHandle::Start, 0.0);
    assert_eq!(ui.export.range, ExportRange::new(70.0, 150.0));
}

#[test]
fn the_handle_fractions_follow_the_range() {
    let (mut ui, _rx) = reviewing(review_entry());
    let id = id_of(&ui);
    ui.open_export_clip(&id, 1);
    assert_eq!(ui.export.range_fractions(), (0.1, 0.6));
    ui.drag_export_handle(RangeHandle::End, 0.9);
    assert_eq!(ui.export.range_fractions(), (0.1, 0.9));
    assert_eq!(ui.export.default_fractions(), (0.1, 0.6));
}

#[test]
fn discord_needs_a_platform_that_can_re_encode() {
    let (mut ui, _rx) = reviewing(review_entry());
    let id = id_of(&ui);
    ui.open_export_clip(&id, 1);
    ui.export.can_reencode = false;
    ui.set_export_preset(1);
    assert_eq!(ui.export.preset, ExportPreset::Original);
    ui.export.can_reencode = true;
    ui.set_export_preset(1);
    assert_eq!(ui.export.preset, ExportPreset::Discord);
    ui.set_export_preset(0);
    assert_eq!(ui.export.preset, ExportPreset::Original);
    ui.set_export_preset(9);
    assert_eq!(ui.export.preset, ExportPreset::Original);
}

#[test]
fn downloads_needs_a_downloads_folder() {
    let (mut ui, _rx) = reviewing(review_entry());
    let id = id_of(&ui);
    ui.open_export_clip(&id, 1);
    ui.export.downloads = None;
    ui.set_export_destination(1);
    assert_eq!(ui.export.destination, Destination::Game);
    ui.export.downloads = Some("/dl".into());
    ui.set_export_destination(1);
    assert_eq!(ui.export.destination, Destination::Downloads);
    assert_eq!(ui.export.destination_dir(), Some("/dl".into()));
    ui.set_export_destination(0);
    let folder = ui.review.as_ref().unwrap().entry.folder.clone();
    assert_eq!(ui.export.destination_dir(), Some(folder.join("exports")));
}

#[test]
fn export_sends_the_job_and_waits() {
    let (mut ui, rx) = reviewing(review_entry());
    let id = id_of(&ui);
    ui.open_export_clip(&id, 1);
    ui.start_export();
    let sent = exports(&rx);
    let [EngineCmd::Export(job)] = sent.as_slice() else {
        panic!("expected one export command");
    };
    let entry = &ui.review.as_ref().unwrap().entry;
    assert_eq!(job.range, ExportRange::new(1.0, 6.0));
    assert_eq!(job.preset, ExportPreset::Original);
    assert_eq!(job.dest_dir, entry.folder.join("exports"));
    assert_eq!(job.stem, "Ahri_00-01_00-06_Original");
    assert_eq!(job.video, entry.meta.video_format);
    assert_eq!(job.source_offset_secs, 0.0);
    assert!(ui.export.working());
    ui.start_export();
    assert!(exports(&rx).is_empty());
}

#[test]
fn a_clip_too_long_for_discord_cannot_start() {
    let (mut ui, rx) = reviewing(review_entry());
    let id = id_of(&ui);
    ui.open_export_clip(&id, 1);
    ui.export.can_reencode = true;
    ui.export.preset = ExportPreset::Discord;
    ui.export.range = ExportRange::new(0.0, 400.0);
    assert!(ui.export.plan_problem().is_some_and(|p| p.contains("258")));
    assert!(!ui.export.can_start());
    ui.start_export();
    assert!(exports(&rx).is_empty());
    ui.export.preset = ExportPreset::Original;
    assert!(ui.export.can_start());
}

#[test]
fn progress_then_done_keeps_the_wanted_start() {
    let (mut ui, _rx) = reviewing(review_entry());
    let id = id_of(&ui);
    ui.open_export_clip(&id, 1);
    ui.start_export();
    ui.apply_export(ExportEvent::Progress(0.4));
    assert!(matches!(
        ui.export.phase,
        ExportPhase::Working { fraction, .. } if (fraction - 0.4).abs() < 1e-6
    ));
    ui.apply_export(ExportEvent::Shrinking);
    assert!(matches!(
        ui.export.phase,
        ExportPhase::Working { fraction, shrinking: true, .. } if fraction == 0.0
    ));
    ui.apply_export(done_event(7_000_000));
    let ExportPhase::Done {
        done,
        wanted_start,
        copy,
    } = &ui.export.phase
    else {
        panic!("not done");
    };
    assert_eq!(done.bytes, 7_000_000);
    assert_eq!(*wanted_start, 1.0);
    assert_eq!(*copy, CopyState::Idle);
}

#[test]
fn a_failure_can_be_retried_and_late_events_are_ignored() {
    let (mut ui, _rx) = reviewing(review_entry());
    let id = id_of(&ui);
    ui.open_export_clip(&id, 1);
    ui.apply_export(ExportEvent::Failed("nope".into()));
    assert!(
        ui.export.editing(),
        "an event with no export running is ignored"
    );
    ui.start_export();
    ui.apply_export(ExportEvent::Failed(
        "Couldn't cut that part of the video.".into(),
    ));
    assert!(matches!(&ui.export.phase, ExportPhase::Failed(text) if text.contains("cut")));
    ui.apply_export(ExportEvent::Progress(0.9));
    assert!(matches!(ui.export.phase, ExportPhase::Failed(_)));
    ui.export_again();
    assert!(ui.export.editing());
    assert_eq!(ui.export.range, ExportRange::new(1.0, 6.0));
}

#[test]
fn cancelling_asks_the_engine_once_and_closes_when_it_confirms() {
    let (mut ui, rx) = reviewing(review_entry());
    let id = id_of(&ui);
    ui.open_export_clip(&id, 1);
    ui.start_export();
    exports(&rx);
    ui.close_export();
    ui.cancel_export();
    assert_eq!(exports(&rx), vec![EngineCmd::CancelExport]);
    assert!(matches!(
        ui.export.phase,
        ExportPhase::Working {
            cancelling: true,
            ..
        }
    ));
    assert!(ui.export.open);
    ui.apply_export(ExportEvent::Cancelled);
    assert!(!ui.export.open && ui.export.editing());
}

#[test]
fn closing_again_while_cancelling_does_not_trap_the_user() {
    let (mut ui, rx) = reviewing(review_entry());
    let id = id_of(&ui);
    ui.open_export_clip(&id, 1);
    ui.start_export();
    exports(&rx);
    ui.close_export();
    assert!(ui.export.open);
    ui.close_export();
    assert!(!ui.export.open);
    ui.apply_export(done_event(1));
    assert!(ui.export.editing());
}

#[test]
fn a_new_export_cannot_replace_one_that_is_running() {
    let (mut ui, _rx) = reviewing(review_entry());
    let id = id_of(&ui);
    ui.open_export_clip(&id, 1);
    ui.start_export();
    ui.open_export_clip(&id, 0);
    assert_eq!(ui.export.range, ExportRange::new(1.0, 6.0));
    assert!(ui.export.working());
}

#[test]
fn done_closes_with_the_dialog() {
    let (mut ui, _rx) = reviewing(review_entry());
    let id = id_of(&ui);
    ui.open_export_clip(&id, 1);
    ui.start_export();
    ui.apply_export(done_event(1_000_000));
    ui.close_export();
    assert!(!ui.export.open && ui.export.editing());
}
