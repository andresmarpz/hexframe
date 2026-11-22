#![cfg(windows)]

use std::collections::BTreeSet;
use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant};

use greyscreen_capture::clock::QpcMap;
use greyscreen_capture::com::ComGuard;
use greyscreen_capture::d3d::GpuDevice;
use greyscreen_capture::testing::cloaked_window::{
    COUNTER_BITS, CloakedWindow, Frame, FrameReader, PatternError, Reveal, read_counter,
};
use greyscreen_capture::wgc::{WgcCapture, WgcOptions};

const WIDTH: i32 = 640;
const HEIGHT: i32 = 360;
const RESIZED: (i32, i32) = (800, 450);
const WANTED_FRAMES: u64 = 60;
const FIRST_FRAMES_WAIT: Duration = Duration::from_secs(10);
const PHASE_WAIT: Duration = Duration::from_millis(2000);
const POLL: Duration = Duration::from_millis(1);
const CLOSE_OBSERVE: Duration = Duration::from_secs(5);
const KEEPS_ARRIVING_FPS: f64 = 30.0;
const STOPPED_FRAMES: u64 = 3;
const OPTIONS: WgcOptions = WgcOptions {
    fps: 60,
    cursor: false,
    display_hz: 60,
};
static SERIAL: Mutex<()> = Mutex::new(());

#[derive(Debug, Default)]
struct Phase {
    name: &'static str,
    frames: u64,
    ticks: u32,
    seconds: f64,
    checked: u32,
    pattern_ok: u32,
    first_error: Option<String>,
    sizes: BTreeSet<(usize, usize)>,
    counters: Vec<u16>,
}

impl Phase {
    fn rate(&self) -> f64 {
        if self.seconds > 0.0 {
            self.frames as f64 / self.seconds
        } else {
            0.0
        }
    }

    fn counters_advance(&self) -> bool {
        self.counters.windows(2).all(|pair| pair[1] >= pair[0])
    }

    fn report(&self) {
        eprintln!(
            "PHASE {:<14} window drew {:>4} ticks, capture delivered {:>4} frames in {:>5.2} s = {:>6.1} fps, pattern ok {}/{}, counters {} to {} (advancing: {}), sizes {:?}{}",
            self.name,
            self.ticks,
            self.frames,
            self.seconds,
            self.rate(),
            self.pattern_ok,
            self.checked,
            self.counters.first().map_or(-1, |c| i32::from(*c)),
            self.counters.last().map_or(-1, |c| i32::from(*c)),
            self.counters_advance(),
            self.sizes,
            self.first_error
                .as_ref()
                .map_or(String::new(), |e| format!(", first error: {e}")),
        );
    }
}

fn start(window: &CloakedWindow) -> Result<(GpuDevice, WgcCapture), String> {
    let target = window.target();
    let gpu = GpuDevice::for_target(&target)
        .map_err(|error| format!("no GPU for the window: {error}"))?;
    let capture = WgcCapture::start(&gpu, &target, QpcMap::now(), OPTIONS)
        .map_err(|error| format!("capture cannot start on the cloaked window: {error}"))?;
    Ok((gpu, capture))
}

fn latest_seq(capture: &WgcCapture) -> u64 {
    let seq = capture.latest().map_or(0, |(_, _, seq)| seq);
    capture.done_reading();
    seq
}

fn watch(
    name: &'static str,
    window: &CloakedWindow,
    reader: &mut FrameReader,
    capture: &WgcCapture,
    limit: Duration,
    stop: impl Fn(&Phase) -> bool,
) -> Phase {
    let mut phase = Phase {
        name,
        ..Phase::default()
    };
    let before = latest_seq(capture);
    let ticks_before = window.ticks();
    let mut seen = before;
    let started = Instant::now();
    while started.elapsed() < limit && !stop(&phase) {
        std::thread::sleep(POLL);
        let Some((texture, size, seq)) = capture.latest() else {
            continue;
        };
        if seq == seen {
            capture.done_reading();
            continue;
        }
        seen = seq;
        let frame = reader.read(&texture, size);
        capture.done_reading();
        phase.frames = seen - before;
        phase.seconds = started.elapsed().as_secs_f64();
        match frame {
            Ok(frame) => check(&mut phase, &frame),
            Err(error) => {
                phase
                    .first_error
                    .get_or_insert(format!("readback: {error}"));
            }
        }
    }
    phase.seconds = started.elapsed().as_secs_f64();
    phase.ticks = window.ticks().wrapping_sub(ticks_before);
    phase.report();
    phase
}

fn check(phase: &mut Phase, frame: &Frame) {
    phase.checked += 1;
    phase.sizes.insert((frame.width, frame.height));
    match read_counter(frame) {
        Ok(counter) => {
            phase.pattern_ok += 1;
            phase.counters.push(counter);
        }
        Err(error) => {
            phase.first_error.get_or_insert(error.to_string());
        }
    }
}

fn open(reveal: Reveal) -> CloakedWindow {
    let window = CloakedWindow::open(WIDTH, HEIGHT, reveal)
        .unwrap_or_else(|error| panic!("cannot make the cloaked window: {error}"));
    assert!(window.is_cloaked(), "the cloak must read back");
    assert!(window.foreground_unchanged());
    eprintln!(
        "WINDOW {reveal:?}: cloaked {}, client {:?}, minimised {}",
        window.is_cloaked(),
        window.client_size(),
        window.is_minimised()
    );
    window
}

