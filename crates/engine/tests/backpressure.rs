#![allow(clippy::expect_used)]

mod common;

use std::fs;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

use greyscreen_core::keybinds::Action;
use greyscreen_core::settings::{ClipSettings, Settings};
use greyscreen_engine::{EngineCmd, FakeMediaSource, NoticeLevel, UiMsg, set_write_delay};

use common::{
    BlockingFreeSpace, Harness, WAIT, recording_since, start_engine_with_space, start_idle,
    temp_dir, wait_for,
};

static WRITE_DELAY: Mutex<()> = Mutex::new(());

const DISK_TOO_SLOW: &str = "Disk too slow";

fn serial() -> MutexGuard<'static, ()> {
    WRITE_DELAY.lock().unwrap_or_else(PoisonError::into_inner)
}

fn settings(tag: &str) -> Settings {
    Settings {
        output_dir: temp_dir(tag),
        ..Settings::default()
    }
}

fn send(harness: &Harness, cmd: EngineCmd) {
    harness
        .engine
        .as_ref()
        .expect("engine")
        .cmd
        .send(cmd)
        .expect("send");
}

fn error_texts(ui: &crossbeam_channel::Receiver<UiMsg>) -> Vec<String> {
    let mut errors = Vec::new();
    while let Ok(msg) = ui.recv_timeout(Duration::from_millis(300)) {
        if let UiMsg::Notice {
            level: NoticeLevel::Error,
            text,
        } = msg
        {
            errors.push(text);
        }
    }
    errors
}

#[test]
fn a_disk_that_stalls_ends_the_game_video_once_and_never_blocks_capture() {
    let _serial = serial();
    set_write_delay(Some(Duration::from_secs(5)));
    let media = FakeMediaSource::new().with_speed(40);
    let probe = media.probe();
    let harness = start_idle(settings("slow-disk"), Arc::new(media));
    send(&harness, EngineCmd::StartTestRecording { secs: 600 });
    recording_since(&harness.ui);

    let mut errors = Vec::new();
    let heard = wait_for(&harness.ui, |msg| match msg {
        UiMsg::Notice {
            level: NoticeLevel::Error,
            text,
        } => {
            errors.push(text.clone());
            text.contains(DISK_TOO_SLOW).then_some(())
        }
        _ => None,
    });
    set_write_delay(None);
    assert!(heard.is_some(), "no disk too slow notice");
    assert!(probe.max_send_wait() < Duration::from_secs(1));

    send(&harness, EngineCmd::StopRecording);
    let saved = wait_for(&harness.ui, |msg| match msg {
        UiMsg::Notice {
            level: NoticeLevel::Error,
            text,
        } => {
            errors.push(text.clone());
            None
        }
        UiMsg::RecordingSaved(entry) => Some(entry.clone()),
        _ => None,
    })
    .expect("recording saved");
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert_eq!(saved.meta.video, "game.mp4");
    assert!(saved.folder.join("game.mp4").is_file());
    assert!(!saved.folder.join("game.mp4.part").exists());
}

#[test]
fn a_control_thread_stuck_in_a_slow_drive_does_not_stall_capture() {
    let _serial = serial();
    let space = BlockingFreeSpace::new(Duration::from_secs(12));
    let media = FakeMediaSource::new();
    let probe = media.probe();
    let harness = start_engine_with_space(
        settings("slow-drive"),
        Arc::new(media),
        common::no_league(),
        None,
        Some(space.clone()),
    );
    send(&harness, EngineCmd::StartTestRecording { secs: 600 });
    recording_since(&harness.ui);
    space.arm();
    thread::sleep(Duration::from_secs(14));
    assert!(
        probe.max_send_wait() < Duration::from_secs(1),
        "capture waited {:?}",
        probe.max_send_wait()
    );
}

