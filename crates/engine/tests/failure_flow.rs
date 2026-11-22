#![allow(clippy::expect_used)]

mod common;

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;
use std::time::Duration;

use crossbeam_channel::{Receiver, Sender, bounded, unbounded};
use greyscreen_core::library;
use greyscreen_core::media::{
    CaptureConfig, CaptureTarget, MediaEvent, MediaSession, MediaSource, Track,
};
use greyscreen_core::settings::Settings;
use greyscreen_engine::{
    EngineCmd, EngineStatus, FakeMediaSource, LeagueFactory, NoticeLevel, RecordingEntry, UiMsg,
};
use greyscreen_league::LeagueMsg;

use common::{
    Harness, WAIT, ffprobe, info, recording_since, start_engine, start_idle, temp_dir, wait_for,
};

const RAW_ERROR: &str = "driver said 0xC0DE <&> in module hx";

fn settings(tag: &str) -> Settings {
    Settings {
        output_dir: temp_dir(tag),
        ..Settings::default()
    }
}

fn start_test_recording(harness: &Harness, secs: u32) {
    harness
        .engine
        .as_ref()
        .expect("engine")
        .cmd
        .send(EngineCmd::StartTestRecording { secs })
        .expect("send");
}

fn folders_in(dir: &Path) -> usize {
    std::fs::read_dir(dir).map_or(0, |entries| entries.flatten().count())
}

fn saved(harness: &Harness) -> RecordingEntry {
    wait_for(&harness.ui, |msg| match msg {
        UiMsg::RecordingSaved(entry) => Some(entry.clone()),
        _ => None,
    })
    .expect("recording saved")
}

fn saved_with_warnings(harness: &Harness) -> (RecordingEntry, Vec<String>) {
    let mut warnings = Vec::new();
    let entry = wait_for(&harness.ui, |msg| match msg {
        UiMsg::Notice {
            level: NoticeLevel::Warn,
            text,
        } => {
            warnings.push(text.clone());
            None
        }
        UiMsg::RecordingSaved(entry) => Some(entry.clone()),
        _ => None,
    })
    .expect("recording saved");
    (entry, warnings)
}

fn last_packet_secs(file: &Path, kind: &str) -> Option<f64> {
    let packets = ffprobe(
        file,
        &[
            "-show_entries",
            "packet=codec_type,pts_time",
            "-of",
            "csv=p=0",
        ],
    )?;
    packets
        .lines()
        .filter_map(|line| {
            let mut fields = line.split(',');
            let codec = fields.next()?;
            let time = fields.next()?;
            (codec == kind).then(|| time.parse::<f64>().ok())?
        })
        .reduce(f64::max)
}

struct FailsFirstStart {
    inner: FakeMediaSource,
    failed: AtomicBool,
}

impl MediaSource for FailsFirstStart {
    fn start(
        &self,
        target: CaptureTarget,
        cfg: CaptureConfig,
        out: Sender<MediaEvent>,
    ) -> Result<Box<dyn MediaSession>, String> {
        if !self.failed.swap(true, Ordering::SeqCst) {
            return Err(RAW_ERROR.to_owned());
        }
        self.inner.start(target, cfg, out)
    }
}

struct Relay {
    inner: FakeMediaSource,
    map: fn(MediaEvent) -> Vec<MediaEvent>,
    targets: Arc<Mutex<Vec<CaptureTarget>>>,
}

impl MediaSource for Relay {
    fn start(
        &self,
        target: CaptureTarget,
        cfg: CaptureConfig,
        out: Sender<MediaEvent>,
    ) -> Result<Box<dyn MediaSession>, String> {
        self.targets
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(target);
        let (tx, rx) = bounded(64);
        let session = self.inner.start(target, cfg, tx)?;
        let map = self.map;
        thread::spawn(move || {
            while let Ok(event) = rx.recv() {
                for mapped in map(event) {
                    if out.send(mapped).is_err() {
                        return;
                    }
                }
            }
        });
        Ok(session)
    }
}

