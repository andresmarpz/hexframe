use std::collections::VecDeque;
use std::sync::Mutex;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

use crossbeam_channel::{Sender, bounded, unbounded};
use greyscreen_core::export::{EncodePlan, ExportPreset, ExportRange};
use greyscreen_core::library::VideoInfo;
use greyscreen_core::media::{
    CaptureConfig, CaptureTarget, MediaEvent, MediaSource, Packet, Track, VideoFormat,
};
use greyscreen_core::settings::Settings;

use super::*;
use crate::FakeMediaSource;

const SOURCE_SECS: f64 = 6.0;
const VIDEO: VideoInfo = VideoInfo {
    width: 1920,
    height: 1080,
    fps: 60,
};

fn fixture_mp4(path: &Path) {
    let (tx, rx) = bounded(4096);
    let cfg: CaptureConfig = Settings::default().capture_config(2160);
    let session = FakeMediaSource::fast()
        .start(CaptureTarget::PrimaryMonitor, cfg, tx)
        .unwrap();
    let mut formats = None;
    let mut packets: Vec<Arc<Packet>> = Vec::new();
    while let Ok(event) = rx.recv_timeout(Duration::from_secs(30)) {
        match event {
            MediaEvent::Started { video, audio, .. } => formats = Some((video, audio)),
            MediaEvent::Packet(packet) => {
                let secs = packet.dts as f64 / f64::from(VideoFormat::TIMESCALE);
                let done = packet.track == Track::Video && secs >= SOURCE_SECS;
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

struct Scratch {
    _dir: tempfile::TempDir,
    source: PathBuf,
    out: PathBuf,
}

fn scratch() -> Scratch {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("game.mp4");
    fixture_mp4(&source);
    let out = dir.path().join("exports");
    Scratch {
        _dir: dir,
        source,
        out,
    }
}

fn job(scratch: &Scratch, preset: ExportPreset, start: f64, end: f64) -> ExportJob {
    ExportJob {
        source: scratch.source.clone(),
        source_offset_secs: 0.0,
        range: ExportRange::new(start, end),
        preset,
        video: VIDEO,
        dest_dir: scratch.out.clone(),
        stem: "Ahri_Test".into(),
    }
}

struct Fake {
    sizes: Mutex<VecDeque<u64>>,
    plans: Mutex<Vec<EncodePlan>>,
    cuts: Mutex<Vec<(PathBuf, f64, f64)>>,
    block_until_cancelled: Option<Sender<()>>,
    fail: Option<ReencodeError>,
}

impl Fake {
    fn writing(sizes: &[u64]) -> Self {
        Self {
            sizes: Mutex::new(sizes.iter().copied().collect()),
            plans: Mutex::new(Vec::new()),
            cuts: Mutex::new(Vec::new()),
            block_until_cancelled: None,
            fail: None,
        }
    }

    fn blocking(started: Sender<()>) -> Self {
        Self {
            block_until_cancelled: Some(started),
            ..Self::writing(&[])
        }
    }

    fn failing(error: ReencodeError) -> Self {
        Self {
            fail: Some(error),
            ..Self::writing(&[])
        }
    }

    fn cuts(&self) -> Vec<(PathBuf, f64, f64)> {
        self.cuts.lock().unwrap().clone()
    }

    fn plans(&self) -> Vec<EncodePlan> {
        self.plans.lock().unwrap().clone()
    }
}

impl Reencode for Fake {
    fn run(
        &self,
        request: &ReencodeRequest<'_>,
        progress: &mut dyn FnMut(f32),
        cancel: &AtomicBool,
    ) -> Result<(), ReencodeError> {
        self.plans.lock().unwrap().push(request.plan);
        self.cuts.lock().unwrap().push((
            request.src.to_path_buf(),
            request.start_secs,
            request.end_secs,
        ));
        if let Some(error) = &self.fail {
            return Err(match error {
                ReencodeError::Cancelled => ReencodeError::Cancelled,
                ReencodeError::Unsupported => ReencodeError::Unsupported,
                ReencodeError::Failed(why) => ReencodeError::Failed(why.clone()),
            });
        }
        if let Some(started) = &self.block_until_cancelled {
            std::fs::write(request.dst, b"partial").unwrap();
            started.send(()).unwrap();
            while !cancel.load(Ordering::SeqCst) {
                std::thread::sleep(Duration::from_millis(5));
            }
            return Err(ReencodeError::Cancelled);
        }
        progress(0.5);
        let size = self.sizes.lock().unwrap().pop_front().unwrap_or(1_000);
        let file = std::fs::File::create(request.dst).unwrap();
        file.set_len(size).unwrap();
        progress(1.0);
        Ok(())
    }
}

fn run_job(job: &ExportJob, reencoder: &dyn Reencode) -> (ExportEvent, Vec<ExportEvent>) {
    let mut seen = Vec::new();
    let cancel = AtomicBool::new(false);
    let event = run(job, reencoder, &mut |event| seen.push(event), &cancel);
    (event, seen)
}

fn done(event: ExportEvent) -> ExportDone {
    match event {
        ExportEvent::Done(done) => done,
        other => panic!("expected Done, got {other:?}"),
    }
}

fn leftovers(dir: &Path) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

#[test]
fn original_quality_cuts_at_the_keyframe_before_the_start() {
    let scratch = scratch();
    let (event, seen) = run_job(
        &job(&scratch, ExportPreset::Original, 2.4, 4.2),
        &Fake::writing(&[]),
    );
    let done = done(event);
    assert_eq!(done.path, scratch.out.join("Ahri_Test.mp4"));
    assert_eq!(done.range.start, 2.0);
    assert!((done.range.end - 4.2).abs() < 0.02);
    assert!(!done.shrunk);
    assert_eq!(done.preset, ExportPreset::Original);
    assert_eq!(done.bytes, std::fs::metadata(&done.path).unwrap().len());
    assert!(seen.contains(&ExportEvent::Progress(0.0)));
    assert_eq!(leftovers(&scratch.out), ["Ahri_Test.mp4"]);

    let movie = Movie::open(&done.path).unwrap();
    assert!(movie.video_samples[0].key);
    assert!((movie.video_secs() - done.range.duration()).abs() < 0.02);
}

#[test]
fn a_clip_file_source_maps_recording_time_back_to_file_time() {
    let scratch = scratch();
    let mut clip = job(&scratch, ExportPreset::Original, 102.4, 104.0);
    clip.source_offset_secs = 100.0;
    let done = done(run_job(&clip, &Fake::writing(&[])).0);
    assert_eq!(done.range.start, 102.0);
    assert!((done.range.end - 104.0).abs() < 0.02);
}

#[test]
fn exporting_twice_keeps_both_files() {
    let scratch = scratch();
    let first = done(
        run_job(
            &job(&scratch, ExportPreset::Original, 1.0, 3.0),
            &Fake::writing(&[]),
        )
        .0,
    );
    let second = done(
        run_job(
            &job(&scratch, ExportPreset::Original, 1.0, 3.0),
            &Fake::writing(&[]),
        )
        .0,
    );
    assert_eq!(first.path.file_name().unwrap(), "Ahri_Test.mp4");
    assert_eq!(second.path.file_name().unwrap(), "Ahri_Test (2).mp4");
    assert_eq!(leftovers(&scratch.out).len(), 2);
}

#[test]
fn exporting_into_the_source_folder_never_touches_the_source() {
    let scratch = scratch();
    let before = std::fs::read(&scratch.source).unwrap();
    let mut here = job(&scratch, ExportPreset::Original, 1.0, 3.0);
    here.dest_dir = scratch.source.parent().unwrap().to_path_buf();
    here.stem = "game".into();
    let saved = done(run_job(&here, &Fake::writing(&[])).0);
    assert_eq!(saved.path.file_name().unwrap(), "game (2).mp4");
    assert_eq!(std::fs::read(&scratch.source).unwrap(), before);
}

#[test]
fn a_source_that_was_deleted_says_so() {
    let scratch = scratch();
    std::fs::remove_file(&scratch.source).unwrap();
    let (event, _) = run_job(
        &job(&scratch, ExportPreset::Original, 1.0, 3.0),
        &Fake::writing(&[]),
    );
    assert_eq!(event, ExportEvent::Failed(NO_SOURCE.into()));
    assert!(leftovers(&scratch.out).is_empty());
}

#[test]
fn an_unusable_destination_is_reported() {
    let scratch = scratch();
    let mut blocked = job(&scratch, ExportPreset::Original, 1.0, 3.0);
    blocked.dest_dir = scratch.source.join("inside-a-file");
    let (event, _) = run_job(&blocked, &Fake::writing(&[]));
    assert!(
        matches!(event, ExportEvent::Failed(text) if text.starts_with("Couldn't use the folder"))
    );
}

#[test]
fn discord_encodes_with_a_plan_that_fits_and_reports_progress() {
    let scratch = scratch();
    let fake = Fake::writing(&[7_000_000]);
    let (event, seen) = run_job(&job(&scratch, ExportPreset::Discord, 1.0, 5.0), &fake);
    let done = done(event);
    assert_eq!(done.bytes, 7_000_000);
    assert!(!done.shrunk);
    assert_eq!(done.range, ExportRange::new(1.0, 5.0));
    let plans = fake.plans();
    assert_eq!(plans.len(), 1);
    assert_eq!(fake.cuts(), [(scratch.source.clone(), 1.0, 5.0)]);
    assert_eq!((plans[0].width, plans[0].height), (1280, 720));
    assert!(seen.contains(&ExportEvent::Progress(0.5)));
    assert!(!seen.contains(&ExportEvent::Shrinking));
    assert_eq!(leftovers(&scratch.out), ["Ahri_Test.mp4"]);
}

#[test]
fn discord_asks_for_the_end_of_the_video_when_the_range_runs_past_it() {
    let scratch = scratch();
    let done = done(
        run_job(
            &job(&scratch, ExportPreset::Discord, 2.0, 500.0),
            &Fake::writing(&[1_000]),
        )
        .0,
    );
    assert!((done.range.end - SOURCE_SECS).abs() < 0.05);
}

#[test]
fn an_oversized_discord_file_is_retried_once_at_a_lower_bitrate() {
    let scratch = scratch();
    let fake = Fake::writing(&[11_500_000, 8_800_000]);
    let (event, seen) = run_job(&job(&scratch, ExportPreset::Discord, 1.0, 5.0), &fake);
    let done = done(event);
    assert!(done.shrunk);
    assert_eq!(done.bytes, 8_800_000);
    let plans = fake.plans();
    assert_eq!(plans.len(), 2);
    assert!(plans[1].video_bps < plans[0].video_bps);
    assert!(seen.contains(&ExportEvent::Shrinking));
    assert_eq!(leftovers(&scratch.out), ["Ahri_Test.mp4"]);
}

#[test]
fn a_file_that_is_still_too_big_after_the_retry_fails_and_leaves_nothing() {
    let scratch = scratch();
    let fake = Fake::writing(&[11_500_000, 10_400_000]);
    let (event, _) = run_job(&job(&scratch, ExportPreset::Discord, 1.0, 5.0), &fake);
    assert!(matches!(event, ExportEvent::Failed(text) if text.contains("under 10 MB")));
    assert_eq!(fake.plans().len(), 2);
    assert!(leftovers(&scratch.out).is_empty());
}

#[test]
fn an_encoder_failure_is_plain_and_leaves_nothing() {
    let scratch = scratch();
    let (event, _) = run_job(
        &job(&scratch, ExportPreset::Discord, 1.0, 5.0),
        &Fake::failing(ReencodeError::Failed("MFT 0xC00D36B4".into())),
    );
    assert_eq!(
        event,
        ExportEvent::Failed("Couldn't make the smaller file. Try Original quality.".into())
    );
    assert!(leftovers(&scratch.out).is_empty());
}

#[test]
fn a_platform_without_an_encoder_says_only_original_works() {
    let scratch = scratch();
    let (event, _) = run_job(
        &job(&scratch, ExportPreset::Discord, 1.0, 5.0),
        &Fake::failing(ReencodeError::Unsupported),
    );
    assert!(matches!(event, ExportEvent::Failed(text) if text.contains("Only Original quality")));
}

#[test]
fn cancelling_an_encode_stops_it_and_removes_the_partial_file() {
    let scratch = scratch();
    let (started_tx, started_rx) = unbounded();
    let fake = Arc::new(Fake::blocking(started_tx));
    let cancel = Arc::new(AtomicBool::new(false));
    let worker = {
        let (fake, cancel) = (fake.clone(), cancel.clone());
        let job = job(&scratch, ExportPreset::Discord, 1.0, 5.0);
        std::thread::spawn(move || run(&job, fake.as_ref(), &mut |_| {}, &cancel))
    };
    started_rx.recv_timeout(Duration::from_secs(10)).unwrap();
    assert_eq!(leftovers(&scratch.out), ["Ahri_Test.mp4.part"]);
    cancel.store(true, Ordering::SeqCst);
    assert_eq!(worker.join().unwrap(), ExportEvent::Cancelled);
    assert!(leftovers(&scratch.out).is_empty());
}

#[test]
fn cancelling_a_finished_cut_before_it_is_saved_leaves_no_file() {
    let scratch = scratch();
    let cancel = AtomicBool::new(true);
    let event = run(
        &job(&scratch, ExportPreset::Original, 1.0, 3.0),
        &Fake::writing(&[]),
        &mut |_| {},
        &cancel,
    );
    assert_eq!(event, ExportEvent::Cancelled);
    assert!(leftovers(&scratch.out).is_empty());
}

#[test]
fn the_handle_runs_one_export_at_a_time_and_cancel_frees_it() {
    let scratch = scratch();
    let (started_tx, started_rx) = unbounded();
    let handle = ExportHandle::new(Arc::new(Fake::blocking(started_tx)));
    let (events_tx, events_rx) = unbounded();
    let sink: Sink = Arc::new(move |msg| {
        if let UiMsg::Export(event) = msg {
            events_tx.send(event).unwrap();
        }
    });
    handle.start(job(&scratch, ExportPreset::Discord, 1.0, 5.0), sink.clone());
    started_rx.recv_timeout(Duration::from_secs(10)).unwrap();

    handle.start(job(&scratch, ExportPreset::Discord, 1.0, 5.0), sink.clone());
    let busy = loop {
        match events_rx.recv_timeout(Duration::from_secs(10)).unwrap() {
            ExportEvent::Progress(_) => {}
            other => break other,
        }
    };
    assert_eq!(
        busy,
        ExportEvent::Failed("Another export is still running.".into())
    );

    handle.cancel();
    loop {
        let event = events_rx.recv_timeout(Duration::from_secs(10)).unwrap();
        if event == ExportEvent::Cancelled {
            break;
        }
        assert!(matches!(event, ExportEvent::Progress(_)));
    }

    handle.start(job(&scratch, ExportPreset::Original, 1.0, 3.0), sink);
    loop {
        match events_rx.recv_timeout(Duration::from_secs(10)).unwrap() {
            ExportEvent::Done(done) => {
                assert_eq!(done.preset, ExportPreset::Original);
                break;
            }
            ExportEvent::Progress(_) => {}
            other => panic!("unexpected {other:?}"),
        }
    }
}
