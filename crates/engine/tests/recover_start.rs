#![allow(clippy::expect_used)]

mod common;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::time::{Duration, Instant};

use crossbeam_channel::Sender;

use greyscreen_core::library;
use greyscreen_core::settings::Settings;
use greyscreen_engine::{FakeMediaSource, NoticeAction, NoticeLevel, RecordingEntry, UiMsg};
use greyscreen_league::LeagueMsg;
use greyscreen_mp4::inspect::{inspect, sample_secs};

use common::{
    Harness, assert_playable_from_key_frame, info, leftover_part, library_message, media,
    start_engine, start_idle, temp_dir, wait_for,
};

const CHILD_OUTPUT: &str = "GREYSCREEN_KILL_TEST_OUTPUT";
const MIN_FRAGMENTS: u32 = 2;

const RECOVERED: &str = "Recovered 1 recording that was cut off.";

fn launch(output: &Path) -> Harness {
    start_idle(
        Settings {
            output_dir: output.to_path_buf(),
            ..Settings::default()
        },
        Arc::new(FakeMediaSource::fast()),
    )
}

fn truncate_into_a_fragment(part: &Path) {
    let len = fs::metadata(part).expect("part").len();
    fs::OpenOptions::new()
        .write(true)
        .open(part)
        .expect("open")
        .set_len(len - len / 7)
        .expect("truncate");
}

fn notices(harness: &Harness) -> Vec<UiMsg> {
    let mut found = Vec::new();
    while let Ok(msg) = harness.ui.recv_timeout(Duration::from_millis(400)) {
        if matches!(msg, UiMsg::Notice { .. } | UiMsg::NoticeWithAction { .. }) {
            found.push(msg);
        }
    }
    found
}

fn first_library(harness: &Harness) -> Vec<RecordingEntry> {
    library_message(harness)
}

#[test]
fn a_recording_cut_off_by_a_crash_comes_back_with_one_quiet_notice() {
    let media = media(480);
    let output = temp_dir("recover-truncated");
    let folder = leftover_part(&output, "2025-02-01_00-00-00_Ahri_CLASSIC", &media, false);
    truncate_into_a_fragment(&folder.join("game.mp4.part"));

    let harness = launch(&output);
    let entries = first_library(&harness);

    assert_eq!(entries.len(), 1);
    let entry = &entries[0];
    assert!(entry.meta.recovered);
    assert_eq!(entry.meta.video, "game.mp4");
    assert!(
        entry.meta.duration_secs > 1.0,
        "{}",
        entry.meta.duration_secs
    );
    assert!(entry.size_bytes > 0);
    assert!(!folder.join("game.mp4.part").exists());
    assert_playable_from_key_frame(&folder.join("game.mp4"));
    assert!(library::read_meta(&folder).expect("sidecar").recovered);

    let notice = wait_for(&harness.ui, |msg| match msg {
        UiMsg::Notice { level, text } => Some((*level, text.clone())),
        _ => None,
    })
    .expect("recovered notice");
    assert_eq!(notice, (NoticeLevel::Info, RECOVERED.to_owned()));
}

#[cfg(any(target_os = "macos", windows))]
#[test]
fn a_recovered_recording_gets_its_thumbnail() {
    let media = media(480);
    let output = temp_dir("recover-thumb");
    let folder = leftover_part(&output, "2025-02-02_00-00-00_Ahri_CLASSIC", &media, false);
    truncate_into_a_fragment(&folder.join("game.mp4.part"));

    let harness = launch(&output);

    let thumb = wait_for(&harness.ui, |msg| match msg {
        UiMsg::Thumbnail(path) => Some(path.clone()),
        _ => None,
    })
    .expect("thumbnail message");
    assert_eq!(thumb, folder.join("thumb.jpg"));
    assert!(thumb.is_file());
}

#[test]
fn a_second_launch_finds_nothing_to_recover() {
    let media = media(480);
    let output = temp_dir("recover-twice");
    let folder = leftover_part(&output, "2025-02-03_00-00-00_Ahri_CLASSIC", &media, false);
    truncate_into_a_fragment(&folder.join("game.mp4.part"));
    let mut first = launch(&output);
    first_library(&first);
    wait_for(&first.ui, |msg| {
        matches!(msg, UiMsg::Notice { .. }).then_some(())
    })
    .expect("first notice");
    first.engine.take();
    let video = fs::read(folder.join("game.mp4")).expect("video");
    let sidecar = fs::read(folder.join("game.json")).expect("sidecar");

    let second = launch(&output);
    let entries = first_library(&second);

    assert_eq!(entries.len(), 1);
    assert!(entries[0].meta.recovered);
    assert!(notices(&second).is_empty());
    assert_eq!(fs::read(folder.join("game.mp4")).expect("video"), video);
    assert_eq!(
        fs::read(folder.join("game.json")).expect("sidecar"),
        sidecar
    );
}