fn without_audio(event: MediaEvent) -> Vec<MediaEvent> {
    match event {
        MediaEvent::Started {
            t0, video, encoder, ..
        } => vec![
            MediaEvent::Started {
                t0,
                video,
                audio: None,
                encoder,
            },
            MediaEvent::Warning("Game audio could not be captured. Recording video only.".into()),
        ],
        MediaEvent::Packet(packet) if packet.track == Track::Audio => Vec::new(),
        other => vec![other],
    }
}

fn pass_through(event: MediaEvent) -> Vec<MediaEvent> {
    vec![event]
}

fn game_channel() -> (LeagueFactory, Receiver<Sender<LeagueMsg>>) {
    let (tx, rx) = unbounded();
    let factory: LeagueFactory = Box::new(move |out: Sender<LeagueMsg>| {
        tx.send(out).ok();
        Box::new(())
    });
    (factory, rx)
}

fn open_game(out: &Sender<LeagueMsg>, hwnd: isize, pid: u32) {
    out.send(LeagueMsg::WindowFound { hwnd, pid }).ok();
    out.send(LeagueMsg::Ready(info())).ok();
}

#[test]
fn a_failed_start_gives_an_error_notice_and_the_next_good_start_shows_none() {
    let media = FailsFirstStart {
        inner: FakeMediaSource::new(),
        failed: AtomicBool::new(false),
    };
    let harness = start_idle(settings("flow-start"), Arc::new(media));
    start_test_recording(&harness, 1);
    let mut first_notices = Vec::new();
    let status = wait_for(&harness.ui, |msg| match msg {
        UiMsg::Notice { level, text } => {
            first_notices.push((*level, text.clone()));
            None
        }
        UiMsg::Status(EngineStatus::Error(text)) => Some(text.clone()),
        _ => None,
    })
    .expect("error status");
    assert!(status.contains(RAW_ERROR), "{status}");
    assert!(
        first_notices
            .iter()
            .any(|(level, text)| *level == NoticeLevel::Error && text.contains(RAW_ERROR)),
        "{first_notices:?}"
    );

    start_test_recording(&harness, 1);
    let mut after = Vec::new();
    let mut recorded = false;
    wait_for(&harness.ui, |msg| {
        match msg {
            UiMsg::Status(EngineStatus::Recording { .. }) => recorded = true,
            UiMsg::Status(EngineStatus::Error(text)) => after.push(text.clone()),
            UiMsg::Notice {
                level: NoticeLevel::Error,
                text,
            } => after.push(text.clone()),
            _ => {}
        }
        matches!(msg, UiMsg::RecordingSaved(_)).then_some(())
    })
    .expect("second recording saved");
    assert!(recorded);
    assert_eq!(after, Vec::<String>::new());
}

#[test]
fn an_encoder_refusal_is_a_start_failure_with_the_raw_text_and_no_folder() {
    let reason = "the video encoder failed to open: 0x8007000e";
    let media = FakeMediaSource::new().failing_to_start(reason);
    let probe = media.probe();
    let harness = start_idle(settings("flow-refuse"), Arc::new(media));
    start_test_recording(&harness, 1);
    let mut notices = Vec::new();
    let status = wait_for(&harness.ui, |msg| match msg {
        UiMsg::Notice {
            level: NoticeLevel::Error,
            text,
        } => {
            notices.push(text.clone());
            None
        }
        UiMsg::Status(EngineStatus::Error(text)) => Some(text.clone()),
        _ => None,
    })
    .expect("error status");
    assert!(status.contains(reason), "{status}");
    assert_eq!(notices.len(), 1, "{notices:?}");
    assert_eq!(folders_in(&harness.dir), 0);
    assert_eq!(probe.live_sessions(), 0);
}

#[test]
fn a_window_that_closes_between_start_and_started_ends_idle() {
    let media = FakeMediaSource::new().blocking_start(Duration::from_millis(800));
    let probe = media.probe();
    let (league, game) = game_channel();
    let harness = start_engine(settings("flow-closed-early"), Arc::new(media), league, None);
    let out = game.recv_timeout(WAIT).expect("league sender");
    open_game(&out, 7, 42);
    thread::sleep(Duration::from_millis(200));
    out.send(LeagueMsg::Gone).ok();

    let mut last_status = None;
    while let Ok(msg) = harness.ui.recv_timeout(Duration::from_millis(2500)) {
        match msg {
            UiMsg::Status(EngineStatus::Recording { .. }) | UiMsg::RecordingSaved(_) => {
                panic!("a recording began after the window closed")
            }
            UiMsg::Status(status) => last_status = Some(status),
            _ => {}
        }
    }
    assert_eq!(last_status, Some(EngineStatus::Idle));
    assert_eq!(folders_in(&harness.dir), 0);
    assert_eq!(probe.live_sessions(), 0);
}

