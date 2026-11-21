#![allow(clippy::expect_used)]

mod common;

use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crossbeam_channel::{Receiver, Sender, unbounded};
use greyscreen_core::game::{GameEvent, GameResult, TimedEvent};
use greyscreen_core::library::{self, RecordingMeta};
use greyscreen_core::settings::{ClipSettings, Settings};
use greyscreen_core::time::{Hns, secs_to_hns};
use greyscreen_engine::{
    EngineCmd, EngineStatus, FakeMediaSource, LeagueFactory, NoticeLevel, UiMsg,
};
use greyscreen_league::LeagueMsg;

use common::{
    Harness, WAIT, assert_playable_from_key_frame, info, kill, recording_since, start_engine,
    start_idle, temp_dir, wait_for,
};

fn clip_settings(tag: &str, enabled: bool) -> Settings {
    Settings {
        output_dir: temp_dir(tag),
        clips: ClipSettings {
            enabled,
            pre_secs: 1,
            post_secs: 1,
            ..ClipSettings::default()
        },
        ..Settings::default()
    }
}

fn dragon() -> GameEvent {
    GameEvent::DragonKill {
        dragon_type: "Fire".to_owned(),
        stolen: false,
        killer: "Hero".to_owned(),
        assisters: Vec::new(),
    }
}

fn game(settings: Settings, offsets: Vec<(f64, GameEvent)>, finish: LeagueMsg) -> Harness {
    let (go_tx, go_rx) = unbounded::<Hns>();
    let league: LeagueFactory = Box::new(move |out: Sender<LeagueMsg>| {
        let (go, offsets, finish) = (go_rx.clone(), offsets.clone(), finish.clone());
        Box::new(thread::spawn(move || {
            out.send(LeagueMsg::WindowFound { hwnd: 7, pid: 42 }).ok();
            out.send(LeagueMsg::Ready(info())).ok();
            let Ok(t0) = go.recv_timeout(WAIT) else {
                return;
            };
            for (index, (secs, kind)) in offsets.into_iter().enumerate() {
                out.send(LeagueMsg::Event(TimedEvent {
                    id: index as u32 + 1,
                    game_secs: secs,
                    at: t0 + secs_to_hns(secs),
                    kind,
                }))
                .ok();
            }
            out.send(finish).ok();
        }))
    });
    let tap = Box::new(move |msg: &UiMsg| {
        if let UiMsg::Status(EngineStatus::Recording { since, .. }) = msg {
            go_tx.send(*since).ok();
        }
    });
    start_engine(
        settings,
        Arc::new(FakeMediaSource::new()),
        league,
        Some(tap),
    )
}

fn collect_until_saved(ui: &Receiver<UiMsg>) -> Vec<UiMsg> {
    let mut seen = Vec::new();
    while let Ok(msg) = ui.recv_timeout(WAIT) {
        let saved = matches!(msg, UiMsg::RecordingSaved(_));
        seen.push(msg);
        if saved {
            break;
        }
    }
    seen
}

fn saved_meta(msgs: &[UiMsg]) -> RecordingMeta {
    let folder = msgs
        .iter()
        .find_map(|m| match m {
            UiMsg::RecordingSaved(entry) => Some(entry.folder.clone()),
            _ => None,
        })
        .expect("recording saved");
    library::read_meta(&folder).expect("sidecar")
}

