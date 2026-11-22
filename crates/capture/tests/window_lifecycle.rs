#![cfg(windows)]

use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant};

use greyscreen_capture::clock::QpcMap;
use greyscreen_capture::com::ComGuard;
use greyscreen_capture::d3d::GpuDevice;
use greyscreen_capture::testing::cloaked_window::{CloakedWindow, FrameReader, Reveal, read_counter};
use greyscreen_capture::wgc::{WgcCapture, WgcOptions};
use greyscreen_core::health::StallWatch;
use greyscreen_core::time::HNS_PER_SEC;

const WIDTH: i32 = 640;
const HEIGHT: i32 = 360;
const SIZES: [(i32, i32); 3] = [(640, 360), (800, 450), (1024, 576)];
const RESIZES: usize = 20;
const IDLE: Duration = Duration::from_secs(15);
const WARM: Duration = Duration::from_secs(2);
const RESUME: Duration = Duration::from_secs(3);
const POLL: Duration = Duration::from_millis(5);
const SAMPLE: Duration = Duration::from_millis(250);
const RESIZE_SETTLE: Duration = Duration::from_millis(400);
const LONGEST_RESIZE_GAP: Duration = Duration::from_secs(1);
const CLOSE_OBSERVE: Duration = Duration::from_secs(10);
const MIN_FPS: f64 = 30.0;
const OPTIONS: WgcOptions = WgcOptions {
    fps: 60,
    cursor: false,
    display_hz: 60,
};
static SERIAL: Mutex<()> = Mutex::new(());

fn start(window: &CloakedWindow) -> (GpuDevice, WgcCapture) {
    let target = window.target();
    let gpu = GpuDevice::for_target(&target).expect("GPU for the window");
    let capture =
        WgcCapture::start(&gpu, &target, QpcMap::now(), OPTIONS).expect("capture the window");
    (gpu, capture)
}

fn open() -> CloakedWindow {
    let window = CloakedWindow::open(WIDTH, HEIGHT, Reveal::ShownCloaked).expect("cloaked window");
    assert!(window.is_cloaked(), "the cloak must read back");
    window
}

fn seq(capture: &WgcCapture) -> u64 {
    let seq = capture.latest().map_or(0, |(_, _, seq)| seq);
    capture.done_reading();
    seq
}

struct Run {
    stalls: StallWatch,
    began: Instant,
    audio: u64,
    last_sample: Instant,
}

impl Run {
    fn new() -> Self {
        Self {
            stalls: StallWatch::default(),
            began: Instant::now(),
            audio: 0,
            last_sample: Instant::now(),
        }
    }

    fn sample(&mut self, capture: &WgcCapture) {
        self.audio += 1;
        let at = (self.began.elapsed().as_secs_f64() * HNS_PER_SEC as f64) as i64;
        self.stalls.observe(at, seq(capture), self.audio);
        self.last_sample = Instant::now();
    }

    fn run_for(&mut self, capture: &WgcCapture, span: Duration) -> f64 {
        let before = seq(capture);
        let started = Instant::now();
        while started.elapsed() < span {
            std::thread::sleep(POLL);
            if self.last_sample.elapsed() >= SAMPLE {
                self.sample(capture);
            }
        }
        self.sample(capture);
        (seq(capture) - before) as f64 / started.elapsed().as_secs_f64()
    }
}

fn pattern_reads(capture: &WgcCapture, reader: &mut FrameReader) -> bool {
    let ok = capture.latest().is_some_and(|(texture, size, _)| {
        reader
            .read(&texture, size)
            .is_ok_and(|frame| read_counter(&frame).is_ok())
    });
    capture.done_reading();
    ok
}

