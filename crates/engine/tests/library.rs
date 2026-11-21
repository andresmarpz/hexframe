#![allow(clippy::expect_used)]

mod common;

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crossbeam_channel::bounded;
use greyscreen_core::game::{GameResult, Team};
use greyscreen_core::library::{self, GameRecord, META_VERSION, RecordingMeta, VideoInfo};
use greyscreen_core::media::{
    AudioFormat, CaptureConfig, CaptureTarget, MediaEvent, MediaSource, Packet, Track, VideoFormat,
};
use greyscreen_core::settings::Settings;
use greyscreen_engine::{EngineCmd, EngineStatus, FakeMediaSource, RecordingEntry, UiMsg};
use greyscreen_mp4::fragmented::FragmentedWriter;
use jiff::Zoned;

use common::{assert_playable_from_key_frame, info, start_idle, temp_dir, wait_for};

struct Media {
    video: VideoFormat,
    audio: AudioFormat,
    packets: Vec<Packet>,
}

fn capture_config() -> CaptureConfig {
    Settings::default().capture_config(1080)
}

fn media(video_frames: usize) -> Media {
    let (tx, rx) = bounded(4096);
    let session = FakeMediaSource::fast()
        .start(CaptureTarget::PrimaryMonitor, capture_config(), tx)
        .expect("fake media");
    let mut formats = None;
    let mut packets = Vec::new();
    let mut video_seen = 0;
    while video_seen < video_frames {
        match rx.recv().expect("media event") {
            MediaEvent::Started { video, audio, .. } => formats = Some((video, audio)),
            MediaEvent::Packet(packet) => {
                video_seen += usize::from(packet.track == Track::Video);
                packets.push(packet);
            }
            _ => {}
        }
    }
    drop(rx);
    session.stop();
    let (video, audio) = formats.expect("started event");
    Media {
        video,
        audio: audio.expect("audio format"),
        packets,
    }
}

fn leftover_part(output: &Path, name: &str, media: &Media, finish: bool) -> PathBuf {
    let folder = output.join(name);
    fs::create_dir_all(&folder).expect("folder");
    let part = folder.join("game.mp4.part");
    let mut writer =
        FragmentedWriter::create(&part, &media.video, Some(&media.audio)).expect("writer");
    for packet in &media.packets {
        writer.push(packet).expect("push");
    }
    if finish {
        writer.finish().expect("finish");
    } else {
        drop(writer);
    }
    let meta = RecordingMeta {
        version: META_VERSION,
        id: name.to_owned(),
        started_at: Zoned::now(),
        duration_secs: 0.0,
        video: "game.mp4".to_owned(),
        game: GameRecord {
            info: info(),
            result: GameResult::Unknown,
        },
        events: Vec::new(),
        clips: Vec::new(),
        encoder: "fake".to_owned(),
        video_format: VideoInfo {
            width: media.video.width,
            height: media.video.height,
            fps: media.video.fps,
        },
    };
    library::write_meta(&folder, &meta).expect("sidecar");
    folder
}

fn library_message(harness: &common::Harness) -> Vec<RecordingEntry> {
    wait_for(&harness.ui, |msg| match msg {
        UiMsg::Library(entries) => Some(entries.clone()),
        _ => None,
    })
    .expect("library message")
}

#[test]
fn leftover_parts_are_recovered_or_renamed_before_the_first_scan() {
    let media = media(240);
    let output = temp_dir("recover");
    let crashed = leftover_part(&output, "2025-01-01_00-00-00_Ahri_CLASSIC", &media, false);
    let done = leftover_part(&output, "2025-01-02_00-00-00_Zed_CLASSIC", &media, true);
    let finished_bytes = fs::metadata(done.join("game.mp4.part"))
        .expect("part")
        .len();

    let harness = start_idle(
        Settings {
            output_dir: output,
            ..Settings::default()
        },
        Arc::new(FakeMediaSource::fast()),
    );
    let entries = library_message(&harness);
    assert_eq!(entries.len(), 2);
    for folder in [&crashed, &done] {
        assert!(folder.join("game.mp4").is_file(), "{}", folder.display());
        assert!(!folder.join("game.mp4.part").exists());
        assert_playable_from_key_frame(&folder.join("game.mp4"));
        let entry = entries.iter().find(|e| &e.folder == folder).expect("entry");
        assert!(entry.size_bytes > 0);
        assert!(
            entry.meta.duration_secs > 1.0,
            "{}",
            entry.meta.duration_secs
        );
    }
    assert_eq!(
        fs::metadata(done.join("game.mp4")).expect("video").len(),
        finished_bytes
    );
    assert_eq!(entries[0].meta.game.info.me.team, Team::Order);
}