#[test]
fn highlights_become_merged_clips_before_recording_saved() {
    let events = vec![
        (1.0, kill("Hero", "Villain")),
        (3.0, kill("Villain", "Hero")),
        (3.5, dragon()),
    ];
    let harness = game(
        clip_settings("clips", true),
        events,
        LeagueMsg::Ended(GameResult::Win),
    );
    let msgs = collect_until_saved(&harness.ui);
    for msg in &msgs {
        if let UiMsg::Notice { level, text } = msg {
            assert_ne!(*level, NoticeLevel::Error, "{text}");
            assert_ne!(*level, NoticeLevel::Warn, "{text}");
        }
    }
    let clip_count = msgs
        .iter()
        .filter(|m| matches!(m, UiMsg::ClipSaved(_)))
        .count();
    assert_eq!(clip_count, 2);

    let Some(UiMsg::RecordingSaved(entry)) = msgs.last() else {
        panic!("last message is not RecordingSaved");
    };
    assert!(entry.size_bytes > 0);
    let meta = saved_meta(&msgs);
    assert_eq!(meta.clips.len(), 2);
    assert_eq!(meta.clips[0].label, "MyKill");
    assert_eq!(meta.clips[1].label, "MyDeath+Dragon");
    assert!(meta.clips[0].start_secs.abs() < 1e-9);
    assert!((meta.clips[1].start_secs - 2.0).abs() < 0.05);
    assert!((meta.clips[1].end_secs - 4.5).abs() < 0.05);
    for clip in &meta.clips {
        let path = entry.folder.join(&clip.file);
        assert!(path.is_file(), "{}", path.display());
        assert_playable_from_key_frame(&path);
    }
    let files = std::fs::read_dir(entry.folder.join("clips"))
        .expect("clips dir")
        .count();
    assert_eq!(files, 2);
}

#[test]
fn clips_disabled_writes_nothing() {
    let harness = game(
        clip_settings("noclips", false),
        vec![
            (0.2, kill("Hero", "Villain")),
            (0.4, kill("Villain", "Hero")),
        ],
        LeagueMsg::Gone,
    );
    let msgs = collect_until_saved(&harness.ui);
    assert!(!msgs.iter().any(|m| matches!(m, UiMsg::ClipSaved(_))));
    let meta = saved_meta(&msgs);
    assert!(meta.clips.is_empty());
    assert_eq!(meta.events.len(), 2);
    let Some(UiMsg::RecordingSaved(entry)) = msgs.last() else {
        panic!("last message is not RecordingSaved");
    };
    let clips = entry.folder.join("clips");
    assert!(!clips.exists() || std::fs::read_dir(clips).expect("clips dir").count() == 0);
}

#[test]
fn test_recording_makes_a_manual_folder() {
    let harness = start_idle(
        clip_settings("test-rec", true),
        Arc::new(FakeMediaSource::fast()),
    );
    let cmd = harness.engine.as_ref().expect("engine").cmd.clone();
    cmd.send(EngineCmd::StartTestRecording { secs: 1 }).ok();
    let entry = wait_for(&harness.ui, |msg| match msg {
        UiMsg::RecordingSaved(entry) => Some(entry.clone()),
        _ => None,
    })
    .expect("recording saved");
    let name = entry
        .folder
        .file_name()
        .expect("name")
        .to_string_lossy()
        .into_owned();
    assert!(name.ends_with("_Test_MANUAL"), "{name}");
    assert!(entry.folder.join("game.mp4").is_file());
    assert!(
        entry.meta.duration_secs >= 0.5,
        "{}",
        entry.meta.duration_secs
    );
    assert!(entry.size_bytes > 0);
}

#[test]
fn save_clip_now_cuts_a_manual_clip() {
    let harness = start_idle(
        clip_settings("manual", true),
        Arc::new(FakeMediaSource::new()),
    );
    let cmd = harness.engine.as_ref().expect("engine").cmd.clone();
    cmd.send(EngineCmd::StartTestRecording { secs: 3 }).ok();
    recording_since(&harness.ui);
    thread::sleep(Duration::from_millis(1500));
    cmd.send(EngineCmd::SaveClipNow).ok();
    let msgs = collect_until_saved(&harness.ui);
    let clip = msgs
        .iter()
        .find_map(|m| match m {
            UiMsg::ClipSaved(clip) => Some(clip.clone()),
            _ => None,
        })
        .expect("clip saved");
    assert_eq!(clip.clip.label, "Manual");
    assert!(clip.clip.start_secs.abs() < 1e-9);
    assert!(clip.clip.end_secs > 1.0 && clip.clip.end_secs < 3.0);
    assert_playable_from_key_frame(&clip.folder.join(&clip.clip.file));
    assert_eq!(saved_meta(&msgs).clips.len(), 1);
}
