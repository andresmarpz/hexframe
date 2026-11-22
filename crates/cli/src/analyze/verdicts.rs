use greyscreen_core::health::verdict::{
    AUDIO_GAP_BAD_MS, AUDIO_GAP_WARN_MS, DRIFT_BAD_MS, DRIFT_WARN_MS, DROPPED_BAD_PCT,
    DROPPED_WARN_PCT, GAP_BAD_MS, GAP_WARN_MS, MIN_BITRATE_KBPS,
};
use greyscreen_core::health::{Level, RecordingHealth};
use serde::Serialize;

use super::FileReport;
use super::audio::AudioReport;
use super::diag::DiagReport;
use super::video::VideoReport;

const KEYFRAME_WARN_SECS: f64 = 5.0;
const QUEUE_WARN: u64 = 1_000;
const MIN_SECS_FOR_BITRATE: f64 = 30.0;
const MIN_HEIGHT_FOR_BITRATE: u32 = 720;
const VBV_TOLERANCE: f64 = 1.1;

#[derive(Debug, Clone, Serialize)]
pub(crate) struct Verdict {
    pub area: String,
    pub level: Level,
    pub text: String,
}

fn verdict(area: &str, level: Level, text: String) -> Verdict {
    Verdict {
        area: area.to_owned(),
        level,
        text,
    }
}

pub(super) fn collect(
    file: &FileReport,
    video: &VideoReport,
    audio: Option<&AudioReport>,
    health: Option<&RecordingHealth>,
    diag: Option<&DiagReport>,
) -> Vec<Verdict> {
    let mut out = vec![
        container(file),
        frames(video),
        gaps(video),
        keyframes(video),
        bitrate(video, file.height),
    ];
    out.extend(vbv(video, diag));
    if let Some(audio) = audio {
        out.push(audio_continuity(audio));
        out.push(audio_sync(audio));
    }
    if let Some(health) = health {
        out.extend(
            health
                .findings()
                .into_iter()
                .map(|f| verdict(&format!("Health: {}", f.area.label()), f.level, f.text)),
        );
        out.extend(
            health
                .warnings
                .iter()
                .map(|text| verdict("Health: Setup", Level::Warn, text.clone())),
        );
    }
    if let Some(diag) = diag {
        out.extend(diagnostics(diag, file.target_fps));
    }
    out
}

fn container(file: &FileReport) -> Verdict {
    match (file.finalized, file.fragments, file.trailing_bytes) {
        (true, ..) => verdict("File", Level::Ok, "finalized MP4, index intact".to_owned()),
        (false, Some(fragments), Some(trailing)) if trailing > 0 => verdict(
            "File",
            Level::Bad,
            format!(
                "truncated: {fragments} complete fragments, {trailing} bytes cut off at the end"
            ),
        ),
        (false, fragments, _) => verdict(
            "File",
            Level::Warn,
            format!(
                "not finalized ({} fragments): the recording crashed or is still running; Greyscreen repairs it at the next start",
                fragments.unwrap_or(0)
            ),
        ),
    }
}

fn frames(video: &VideoReport) -> Verdict {
    let level = Level::grade(video.missing_pct, DROPPED_WARN_PCT, DROPPED_BAD_PCT);
    verdict(
        "Video frames",
        level,
        format!(
            "{} frames, {:.2} fps average (target {}), {} missing ({:.2}%)",
            video.frames,
            video.average_fps,
            video.nominal_fps,
            video.missing_frames,
            video.missing_pct
        ),
    )
}

fn gaps(video: &VideoReport) -> Verdict {
    let level = Level::grade(video.gaps.longest_ms, GAP_WARN_MS, GAP_BAD_MS);
    let text = if video.gaps.count == 0 {
        "no gap longer than 2 frame intervals".to_owned()
    } else {
        format!(
            "{} gaps longer than 2 frame intervals, longest {:.0} ms",
            video.gaps.count, video.gaps.longest_ms
        )
    };
    verdict("Video gaps", level, text)
}

fn keyframes(video: &VideoReport) -> Verdict {
    let keys = &video.keyframes;
    let level = if keys.count == 0 {
        Level::Bad
    } else if keys.max_secs > KEYFRAME_WARN_SECS {
        Level::Warn
    } else {
        Level::Ok
    };
    verdict(
        "Keyframes",
        level,
        format!(
            "{} keyframes, interval {:.2} s average, {:.2} s longest",
            keys.count, keys.avg_secs, keys.max_secs
        ),
    )
}

