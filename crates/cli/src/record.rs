use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use crossbeam_channel::{Receiver, RecvTimeoutError, bounded};
use greyscreen_core::media::{
    AudioFormat, CaptureConfig, CaptureTarget, MediaEvent, MediaSession, MediaSource, Packet,
    Track, VideoFormat,
};
use greyscreen_core::settings::Settings;
use greyscreen_mp4::fragmented::FragmentedWriter;

const CHANNEL_CAPACITY: usize = 1024;
const EVENT_TIMEOUT: Duration = Duration::from_secs(15);
const DRAIN_POLL: Duration = Duration::from_millis(50);

#[derive(Debug)]
pub(crate) struct Args {
    pub(crate) seconds: f64,
    pub(crate) out: PathBuf,
    pub(crate) window: Option<String>,
    pub(crate) fps: u32,
    pub(crate) height: u32,
    pub(crate) audio: bool,
}

#[derive(Debug, Default)]
struct Tally {
    video: u64,
    audio: u64,
    warnings: Vec<String>,
}

impl Tally {
    fn count(&mut self, packet: &Packet) {
        match packet.track {
            Track::Video => self.video += 1,
            Track::Audio => self.audio += 1,
        }
    }
}

type Started = (VideoFormat, Option<AudioFormat>, String);

pub(crate) fn run(args: &Args) -> ExitCode {
    match record(args) {
        Ok(code) => code,
        Err(err) => {
            eprintln!("error: {err:#}");
            ExitCode::FAILURE
        }
    }
}

fn record(args: &Args) -> Result<ExitCode> {
    if !args.seconds.is_finite() || args.seconds <= 0.0 {
        bail!("--seconds must be a positive number");
    }
    #[cfg(windows)]
    crate::init_windows();
    let (source, target) = source_and_target(args)?;
    let mut cfg = Settings::default().capture_config(2160);
    cfg.fps = args.fps;
    cfg.max_height = Some(args.height);
    cfg.audio = args.audio;
    run_with(source.as_ref(), target, cfg, args)
}

fn fail(message: &str, part: Option<&Path>) -> Result<ExitCode> {
    println!("FAILED: {message}");
    if let Some(part) = part {
        println!("partial file kept: {}", part.display());
    }
    Ok(ExitCode::FAILURE)
}

fn run_with(
    source: &dyn MediaSource,
    target: CaptureTarget,
    cfg: CaptureConfig,
    args: &Args,
) -> Result<ExitCode> {
    let (tx, rx) = bounded(CHANNEL_CAPACITY);
    let session = source
        .start(target, cfg, tx)
        .map_err(|e| anyhow::anyhow!("capture failed to start: {e}"))?;
    let mut tally = Tally::default();
    let (video, audio, encoder) = match wait_started(&rx, &mut tally) {
        Ok(started) => started,
        Err(message) => {
            session.stop();
            return fail(&message, None);
        }
    };
    let part = part_path(&args.out);
    let mut writer = FragmentedWriter::create(&part, &video, audio.as_ref())
        .with_context(|| format!("creating {}", part.display()))?;
    let target_ticks = (args.seconds * f64::from(VideoFormat::TIMESCALE)) as i64;
    if let Err(message) = pump(&rx, &mut writer, &mut tally, target_ticks) {
        session.stop();
        return fail(&message, Some(&part));
    }
    if let Err(message) = stop_and_drain(session, &rx, &mut writer, &mut tally) {
        return fail(&message, Some(&part));
    }
    let info = writer.finish().context("finishing the recording")?;
    std::fs::rename(&part, &args.out)
        .with_context(|| format!("renaming to {}", args.out.display()))?;
    println!("encoder: {encoder}");
    println!(
        "video: {}x{} @ {} fps",
        video.width, video.height, video.fps
    );
    match &audio {
        Some(a) => println!(
            "audio: {} Hz, {} ch, {} bps",
            a.sample_rate, a.channels, a.bitrate
        ),
        None => println!("audio: none"),
    }
    println!("packets: {} video, {} audio", tally.video, tally.audio);
    println!("duration: {:.2} s", info.duration_secs);
    println!("bytes: {}", info.bytes);
    for warning in &tally.warnings {
        println!("warning: {warning}");
    }
    println!("wrote {}", args.out.display());
    Ok(ExitCode::SUCCESS)
}

fn wait_started(rx: &Receiver<MediaEvent>, tally: &mut Tally) -> Result<Started, String> {
    loop {
        match rx.recv_timeout(EVENT_TIMEOUT) {
            Ok(MediaEvent::Started {
                video,
                audio,
                encoder,
                ..
            }) => return Ok((video, audio, encoder)),
            Ok(MediaEvent::Warning(w)) => tally.warnings.push(w),
            Ok(MediaEvent::Failed(m)) => return Err(m),
            Ok(MediaEvent::Packet(_)) => return Err("packet before Started".into()),
            Err(_) => return Err("timed out waiting for Started".into()),
        }
    }
}