#[test]
fn an_encoder_failure_racing_a_stop_gives_one_recording_and_at_most_one_failure() {
    let _serial = serial();
    const FAIL_AFTER: Duration = Duration::from_millis(300);
    let mut failure_notices = 0;
    for stop_ms in (270..=330_u64).step_by(6) {
        let media = FakeMediaSource::new().failing_after(FAIL_AFTER, "encoder died");
        let harness = start_idle(settings("fail-stop-race"), Arc::new(media));
        let began = Instant::now();
        send(&harness, EngineCmd::StartTestRecording { secs: 600 });
        recording_since(&harness.ui);
        let stop_at = Duration::from_millis(stop_ms);
        thread::sleep(stop_at.saturating_sub(began.elapsed()));
        send(&harness, EngineCmd::StopRecording);
        let mut saved = 0;
        let mut errors = Vec::new();
        while let Ok(msg) = harness.ui.recv_timeout(Duration::from_secs(1)) {
            match msg {
                UiMsg::RecordingSaved(_) => saved += 1,
                UiMsg::Notice {
                    level: NoticeLevel::Error,
                    text,
                } => errors.push(text),
                _ => {}
            }
        }
        assert_eq!(saved, 1, "stop at {stop_ms} ms");
        assert!(errors.len() <= 1, "stop at {stop_ms} ms: {errors:?}");
        failure_notices += errors.len();
    }
    assert!(failure_notices > 0, "the encoder failure never arrived");
}

#[test]
fn a_clip_to_an_unwritable_folder_warns_and_leaves_no_clip_behind() {
    let _serial = serial();
    let harness = start_idle(
        settings("clip-unwritable"),
        Arc::new(FakeMediaSource::new()),
    );
    send(&harness, EngineCmd::StartTestRecording { secs: 600 });
    recording_since(&harness.ui);
    let folder = fs::read_dir(&harness.dir)
        .expect("output dir")
        .flatten()
        .map(|entry| entry.path())
        .find(|path| path.is_dir())
        .expect("recording folder");
    fs::write(folder.join("clips"), b"not a folder").expect("block clips");
    thread::sleep(Duration::from_millis(600));
    send(&harness, EngineCmd::SaveClipNow);
    let warned = wait_for(&harness.ui, |msg| match msg {
        UiMsg::Notice {
            level: NoticeLevel::Warn,
            text,
        } => text.contains("could not be saved").then_some(()),
        UiMsg::ClipSaved(_) => panic!("a clip was announced"),
        _ => None,
    });
    assert!(warned.is_some(), "no warning in {WAIT:?}");

    send(&harness, EngineCmd::StopRecording);
    let saved = wait_for(&harness.ui, |msg| match msg {
        UiMsg::ClipSaved(_) => panic!("a clip was announced"),
        UiMsg::RecordingSaved(entry) => Some(entry.clone()),
        _ => None,
    })
    .expect("recording saved");
    assert!(saved.meta.clips.is_empty());
    assert!(folder.join("clips").is_file());
    assert!(error_texts(&harness.ui).is_empty());
}

fn clip_settings(tag: &str) -> Settings {
    Settings {
        clips: ClipSettings {
            enabled: true,
            pre_secs: 1,
            post_secs: 1,
            ..ClipSettings::default()
        },
        ..settings(tag)
    }
}

#[test]
fn clips_still_save_while_the_game_writer_is_stalled() {
    let _serial = serial();
    set_write_delay(Some(Duration::from_secs(60)));
    let harness = start_idle(
        clip_settings("clip-stalled"),
        Arc::new(FakeMediaSource::new().with_speed(4)),
    );
    send(&harness, EngineCmd::StartTestRecording { secs: 600 });
    recording_since(&harness.ui);
    thread::sleep(Duration::from_millis(1500));
    send(&harness, EngineCmd::SaveClipNow);
    let clip = wait_for(&harness.ui, |msg| match msg {
        UiMsg::ClipSaved(clip) => Some(clip.clone()),
        _ => None,
    });
    set_write_delay(None);
    let clip = clip.expect("no clip while the writer was stalled");
    assert!(clip.folder.join(&clip.clip.file).is_file());
}