#[test]
fn the_pattern_decodes_from_a_synthetic_frame() {
    let (width, height) = (320usize, 180usize);
    let mut frame = Frame {
        width,
        height,
        bgra: vec![0; width * height * 4],
    };
    let counter = 0xA5C3u16;
    for y in 0..height {
        for x in 0..width {
            let rgb = match (x < width / 2, y < height / 2) {
                (true, true) => [255, 0, 0],
                (false, true) => [0, 255, 0],
                (true, false) => [0, 0, 255],
                (false, false) => [255, 255, 0],
            };
            let at = (y * width + x) * 4;
            frame.bgra[at..at + 4].copy_from_slice(&[rgb[2], rgb[1], rgb[0], 255]);
        }
    }
    for bit in 0..COUNTER_BITS {
        let value = if counter >> (COUNTER_BITS - 1 - bit) & 1 == 1 {
            255
        } else {
            0
        };
        for y in height * 45 / 100..height * 55 / 100 {
            for x in bit * width / COUNTER_BITS..(bit + 1) * width / COUNTER_BITS {
                let at = (y * width + x) * 4;
                frame.bgra[at..at + 4].copy_from_slice(&[value, value, value, 255]);
            }
        }
    }
    assert_eq!(read_counter(&frame), Ok(counter));
    frame.bgra[(height / 4 * width + width / 4) * 4..][..4].copy_from_slice(&[0, 0, 0, 255]);
    assert_eq!(
        read_counter(&frame),
        Err(PatternError::Quadrant("top left"))
    );
}

#[test]
#[ignore = "needs the PC: opens a cloaked window and captures it"]
fn a_cloaked_window_delivers_sixty_frames_holding_the_pattern() {
    let _serial = SERIAL.lock().unwrap_or_else(PoisonError::into_inner);
    let _com = ComGuard::mta().expect("COM");
    let window = open(Reveal::ShownCloaked);
    let (gpu, capture) = start(&window).expect("capture on the cloaked window");
    let mut reader = FrameReader::new(&gpu);
    let phase = watch(
        "cloaked",
        &window,
        &mut reader,
        &capture,
        FIRST_FRAMES_WAIT,
        |p| p.frames >= WANTED_FRAMES,
    );
    assert!(
        window.foreground_unchanged(),
        "the foreground window changed"
    );
    assert!(phase.frames >= WANTED_FRAMES, "too few frames: {phase:?}");
    assert_eq!(phase.pattern_ok, phase.checked, "{phase:?}");
    assert!(phase.counters_advance(), "{phase:?}");
    assert!(
        phase
            .sizes
            .iter()
            .all(|(w, h)| w * HEIGHT as usize == h * WIDTH as usize),
        "the frame lost the window aspect: {phase:?}"
    );
    capture.stop();
}

#[test]
#[ignore = "needs the PC: opens a cloaked window and captures it"]
fn frames_through_resize_minimise_hide_and_close() {
    let _serial = SERIAL.lock().unwrap_or_else(PoisonError::into_inner);
    let _com = ComGuard::mta().expect("COM");
    let window = open(Reveal::ShownCloaked);
    let (gpu, capture) = start(&window).expect("capture on the cloaked window");
    let mut reader = FrameReader::new(&gpu);
    watch(
        "baseline",
        &window,
        &mut reader,
        &capture,
        PHASE_WAIT,
        |_| false,
    );
    window.resize(RESIZED.0, RESIZED.1).expect("resize");
    let resized = watch(
        "resized",
        &window,
        &mut reader,
        &capture,
        PHASE_WAIT,
        |_| false,
    );
    window.minimise();
    eprintln!("minimised {}", window.is_minimised());
    let minimised = watch(
        "minimised",
        &window,
        &mut reader,
        &capture,
        PHASE_WAIT,
        |_| false,
    );
    window.restore();
    let restored = watch(
        "restored",
        &window,
        &mut reader,
        &capture,
        PHASE_WAIT,
        |_| false,
    );
    window.hide();
    let hidden = watch("hidden", &window, &mut reader, &capture, PHASE_WAIT, |_| {
        false
    });
    window.reshow();
    let shown = watch(
        "shown again",
        &window,
        &mut reader,
        &capture,
        PHASE_WAIT,
        |_| false,
    );
    assert!(
        window.foreground_unchanged(),
        "the foreground window changed"
    );
    assert!(resized.rate() >= KEEPS_ARRIVING_FPS, "resize: {resized:?}");
    assert!(
        resized.sizes.contains(&(1200, 675)),
        "the frame did not take the new size: {resized:?}"
    );
    assert!(
        minimised.frames <= STOPPED_FRAMES,
        "minimise: {minimised:?}"
    );
    assert!(
        restored.rate() >= KEEPS_ARRIVING_FPS,
        "restore: {restored:?}"
    );
    assert!(hidden.frames <= STOPPED_FRAMES, "hide: {hidden:?}");
    assert!(shown.rate() >= KEEPS_ARRIVING_FPS, "show: {shown:?}");
    let seq_before_close = latest_seq(&capture);
    let closed_at = Instant::now();
    assert!(window.close(), "the window thread did not finish");
    let mut closed = capture.is_closed();
    while !closed && closed_at.elapsed() < CLOSE_OBSERVE {
        std::thread::sleep(POLL);
        closed = capture.is_closed();
    }
    eprintln!(
        "CLOSE capture item closed {closed} after {:.0} ms, frames after the close: {}",
        closed_at.elapsed().as_secs_f64() * 1000.0,
        latest_seq(&capture) - seq_before_close
    );
    capture.stop();
}

#[test]
#[ignore = "needs the PC: opens a window that is never shown"]
fn a_window_that_is_never_shown_cannot_be_captured() {
    let _serial = SERIAL.lock().unwrap_or_else(PoisonError::into_inner);
    let _com = ComGuard::mta().expect("COM");
    let window = open(Reveal::NeverShown);
    let started = start(&window);
    eprintln!("NEVER SHOWN: {:?}", started.as_ref().err());
    assert!(started.is_err(), "a never shown window was captured");
}