fn idle_then_resume(away: fn(&CloakedWindow), back: fn(&CloakedWindow)) {
    let _serial = SERIAL.lock().unwrap_or_else(PoisonError::into_inner);
    let _com = ComGuard::mta().expect("COM");
    let window = open();
    let (gpu, capture) = start(&window);
    let mut reader = FrameReader::new(&gpu);
    let mut run = Run::new();
    assert!(run.run_for(&capture, WARM) >= MIN_FPS, "no frames to start");
    away(&window);
    run.run_for(&capture, IDLE);
    back(&window);
    let resumed = run.run_for(&capture, RESUME);
    let (count, longest) = run.stalls.summary();
    eprintln!("IDLE stalls {count}, longest {longest:.1} s, resumed {resumed:.1} fps");
    assert!(window.foreground_unchanged(), "the foreground changed");
    assert!(resumed >= MIN_FPS, "frames did not resume: {resumed}");
    assert!(pattern_reads(&capture, &mut reader), "the pattern is lost");
    assert_eq!(count, 1, "one stall expected");
    assert!(
        (13.0..=19.0).contains(&longest),
        "the stall should last about 15 s: {longest}"
    );
    capture.stop();
}

#[test]
#[ignore = "needs the PC: opens a cloaked window and captures it"]
fn minimising_for_fifteen_seconds_records_one_stall_and_frames_resume() {
    idle_then_resume(CloakedWindow::minimise, CloakedWindow::restore);
}

#[test]
#[ignore = "needs the PC: opens a cloaked window and captures it"]
fn hiding_for_fifteen_seconds_records_one_stall_and_frames_resume() {
    idle_then_resume(CloakedWindow::hide, CloakedWindow::reshow);
}

#[test]
#[ignore = "needs the PC: opens a cloaked window and captures it"]
fn twenty_resizes_across_three_sizes_keep_frames_arriving() {
    let _serial = SERIAL.lock().unwrap_or_else(PoisonError::into_inner);
    let _com = ComGuard::mta().expect("COM");
    let window = open();
    let (gpu, capture) = start(&window);
    let mut reader = FrameReader::new(&gpu);
    let mut run = Run::new();
    run.run_for(&capture, WARM);
    let mut longest_gap = Duration::ZERO;
    for step in 0..RESIZES {
        let (width, height) = SIZES[step % SIZES.len()];
        window.resize(width, height).expect("resize");
        let mut seen = seq(&capture);
        let mut last_new = Instant::now();
        let started = Instant::now();
        while started.elapsed() < RESIZE_SETTLE {
            std::thread::sleep(POLL);
            let now = seq(&capture);
            if now != seen {
                seen = now;
                longest_gap = longest_gap.max(last_new.elapsed());
                last_new = Instant::now();
            }
        }
        longest_gap = longest_gap.max(last_new.elapsed());
        assert!(
            pattern_reads(&capture, &mut reader),
            "resize {step}: no pattern"
        );
    }
    run.sample(&capture);
    let (count, _) = run.stalls.summary();
    eprintln!("RESIZE longest gap between frames {longest_gap:?}, stalls {count}");
    assert!(
        longest_gap < LONGEST_RESIZE_GAP,
        "frames paused: {longest_gap:?}"
    );
    assert_eq!(count, 0, "a resize must not look like a stall");
    capture.stop();
}

#[test]
#[ignore = "needs the PC: opens a cloaked window and captures it"]
fn closing_the_window_is_reported_closed_and_a_relaunch_captures() {
    let _serial = SERIAL.lock().unwrap_or_else(PoisonError::into_inner);
    let _com = ComGuard::mta().expect("COM");
    let window = open();
    let (_gpu, capture) = start(&window);
    let mut run = Run::new();
    run.run_for(&capture, WARM);
    assert!(window.close(), "the window thread did not finish");
    let closed_at = Instant::now();
    while !capture.is_closed() {
        assert!(
            closed_at.elapsed() < CLOSE_OBSERVE,
            "the capture never reported the closed window"
        );
        std::thread::sleep(POLL);
    }
    eprintln!("CLOSE reported after {:?}", closed_at.elapsed());
    capture.stop();

    let again = open();
    let (_gpu, capture) = start(&again);
    let mut run = Run::new();
    assert!(
        run.run_for(&capture, WARM) >= MIN_FPS,
        "the relaunch did not capture"
    );
    capture.stop();
}
