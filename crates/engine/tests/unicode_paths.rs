#![allow(clippy::expect_used)]

mod common;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use greyscreen_core::export::{ExportEvent, ExportJob, ExportPreset, ExportRange};
use greyscreen_core::library::VideoInfo;
use greyscreen_core::settings::Settings;
#[cfg(windows)]
use greyscreen_engine::NoticeLevel;
use greyscreen_engine::updater::UpdateConfig;
use greyscreen_engine::{EngineCmd, FakeMediaSource, UiMsg};

use common::{
    assert_playable_from_key_frame, leftover_part, library_message, media, recording_since, send,
    start_idle, temp_dir, wait_for,
};

fn unicode_output(root: &Path) -> PathBuf {
    root.join("Zoe_é").join("ビデオ").join("Greyscreen")
}

#[test]
fn recording_clipping_recovering_exporting_and_deleting_work_in_a_unicode_folder() {
    let root = temp_dir("unicode");
    let output = unicode_output(&root);
    let sample = media(240);
    let crashed = leftover_part(&output, "2025-01-01_00-00-00_Ahri_CLASSIC", &sample, false);
    assert!(output.is_absolute());

    let harness = start_idle(
        Settings {
            output_dir: output.clone(),
            ..Settings::default()
        },
        Arc::new(FakeMediaSource::new()),
    );
    let recovered = library_message(&harness);
    assert_eq!(recovered.len(), 1);
    assert!(recovered[0].meta.recovered);
    assert!(crashed.join("game.mp4").is_file());
    assert_playable_from_key_frame(&crashed.join("game.mp4"));

    send(&harness, EngineCmd::StartTestRecording { secs: 3 });
    recording_since(&harness.ui);
    thread::sleep(Duration::from_millis(1500));
    send(&harness, EngineCmd::SaveClipNow);
    let mut clip = None;
    let saved = wait_for(&harness.ui, |msg| match msg {
        UiMsg::ClipSaved(entry) => {
            clip = Some(entry.clone());
            None
        }
        UiMsg::RecordingSaved(entry) => Some(entry.clone()),
        _ => None,
    })
    .expect("recording saved");
    let clip = clip.expect("clip saved");
    assert!(clip.folder.join(&clip.clip.file).is_file());
    let video = saved.folder.join(&saved.meta.video);
    assert!(video.is_file());
    assert_playable_from_key_frame(&video);

    send(
        &harness,
        EngineCmd::Export(ExportJob {
            source: video,
            source_offset_secs: 0.0,
            range: ExportRange::new(0.0, 1.0),
            preset: ExportPreset::Original,
            video: VideoInfo {
                width: saved.meta.video_format.width,
                height: saved.meta.video_format.height,
                fps: saved.meta.video_format.fps,
            },
            dest_dir: output.join("exports"),
            stem: "Zoé_ビデオ".into(),
        }),
    );
    let done = wait_for(&harness.ui, |msg| match msg {
        UiMsg::Export(ExportEvent::Done(done)) => Some(done.clone()),
        UiMsg::Export(ExportEvent::Failed(why)) => panic!("export failed: {why}"),
        _ => None,
    })
    .expect("export done");
    assert!(done.path.is_file());
    assert!(done.bytes > 0);

    #[cfg(windows)]
    {
        let id = saved
            .folder
            .file_name()
            .expect("folder name")
            .to_string_lossy()
            .into_owned();
        send(&harness, EngineCmd::DeleteRecording { id });
        let after = wait_for(&harness.ui, |msg| match msg {
            UiMsg::Notice {
                level: NoticeLevel::Error,
                text,
            } => panic!("delete failed: {text}"),
            UiMsg::Library(entries) => Some(entries.clone()),
            _ => None,
        })
        .expect("library after delete");
        assert!(after.iter().all(|entry| entry.folder != saved.folder));
        let gone_by = std::time::Instant::now() + std::time::Duration::from_secs(2);
        while saved.folder.exists() && std::time::Instant::now() < gone_by {
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        assert!(!saved.folder.exists());
    }

    drop(harness);
    std::fs::remove_dir_all(&root).expect("remove temp root");
}

#[test]
fn the_update_download_folder_is_absolute() {
    assert!(UpdateConfig::github("0.0.0", true).dir.is_absolute());
}
