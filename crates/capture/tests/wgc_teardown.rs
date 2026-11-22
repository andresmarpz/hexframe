#![cfg(windows)]

use std::io::Write;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock, PoisonError, RwLock};
use std::time::{Duration, Instant};

use greyscreen_capture::clock::QpcMap;
use greyscreen_capture::com::ComGuard;
use greyscreen_capture::d3d::GpuDevice;
use greyscreen_capture::fault::{Armed, Point};
use greyscreen_capture::testing::cloaked_window::{CloakedWindow, Reveal};
use greyscreen_capture::wgc::{WgcCapture, WgcOptions};
use greyscreen_core::health::CaptureStats;
use windows::Win32::Foundation::E_FAIL;

const OPTIONS: WgcOptions = WgcOptions {
    fps: 60,
    cursor: false,
    display_hz: 60,
};
const FIRST_FRAME_WAIT: Duration = Duration::from_secs(10);
const STRESS_FOR: Duration = Duration::from_secs(60);
const WATCHDOG: Duration = Duration::from_secs(90);
const FAILING_FOR: Duration = Duration::from_secs(2);
const FRAME_FAILED: &str = "capture frame failed";

static SERIAL: Mutex<()> = Mutex::new(());
static LOG: OnceLock<Arc<Mutex<Vec<u8>>>> = OnceLock::new();

#[derive(Clone)]
struct LogSink(Arc<Mutex<Vec<u8>>>);

impl Write for LogSink {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn log() -> &'static Arc<Mutex<Vec<u8>>> {
    LOG.get_or_init(|| {
        let sink = Arc::new(Mutex::new(Vec::new()));
        let writer = LogSink(Arc::clone(&sink));
        let _ = tracing_subscriber::fmt()
            .with_writer(move || writer.clone())
            .with_ansi(false)
            .with_max_level(tracing::Level::DEBUG)
            .try_init();
        sink
    })
}

fn log_count(needle: &str) -> usize {
    let text = String::from_utf8_lossy(&log().lock().unwrap_or_else(PoisonError::into_inner))
        .into_owned();
    text.matches(needle).count()
}

struct Rig {
    _com: ComGuard,
    window: CloakedWindow,
    _gpu: GpuDevice,
    capture: WgcCapture,
}

fn rig() -> Rig {
    log();
    let com = ComGuard::mta().expect("COM");
    let window = CloakedWindow::open(640, 360, Reveal::ShownCloaked).expect("cloaked window");
    let target = window.target();
    let gpu = GpuDevice::for_target(&target).expect("GPU for the window");
    let capture =
        WgcCapture::start(&gpu, &target, QpcMap::now(), OPTIONS).expect("capture starts");
    Rig {
        _com: com,
        window,
        _gpu: gpu,
        capture,
    }
}

fn wait_for_frame(capture: &WgcCapture) -> bool {
    let started = Instant::now();
    while started.elapsed() < FIRST_FRAME_WAIT {
        if capture.has_frame() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    false
}

fn under_watchdog(work: impl FnOnce() + Send + 'static) {
    let (done, finished) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        work();
        let _ = done.send(());
    });
    assert!(
        finished.recv_timeout(WATCHDOG).is_ok(),
        "deadlock: the stress run did not finish in time"
    );
}

#[test]
#[ignore = "needs the PC: opens a cloaked window and captures it"]
fn stop_then_drop_and_drop_alone_do_not_panic() {
    let _serial = SERIAL.lock().unwrap_or_else(PoisonError::into_inner);
    let first = rig();
    assert!(wait_for_frame(&first.capture), "no frame before stop");
    first.capture.stop();
    assert_eq!(log_count("capture stopped"), 1, "stop logs once");
    let second = rig();
    assert!(wait_for_frame(&second.capture), "the next start works");
    drop(second.capture);
    assert!(second.window.close());
    assert!(first.window.close());
}

#[test]
#[ignore = "needs the PC: opens a cloaked window and captures it"]
fn the_latest_frame_is_empty_after_stop() {
    let _serial = SERIAL.lock().unwrap_or_else(PoisonError::into_inner);
    let rig = rig();
    assert!(wait_for_frame(&rig.capture));
    let shared = Arc::new(RwLock::new(Some(rig.capture)));
    let reader = Arc::clone(&shared);
    let reading = Arc::new(AtomicBool::new(true));
    let keep_reading = Arc::clone(&reading);
    under_watchdog(move || {
        let handle = std::thread::spawn(move || {
            while keep_reading.load(Ordering::Acquire) {
                let guard = reader.read().unwrap_or_else(PoisonError::into_inner);
                match guard.as_ref() {
                    Some(capture) => {
                        let _ = capture.latest();
                        capture.done_reading();
                    }
                    None => break,
                }
            }
        });
        std::thread::sleep(Duration::from_millis(300));
        let capture = shared
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .take();
        if let Some(capture) = capture {
            capture.stop();
        }
        reading.store(false, Ordering::Release);
        handle.join().expect("the reader thread must not panic");
    });
    assert!(rig.window.close());
}

