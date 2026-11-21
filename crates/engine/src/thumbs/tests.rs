use std::sync::{Arc, Mutex};
use std::time::Duration;

use crossbeam_channel::bounded;
use greyscreen_core::game::{GameInfo, GameResult, Player, Team};
use greyscreen_core::library::{ClipEntry, GameRecord, META_VERSION, RecordingMeta, VideoInfo};
use greyscreen_core::media::{
    CaptureConfig, CaptureTarget, MediaEvent, MediaSource, Packet, Track, VideoFormat,
};
use greyscreen_core::settings::Settings;
use jiff::Zoned;

use super::*;
use crate::FakeMediaSource;

fn entry(folder: &Path, clips: &[&str]) -> RecordingEntry {
    let player = Player {
        name: "Me".into(),
        champion: "Ahri".into(),
        team: Team::Order,
        is_bot: false,
    };
    RecordingEntry {
        folder: folder.to_path_buf(),
        size_bytes: 0,
        meta: RecordingMeta {
            version: META_VERSION,
            id: "g".into(),
            started_at: Zoned::now(),
            duration_secs: 600.0,
            video: "game.mp4".into(),
            game: GameRecord {
                info: GameInfo {
                    mode: "CLASSIC".into(),
                    map: None,
                    me: player.clone(),
                    players: vec![player],
                },
                result: GameResult::Win,
            },
            events: Vec::new(),
            clips: clips
                .iter()
                .map(|file| ClipEntry {
                    file: (*file).into(),
                    start_secs: 100.0,
                    end_secs: 120.0,
                    label: "Kill".into(),
                })
                .collect(),
            encoder: "test".into(),
            video_format: VideoInfo {
                width: 1920,
                height: 1080,
                fps: 60,
            },
            review: Default::default(),
            health: None,
            recovered: false,
        },
    }
}

#[test]
fn a_game_with_clips_gets_one_job_per_video() {
    let rec = entry(Path::new("/lib/g"), &["clips/a.mp4", "clips/b.mp4"]);
    let dests: Vec<_> = jobs_for(&rec).into_iter().map(|j| j.dest).collect();
    assert_eq!(
        dests,
        [
            PathBuf::from("/lib/g/thumb.jpg"),
            PathBuf::from("/lib/g/clips/a.jpg"),
            PathBuf::from("/lib/g/clips/b.jpg"),
        ]
    );
}

#[test]
fn a_game_whose_video_never_finalized_gets_no_game_job() {
    let mut rec = entry(Path::new("/lib/g"), &["clips/a.mp4"]);
    rec.meta.video.clear();
    assert_eq!(jobs_for(&rec).len(), 1);
}

#[test]
fn clip_jobs_aim_inside_the_clip() {
    let rec = entry(Path::new("/lib/g"), &["clips/a.mp4"]);
    let job = clip_job(&rec.folder, &rec.meta.clips[0]);
    assert!((job.at_secs - 14.0).abs() < 1e-9);
    assert_eq!(job.video, Path::new("/lib/g/clips/a.mp4"));
}

fn fixture_clip(path: &Path) {
    let (tx, rx) = bounded(1024);
    let cfg: CaptureConfig = Settings::default().capture_config(2160);
    let session = FakeMediaSource::fast()
        .start(CaptureTarget::PrimaryMonitor, cfg, tx)
        .unwrap();
    let mut formats = None;
    let mut packets: Vec<Arc<Packet>> = Vec::new();
    while let Ok(event) = rx.recv_timeout(Duration::from_secs(15)) {
        match event {
            MediaEvent::Started { video, audio, .. } => formats = Some((video, audio)),
            MediaEvent::Packet(packet) => {
                let secs = packet.dts as f64 / f64::from(VideoFormat::TIMESCALE);
                let done = packet.track == Track::Video && secs >= 3.0;
                packets.push(Arc::new(packet));
                if done {
                    break;
                }
            }
            _ => {}
        }
    }
    drop(rx);
    session.stop();
    let (video, audio) = formats.unwrap();
    greyscreen_mp4::clip::write_clip(path, &video, audio.as_ref(), &packets).unwrap();
}

#[cfg(any(target_os = "macos", windows))]
#[test]
fn the_worker_turns_real_clips_into_jpeg_thumbnails_and_tells_the_ui() {
    let dir = tempfile::tempdir().unwrap();
    let clips = dir.path().join("clips");
    std::fs::create_dir_all(&clips).unwrap();
    fixture_clip(&clips.join("00-02_Kill.mp4"));
    let mut rec = entry(dir.path(), &["clips/00-02_Kill.mp4"]);
    rec.meta.video = String::new();
    rec.meta.clips[0].end_secs = rec.meta.clips[0].start_secs + 3.0;

    let seen = Arc::new(Mutex::new(Vec::new()));
    let inner = seen.clone();
    let sink = wrap(Arc::new(move |msg| inner.lock().unwrap().push(msg)));
    sink(UiMsg::Library(vec![rec]));

    let thumb = clips.join("00-02_Kill.jpg");
    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    while !seen
        .lock()
        .unwrap()
        .contains(&UiMsg::Thumbnail(thumb.clone()))
    {
        assert!(std::time::Instant::now() < deadline, "no thumbnail in time");
        std::thread::sleep(Duration::from_millis(50));
    }
    let image = image::open(&thumb).unwrap().to_rgb8();
    assert_eq!((image.width(), image.height()), (400, 225));
    let distinct: HashSet<[u8; 3]> = image.pixels().map(|p| p.0).collect();
    assert!(
        distinct.len() > 40,
        "the frame looks flat: {}",
        distinct.len()
    );
}