fn bitrate(video: &VideoReport, height: u32) -> Verdict {
    let rate = &video.bitrate;
    let low = rate.avg_kbps < MIN_BITRATE_KBPS && height >= MIN_HEIGHT_FOR_BITRATE;
    let level = if low && video.duration_secs > MIN_SECS_FOR_BITRATE {
        Level::Warn
    } else {
        Level::Ok
    };
    verdict(
        "Bitrate",
        level,
        format!(
            "{:.1} Mbps average, {:.1} to {:.1} Mbps per {} s",
            f64::from(rate.avg_kbps) / 1000.0,
            f64::from(rate.min_kbps) / 1000.0,
            f64::from(rate.max_kbps) / 1000.0,
            rate.window_secs
        ),
    )
}

pub(super) fn vbv(video: &VideoReport, diag: Option<&DiagReport>) -> Option<Verdict> {
    let cap_kbps = diag?
        .setup
        .as_ref()?
        .get("peak")?
        .as_u64()
        .map(|bps| bps as f64 / 1000.0)?;
    let peak = f64::from(video.vbv.peak_kbps);
    let over = cap_kbps > 0.0 && peak > cap_kbps * VBV_TOLERANCE;
    let text = format!(
        "peak {:.1} Mbps over any 1 s (at {:.1} s) against a {:.1} Mbps cap{}",
        peak / 1000.0,
        video.vbv.at_secs,
        cap_kbps / 1000.0,
        if over {
            ": the encoder ignores the cap or reads the buffer size in another unit"
        } else {
            ""
        }
    );
    Some(verdict(
        "Bitrate cap",
        if over { Level::Warn } else { Level::Ok },
        text,
    ))
}

fn audio_continuity(audio: &AudioReport) -> Verdict {
    let level = Level::grade(audio.longest_gap_ms, AUDIO_GAP_WARN_MS, AUDIO_GAP_BAD_MS);
    verdict(
        "Audio continuity",
        level,
        format!(
            "{} packets, {} samples over {:.1} s, {} gaps (longest {:.0} ms)",
            audio.packets,
            audio.samples,
            audio.timeline_secs,
            audio.gap_count,
            audio.longest_gap_ms
        ),
    )
}

fn audio_sync(audio: &AudioReport) -> Verdict {
    let level = Level::grade(audio.av_diff_ms.abs(), DRIFT_WARN_MS, DRIFT_BAD_MS);
    verdict(
        "A/V duration",
        level,
        format!(
            "audio is {:+.0} ms against video at the end",
            audio.av_diff_ms
        ),
    )
}

fn diagnostics(diag: &DiagReport, fps: u32) -> Vec<Verdict> {
    let frame_ms = 1000.0 / f64::from(fps.max(1));
    let drops = if diag.seconds_with_drops == 0 {
        Level::Ok
    } else if diag.seconds_with_drops * 100 > diag.seconds.max(1) {
        Level::Bad
    } else {
        Level::Warn
    };
    let queues = if diag.queue_writer_max >= QUEUE_WARN {
        Level::Warn
    } else {
        Level::Ok
    };
    let encode = if diag.encode_ms_max > frame_ms * 4.0 {
        Level::Warn
    } else {
        Level::Ok
    };
    vec![
        verdict(
            "Diagnostics: drops",
            drops,
            format!(
                "{} seconds recorded, {} with dropped frames ({} frames), {} repeated frames",
                diag.seconds, diag.seconds_with_drops, diag.dropped_frames, diag.repeated_frames
            ),
        ),
        verdict(
            "Diagnostics: encoder",
            encode,
            format!(
                "slowest second: encode {:.1} ms, capture step {:.1} ms, GPU copy wait {:.1} ms; {} torn frames replaced",
                diag.encode_ms_max, diag.step_ms_max, diag.copy_wait_ms_max, diag.torn_frames
            ),
        ),
        verdict(
            "Diagnostics: queues",
            queues,
            format!(
                "deepest queues: writer {}, engine {}, encoder {}",
                diag.queue_writer_max, diag.queue_media_max, diag.queue_encoder_max
            ),
        ),
    ]
}