#[test]
fn rescan_reports_new_recordings() {
    let output = temp_dir("rescan");
    let harness = start_idle(
        Settings {
            output_dir: output.clone(),
            ..Settings::default()
        },
        Arc::new(FakeMediaSource::fast()),
    );
    assert!(library_message(&harness).is_empty());
    let media = media(120);
    leftover_part(&output, "2025-01-03_00-00-00_Ahri_CLASSIC", &media, true);
    fs::rename(
        output.join("2025-01-03_00-00-00_Ahri_CLASSIC/game.mp4.part"),
        output.join("2025-01-03_00-00-00_Ahri_CLASSIC/game.mp4"),
    )
    .expect("rename");
    let cmd = harness.engine.as_ref().expect("engine").cmd.clone();
    cmd.send(EngineCmd::RescanLibrary).ok();
    let entries = library_message(&harness);
    assert_eq!(entries.len(), 1);
    assert!(entries[0].size_bytes > 0);
}

#[test]
fn applied_settings_are_saved_to_the_given_path() {
    let output = temp_dir("settings");
    let harness = start_idle(
        Settings {
            output_dir: output.clone(),
            ..Settings::default()
        },
        Arc::new(FakeMediaSource::fast()),
    );
    let path = harness.settings_path.clone();
    let changed = Settings {
        output_dir: output,
        fps: 30,
        audio: false,
        ..Settings::default()
    };
    let cmd = harness.engine.as_ref().expect("engine").cmd.clone();
    cmd.send(EngineCmd::ApplySettings(changed.clone())).ok();
    drop(cmd);
    let mut harness = harness;
    harness.engine.take();
    assert_eq!(Settings::load(&path), changed);
}

#[test]
fn recovery_sets_the_video_when_the_sidecar_has_a_duration_but_no_video() {
    let media = media(120);
    let output = temp_dir("recover-video");
    let folder = leftover_part(&output, "2025-01-04_00-00-00_Ahri_CLASSIC", &media, true);
    let mut meta = library::read_meta(&folder).expect("sidecar");
    meta.video = String::new();
    meta.duration_secs = 12.0;
    library::write_meta(&folder, &meta).expect("sidecar");

    let harness = start_idle(
        Settings {
            output_dir: output,
            ..Settings::default()
        },
        Arc::new(FakeMediaSource::fast()),
    );
    let entries = library_message(&harness);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].meta.video, "game.mp4");
    assert!((entries[0].meta.duration_secs - 12.0).abs() < 1e-9);
    assert!(folder.join("game.mp4").is_file());
}

#[test]
fn a_failed_final_rename_is_repaired_by_recovery() {
    let output = temp_dir("rename-fails");
    let settings = Settings {
        output_dir: output.clone(),
        ..Settings::default()
    };
    let blocked = output.clone();
    let tap: common::Tap = Box::new(move |msg| {
        if matches!(msg, UiMsg::Status(EngineStatus::Recording { .. }))
            && let Some(folder) = fs::read_dir(&blocked)
                .ok()
                .and_then(|mut dir| dir.next())
                .and_then(Result::ok)
        {
            fs::create_dir_all(folder.path().join("game.mp4/blocker")).ok();
        }
    });
    let mut harness = common::start_engine(
        settings.clone(),
        Arc::new(FakeMediaSource::new().with_speed(20)),
        common::no_league(),
        Some(tap),
    );
    let cmd = harness.engine.as_ref().expect("engine").cmd.clone();
    cmd.send(EngineCmd::StartTestRecording { secs: 1 }).ok();
    let saved = wait_for(&harness.ui, |msg| match msg {
        UiMsg::RecordingSaved(entry) => Some(entry.clone()),
        _ => None,
    })
    .expect("recording saved");
    assert_eq!(saved.meta.video, "");
    assert!(saved.folder.join("game.mp4.part").is_file());
    drop(cmd);
    harness.engine.take();

    fs::remove_dir_all(saved.folder.join("game.mp4")).expect("unblock");
    let second = start_idle(settings, Arc::new(FakeMediaSource::fast()));
    let entries = library_message(&second);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].meta.video, "game.mp4");
    assert!(saved.folder.join("game.mp4").is_file());
}
