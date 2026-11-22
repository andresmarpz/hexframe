mod reader;
mod writer;

use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use greyscreen_core::export::EncodePlan;

use crate::com::{ComGuard, MfGuard};
use reader::{Item, Source};
use writer::Sink;

const HNS_PER_SEC: f64 = 10_000_000.0;
const FRAME_JITTER_HNS: i64 = 10_000;
const PROGRESS_STEP: f32 = 0.01;

#[derive(Debug, Clone, Copy)]
pub struct Request<'a> {
    pub src: &'a Path,
    pub dst: &'a Path,
    pub start_secs: f64,
    pub end_secs: f64,
    pub plan: EncodePlan,
}

#[derive(Debug, thiserror::Error)]
pub enum TranscodeError {
    #[error("cancelled")]
    Cancelled,
    #[error("{0}")]
    Failed(String),
}

pub(crate) fn failed(context: &str, err: &windows::core::Error) -> TranscodeError {
    TranscodeError::Failed(format!(
        "{context}: {:#010x} {}",
        err.code().0,
        err.message()
    ))
}

pub fn transcode(
    request: &Request<'_>,
    progress: &mut dyn FnMut(f32),
    cancel: &AtomicBool,
) -> Result<(), TranscodeError> {
    let _com = ComGuard::mta().map_err(|err| failed("COM", &err))?;
    let _media = MfGuard::start().map_err(|err| failed("Media Foundation", &err))?;
    match run(request, true, progress, cancel) {
        Err(TranscodeError::Failed(why)) => {
            tracing::warn!(%why, "hardware export failed, trying the software encoder");
            fs::remove_file(request.dst).ok();
            progress(0.0);
            run(request, false, progress, cancel)
        }
        other => other,
    }
}

fn hns(secs: f64) -> i64 {
    (secs.max(0.0) * HNS_PER_SEC) as i64
}

fn run(
    request: &Request<'_>,
    hardware: bool,
    progress: &mut dyn FnMut(f32),
    cancel: &AtomicBool,
) -> Result<(), TranscodeError> {
    let plan = request.plan;
    let (start, end) = (hns(request.start_secs), hns(request.end_secs));
    if end <= start {
        return Err(TranscodeError::Failed("the range is empty".into()));
    }
    let mut source = Source::open(request.src, start)?;
    let sink = Sink::create(request.dst, &plan, source.audio(), hardware)?;
    let width = plan.width as usize;
    let height = plan.height as usize;
    let frame_hns = (HNS_PER_SEC / f64::from(plan.fps.max(1))) as i64;
    let min_gap = frame_hns - FRAME_JITTER_HNS;
    let mut frame = Vec::new();
    let mut last_video: Option<i64> = None;
    let (mut video_done, mut audio_done) = (false, !source.has_audio());
    let mut reported = 0.0f32;
    while !(video_done && audio_done) {
        if cancel.load(Ordering::SeqCst) {
            return Err(TranscodeError::Cancelled);
        }
        match source.next()? {
            Item::End => break,
            Item::Video { sample, time } => {
                if video_done || time < start {
                    continue;
                }
                if time >= end {
                    video_done = true;
                    continue;
                }
                let at = time - start;
                if last_video.is_some_and(|last| at - last < min_gap) {
                    continue;
                }
                source.scale_into(&sample, width, height, &mut frame)?;
                sink.write_video(&frame, at, frame_hns)?;
                last_video = Some(at);
                let fraction = (at as f32 / (end - start) as f32).clamp(0.0, 0.99);
                if fraction - reported >= PROGRESS_STEP {
                    reported = fraction;
                    progress(fraction);
                }
            }
            Item::Audio { sample, time } => {
                if audio_done || time < start {
                    continue;
                }
                if time >= end {
                    audio_done = true;
                    continue;
                }
                sink.write_audio(&sample, time - start)?;
            }
        }
    }
    if last_video.is_none() {
        return Err(TranscodeError::Failed(
            "no video frames in the range".into(),
        ));
    }
    sink.finish()?;
    progress(1.0);
    Ok(())
}

#[cfg(test)]
mod tests;