#[test]
fn a_closed_window_ends_video_and_audio_together_and_the_next_session_has_its_own_pid() {
    let targets = Arc::new(Mutex::new(Vec::new()));
    let media = Relay {
        inner: FakeMediaSource::new(),
        map: pass_through,
        targets: targets.clone(),
    };
    let (league, game) = game_channel();
    let harness = start_engine(settings("flow-pids"), Arc::new(media), league, None);
    let out = game.recv_timeout(WAIT).expect("league sender");

    open_game(&out, 7, 42);
    recording_since(&harness.ui);
    thread::sleep(Duration::from_millis(1500));
    out.send(LeagueMsg::Gone).ok();
    let first = saved(&harness);
    let file = first.folder.join(&first.meta.video);
    if let (Some(video), Some(audio)) = (
        last_packet_secs(&file, "video"),
        last_packet_secs(&file, "audio"),
    ) {
        assert!(video > 1.0, "{video}");
        assert!((video - audio).abs() < 0.25, "video {video}, audio {audio}");
    }
    let health = first.meta.health.expect("health");
    assert!(health.audio.expect("audio").packets > 0);

    open_game(&out, 8, 43);
    recording_since(&harness.ui);
    out.send(LeagueMsg::Gone).ok();
    let second = saved(&harness);
    assert_ne!(first.meta.id, second.meta.id);
    assert_eq!(
        *targets.lock().unwrap_or_else(PoisonError::into_inner),
        vec![
            CaptureTarget::GameWindow { hwnd: 7, pid: 42 },
            CaptureTarget::GameWindow { hwnd: 8, pid: 43 },
        ]
    );
}

#[test]
fn an_audio_start_failure_gives_a_video_only_file_and_a_warning() {
    let media = Relay {
        inner: FakeMediaSource::new(),
        map: without_audio,
        targets: Arc::default(),
    };
    let harness = start_idle(settings("flow-no-audio"), Arc::new(media));
    start_test_recording(&harness, 2);
    let (entry, warnings) = saved_with_warnings(&harness);
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert!(warnings[0].contains("audio"), "{warnings:?}");
    assert!(!entry.meta.video.is_empty());
    let file = entry.folder.join(&entry.meta.video);
    if let Some(streams) = ffprobe(&file, &["-show_entries", "stream=codec_name"]) {
        assert!(streams.contains("h264"), "{streams}");
        assert!(!streams.contains("aac"), "{streams}");
    }
    assert!(entry.meta.health.expect("health").audio.is_none());
}

#[test]
fn audio_that_stops_mid_session_leaves_the_video_complete_and_the_file_finalised() {
    let media = FakeMediaSource::new().stopping_audio_after(50, "The audio device stopped.");
    let harness = start_idle(settings("flow-audio-stops"), Arc::new(media));
    start_test_recording(&harness, 3);
    let (entry, warnings) = saved_with_warnings(&harness);
    assert_eq!(warnings, vec!["The audio device stopped."]);
    assert!(!entry.meta.video.is_empty());
    let file = entry.folder.join(&entry.meta.video);
    assert!(file.is_file());
    assert!(!entry.folder.join("game.mp4.part").exists());
    assert!(
        entry.meta.duration_secs > 2.0,
        "{}",
        entry.meta.duration_secs
    );
    if let (Some(video), Some(audio)) = (
        last_packet_secs(&file, "video"),
        last_packet_secs(&file, "audio"),
    ) {
        assert!(video > 2.0, "{video}");
        assert!(audio < video - 0.5, "video {video}, audio {audio}");
    }
    assert!(library::read_meta(&entry.folder).is_ok());
}