#[test]
#[ignore = "needs the PC: opens a cloaked window and captures it for 60 s"]
fn four_threads_drive_the_capture_for_a_minute_without_deadlock() {
    let _serial = SERIAL.lock().unwrap_or_else(PoisonError::into_inner);
    let rig = rig();
    assert!(wait_for_frame(&rig.capture));
    let shared = Arc::new(RwLock::new(Some(rig.capture)));
    let running = Arc::new(AtomicBool::new(true));
    let reads = Arc::new(AtomicU64::new(0));
    let backwards = Arc::new(AtomicU64::new(0));
    let after_stop = Arc::new(AtomicU64::new(0));
    let stopped = Arc::new(AtomicBool::new(false));
    let counters = (
        Arc::clone(&reads),
        Arc::clone(&backwards),
        Arc::clone(&after_stop),
    );
    let (shared_in, running_in, stopped_in) = (
        Arc::clone(&shared),
        Arc::clone(&running),
        Arc::clone(&stopped),
    );
    under_watchdog(move || {
        let mut threads = Vec::new();
        for role in 0..3 {
            let shared = Arc::clone(&shared_in);
            let running = Arc::clone(&running_in);
            let stopped = Arc::clone(&stopped_in);
            let (reads, backwards, after_stop) = (
                Arc::clone(&counters.0),
                Arc::clone(&counters.1),
                Arc::clone(&counters.2),
            );
            threads.push(std::thread::spawn(move || {
                let mut last_seq = 0;
                let mut last_torn = 0;
                while running.load(Ordering::Acquire) {
                    let guard = shared.read().unwrap_or_else(PoisonError::into_inner);
                    let Some(capture) = guard.as_ref() else {
                        break;
                    };
                    match role {
                        0 => {
                            if let Some((_, _, seq)) = capture.latest() {
                                reads.fetch_add(1, Ordering::Relaxed);
                                if seq < last_seq {
                                    backwards.fetch_add(1, Ordering::Relaxed);
                                }
                                last_seq = seq;
                            }
                            capture.done_reading();
                        }
                        1 => {
                            let _ = capture.has_frame();
                        }
                        _ => {
                            let mut stats = CaptureStats::default();
                            capture.fill_stats(&mut stats);
                            if stats.torn_dropped < last_torn {
                                backwards.fetch_add(1, Ordering::Relaxed);
                            }
                            last_torn = stats.torn_dropped;
                        }
                    }
                    drop(guard);
                    if stopped.load(Ordering::Acquire) && role == 0 {
                        after_stop.fetch_add(1, Ordering::Relaxed);
                    }
                    std::thread::sleep(Duration::from_micros(200));
                }
            }));
        }
        std::thread::sleep(STRESS_FOR);
        let capture = shared_in
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .take();
        if let Some(capture) = capture {
            capture.stop();
            stopped_in.store(true, Ordering::Release);
        }
        running_in.store(false, Ordering::Release);
        for thread in threads {
            thread.join().expect("a worker thread must not panic");
        }
    });
    assert!(reads.load(Ordering::Relaxed) > 0, "no frame was ever read");
    assert_eq!(backwards.load(Ordering::Relaxed), 0, "a counter went back");
    assert_eq!(
        after_stop.load(Ordering::Relaxed),
        0,
        "a frame was read after stop"
    );
    assert!(rig.window.close());
}

#[test]
#[ignore = "needs the PC: opens a cloaked window and captures it"]
fn a_failing_frame_callback_logs_once_and_capture_goes_on() {
    let _serial = SERIAL.lock().unwrap_or_else(PoisonError::into_inner);
    let _com = ComGuard::mta().expect("COM");
    log();
    let before = log_count(FRAME_FAILED);
    let window = CloakedWindow::open(640, 360, Reveal::ShownCloaked).expect("cloaked window");
    let target = window.target();
    let gpu = GpuDevice::for_target(&target).expect("GPU for the window");
    let armed = Armed::new().fail(Point::FrameCopyTexture, E_FAIL);
    let capture =
        WgcCapture::start(&gpu, &target, QpcMap::now(), OPTIONS).expect("capture starts");
    std::thread::sleep(FAILING_FOR);
    assert!(!capture.is_closed(), "the failing callback ended capture");
    assert!(!capture.has_frame(), "a failed copy published a frame");
    assert_eq!(
        log_count(FRAME_FAILED) - before,
        1,
        "a persistent error must log once per streak"
    );
    drop(armed);
    assert!(wait_for_frame(&capture), "capture did not recover");
    capture.stop();
    assert!(window.close());
}

#[test]
#[ignore = "needs the PC: opens a cloaked window and captures it"]
fn a_window_that_is_gone_gives_an_error_not_a_panic() {
    let _serial = SERIAL.lock().unwrap_or_else(PoisonError::into_inner);
    let _com = ComGuard::mta().expect("COM");
    let window = CloakedWindow::open(640, 360, Reveal::ShownCloaked).expect("cloaked window");
    let target = window.target();
    let gpu = GpuDevice::for_target(&target).expect("GPU for the window");
    assert!(window.close());
    let started = WgcCapture::start(&gpu, &target, QpcMap::now(), OPTIONS);
    assert!(started.is_err(), "a destroyed window was captured");
}