fn handle(
    event: MediaEvent,
    writer: &mut FragmentedWriter,
    tally: &mut Tally,
) -> Result<Option<i64>, String> {
    match event {
        MediaEvent::Packet(packet) => {
            writer
                .push(&packet)
                .map_err(|e| format!("writing packet: {e}"))?;
            tally.count(&packet);
            Ok((packet.track == Track::Video).then_some(packet.dts))
        }
        MediaEvent::Warning(w) => {
            tally.warnings.push(w);
            Ok(None)
        }
        MediaEvent::Failed(m) => Err(m),
        MediaEvent::Started { .. } => Ok(None),
    }
}

fn pump(
    rx: &Receiver<MediaEvent>,
    writer: &mut FragmentedWriter,
    tally: &mut Tally,
    target_ticks: i64,
) -> Result<(), String> {
    loop {
        let event = rx
            .recv_timeout(EVENT_TIMEOUT)
            .map_err(|_| "no events from the source for 15 s".to_owned())?;
        if let Some(dts) = handle(event, writer, tally)?
            && dts >= target_ticks
        {
            return Ok(());
        }
    }
}

fn stop_and_drain(
    session: Box<dyn MediaSession>,
    rx: &Receiver<MediaEvent>,
    writer: &mut FragmentedWriter,
    tally: &mut Tally,
) -> Result<(), String> {
    std::thread::scope(|scope| {
        let stopper = scope.spawn(move || session.stop());
        loop {
            match rx.recv_timeout(DRAIN_POLL) {
                Ok(event) => {
                    handle(event, writer, tally)?;
                }
                Err(RecvTimeoutError::Timeout) if !stopper.is_finished() => {}
                Err(_) => break,
            }
        }
        while let Ok(event) = rx.try_recv() {
            handle(event, writer, tally)?;
        }
        Ok(())
    })
}

fn part_path(out: &Path) -> PathBuf {
    let mut name: OsString = out.as_os_str().to_owned();
    name.push(".part");
    PathBuf::from(name)
}

#[cfg(not(windows))]
fn source_and_target(args: &Args) -> Result<(Box<dyn MediaSource>, CaptureTarget)> {
    if args.window.is_some() {
        bail!("--window needs Windows");
    }
    Ok((
        Box::new(greyscreen_engine::FakeMediaSource::new()),
        CaptureTarget::PrimaryMonitor,
    ))
}

#[cfg(windows)]
fn source_and_target(args: &Args) -> Result<(Box<dyn MediaSource>, CaptureTarget)> {
    let target = match &args.window {
        Some(title) => {
            let (hwnd, pid) = find_window(title)
                .with_context(|| format!("no visible window with title containing {title:?}"))?;
            println!("window: hwnd {hwnd:#x}, pid {pid}");
            CaptureTarget::GameWindow { hwnd, pid }
        }
        None => CaptureTarget::PrimaryMonitor,
    };
    Ok((
        Box::new(greyscreen_capture::WindowsMediaSource::new()),
        target,
    ))
}

#[cfg(windows)]
struct Search {
    needle: String,
    found: Option<(isize, u32)>,
}

#[cfg(windows)]
fn find_window(needle: &str) -> Option<(isize, u32)> {
    use windows::Win32::Foundation::LPARAM;
    use windows::Win32::UI::WindowsAndMessaging::EnumWindows;

    let mut search = Search {
        needle: needle.to_lowercase(),
        found: None,
    };
    // SAFETY: the callback only touches `search`, which outlives the synchronous call.
    unsafe {
        EnumWindows(
            Some(enum_proc),
            LPARAM(std::ptr::from_mut(&mut search) as isize),
        )
    }
    .ok();
    search.found
}

#[cfg(windows)]
unsafe extern "system" fn enum_proc(
    hwnd: windows::Win32::Foundation::HWND,
    lparam: windows::Win32::Foundation::LPARAM,
) -> windows::core::BOOL {
    use windows::Win32::UI::WindowsAndMessaging::{
        GetWindowTextW, GetWindowThreadProcessId, IsWindowVisible,
    };
    use windows::core::BOOL;

    // SAFETY: lparam is the `Search` pointer passed by `find_window` for this call.
    let search = unsafe { &mut *(lparam.0 as *mut Search) };
    // SAFETY: hwnd comes from EnumWindows.
    if !unsafe { IsWindowVisible(hwnd) }.as_bool() {
        return BOOL(1);
    }
    let mut buf = [0u16; 512];
    // SAFETY: the buffer is a live local slice.
    let len = unsafe { GetWindowTextW(hwnd, &mut buf) };
    let len = usize::try_from(len).unwrap_or(0);
    let title = String::from_utf16_lossy(&buf[..len]).to_lowercase();
    if len == 0 || !title.contains(&search.needle) {
        return BOOL(1);
    }
    let mut pid = 0u32;
    // SAFETY: `pid` is a live local that receives the process id.
    unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
    search.found = Some((hwnd.0 as isize, pid));
    BOOL(0)
}