#[test]
fn a_part_that_cannot_be_repaired_stays_and_the_notice_offers_its_folder() {
    let output = temp_dir("recover-broken");
    let name = "2025-02-04_00-00-00_Ahri_CLASSIC";
    let media = media(120);
    let folder = leftover_part(&output, name, &media, false);
    let part = folder.join("game.mp4.part");
    fs::write(&part, b"not a video at all").expect("garbage");

    let harness = launch(&output);
    first_library(&harness);

    let notice = wait_for(&harness.ui, |msg| match msg {
        UiMsg::NoticeWithAction {
            level,
            text,
            action,
        } => Some((*level, text.clone(), action.clone())),
        _ => None,
    })
    .expect("failure notice");
    assert_eq!(notice.0, NoticeLevel::Warn);
    assert_eq!(notice.1, format!("Couldn't recover {name}"));
    assert_eq!(notice.2, NoticeAction::Reveal(folder.clone()));
    assert!(!folder.join("game.mp4").exists());
    assert_eq!(
        fs::read(folder.join("game.mp4.recovered-fmp4")).expect("kept"),
        b"not a video at all"
    );
    assert!(folder.join("game.json").is_file());
}

#[test]
fn nothing_is_said_when_every_recording_was_finished() {
    let output = temp_dir("recover-none");
    let harness = launch(&output);

    assert!(first_library(&harness).is_empty());
    assert!(notices(&harness).is_empty());
}

#[test]
fn recording_child() {
    let Ok(output) = std::env::var(CHILD_OUTPUT) else {
        return;
    };
    let league = Box::new(|out: Sender<LeagueMsg>| {
        out.send(LeagueMsg::WindowFound { hwnd: 7, pid: 42 }).ok();
        out.send(LeagueMsg::Ready(info())).ok();
        Box::new(out) as Box<dyn Send>
    });
    let _harness = start_engine(
        Settings {
            output_dir: PathBuf::from(output),
            ..Settings::default()
        },
        Arc::new(FakeMediaSource::new().with_speed(20)),
        league,
        None,
    );
    std::thread::sleep(Duration::from_secs(120));
}

struct KillOnDrop(Child);

impl Drop for KillOnDrop {
    fn drop(&mut self) {
        self.0.kill().ok();
        self.0.wait().ok();
    }
}

fn spawn_recording_child(output: &Path) -> KillOnDrop {
    Command::new(std::env::current_exe().expect("test binary"))
        .args([
            "--exact",
            "recording_child",
            "--nocapture",
            "--test-threads=1",
        ])
        .env(CHILD_OUTPUT, output)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map(KillOnDrop)
        .expect("spawn child")
}

fn recorded_secs(output: &Path) -> Option<f64> {
    let folder = fs::read_dir(output).ok()?.flatten().next()?.path();
    let inspected = inspect(&folder.join("game.mp4.part")).ok()?;
    let greyscreen_mp4::inspect::Layout::Unfinalized { fragments, .. } = inspected.layout else {
        return None;
    };
    (fragments >= MIN_FRAGMENTS).then(|| sample_secs(&inspected.tables.video, 90_000))
}

fn wait_for_fragments(output: &Path) -> f64 {
    let deadline = Instant::now() + Duration::from_secs(30);
    while Instant::now() < deadline {
        if let Some(secs) = recorded_secs(output) {
            return secs;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    panic!("the child never wrote {MIN_FRAGMENTS} fragments");
}

#[test]
fn a_recording_process_killed_mid_game_comes_back_playable_on_the_next_start() {
    if std::env::var_os(CHILD_OUTPUT).is_some() {
        return;
    }
    let output = temp_dir("recover-killed");
    let child = spawn_recording_child(&output);
    let flushed_secs = wait_for_fragments(&output);
    drop(child);
    assert!(flushed_secs > 0.0);

    let harness = launch(&output);
    let entries = first_library(&harness);

    assert_eq!(entries.len(), 1);
    let entry = &entries[0];
    assert!(entry.meta.recovered);
    assert!(
        entry.meta.duration_secs + 0.5 >= flushed_secs,
        "{} < {flushed_secs}",
        entry.meta.duration_secs
    );
    assert!(!entry.folder.join("game.mp4.part").exists());
    assert_playable_from_key_frame(&entry.folder.join("game.mp4"));
    let notice = wait_for(&harness.ui, |msg| match msg {
        UiMsg::Notice { level, text } => Some((*level, text.clone())),
        _ => None,
    })
    .expect("recovered notice");
    assert_eq!(notice, (NoticeLevel::Info, RECOVERED.to_owned()));
}