fn clips_after(harness: &Harness, trigger: EngineCmd) -> usize {
    recording_since(&harness.ui);
    thread::sleep(Duration::from_millis(1500));
    send(harness, trigger);
    let first = wait_for(&harness.ui, |msg| {
        matches!(msg, UiMsg::ClipSaved(_)).then_some(())
    });
    assert!(first.is_some(), "no clip");
    send(harness, EngineCmd::StopRecording);
    let mut clips = 1;
    while let Ok(msg) = harness.ui.recv_timeout(WAIT) {
        match msg {
            UiMsg::ClipSaved(_) => clips += 1,
            UiMsg::RecordingSaved(entry) => {
                assert_eq!(entry.meta.clips.len(), clips);
                return clips;
            }
            _ => {}
        }
    }
    panic!("recording never saved");
}

#[test]
fn the_save_now_command_and_the_hotkey_each_give_exactly_one_clip() {
    let _serial = serial();
    for trigger in [
        EngineCmd::SaveClipNow,
        EngineCmd::GlobalAction(Action::SaveClip),
    ] {
        let harness = start_idle(
            clip_settings("clip-once"),
            Arc::new(FakeMediaSource::new().with_speed(4)),
        );
        send(&harness, EngineCmd::StartTestRecording { secs: 600 });
        let name = format!("{trigger:?}");
        assert_eq!(clips_after(&harness, trigger), 1, "{name}");
    }
}

#[test]
fn a_stalled_writer_never_grows_the_media_queue_past_its_cap() {
    let _serial = serial();
    const ARRIVALS: u64 = 10_000;
    const MEDIA_QUEUE_CAP: u64 = 1024;
    const WRITER_QUEUE_CAP: u64 = 4096;
    set_write_delay(Some(Duration::from_secs(120)));
    let media = FakeMediaSource::fast();
    let probe = media.probe();
    let harness = start_idle(
        Settings {
            recording_diagnostics: true,
            ..settings("queue-cap")
        },
        Arc::new(media),
    );
    send(&harness, EngineCmd::StartTestRecording { secs: 600 });
    recording_since(&harness.ui);
    let began = Instant::now();
    while probe.packets_sent() < ARRIVALS && began.elapsed() < WAIT {
        thread::sleep(Duration::from_millis(20));
    }
    set_write_delay(None);
    assert!(probe.packets_sent() >= ARRIVALS, "{}", probe.packets_sent());
    send(&harness, EngineCmd::StopRecording);
    let mut too_slow = 0;
    let saved = wait_for(&harness.ui, |msg| match msg {
        UiMsg::Notice {
            level: NoticeLevel::Error,
            text,
        } => {
            too_slow += usize::from(text.contains(DISK_TOO_SLOW));
            None
        }
        UiMsg::RecordingSaved(entry) => Some(entry.clone()),
        _ => None,
    })
    .expect("recording saved");
    assert_eq!(too_slow, 1);

    let lines = fs::read_to_string(saved.folder.join("diagnostics.jsonl")).expect("diagnostics");
    let samples: Vec<(u64, u64)> = lines
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .filter(|value| value["type"] == "second")
        .map(|value| {
            (
                value["queue_media"].as_u64().expect("queue_media"),
                value["queue_writer"].as_u64().expect("queue_writer"),
            )
        })
        .collect();
    assert!(samples.len() > 10, "{} samples", samples.len());
    assert!(samples.iter().all(|(media, _)| *media <= MEDIA_QUEUE_CAP));
    assert!(
        samples
            .iter()
            .all(|(_, writer)| *writer <= WRITER_QUEUE_CAP)
    );
    let tail = &samples[samples.len() - 5..];
    assert!(
        tail.iter().all(|(_, writer)| *writer == 0),
        "writer queue still counted after Full: {tail:?}"
    );
}
