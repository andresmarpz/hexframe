use std::fmt::Write;

use super::Report;
use super::diag::DiagReport;

const HISTOGRAM_BAR: usize = 30;

macro_rules! line {
    ($out:expr, $($arg:tt)*) => {
        writeln!($out, $($arg)*).ok()
    };
}

macro_rules! put {
    ($out:expr, $($arg:tt)*) => {
        write!($out, $($arg)*).ok()
    };
}

pub(super) fn text(report: &Report) -> String {
    let mut out = String::new();
    let file = &report.file;
    line!(out, "Greyscreen recording analysis");
    line!(out, "file:    {}", file.path.display());
    line!(
        out,
        "size:    {:.1} MB, {}x{} at {} fps target, {}",
        file.bytes as f64 / 1_000_000.0,
        file.width,
        file.height,
        file.target_fps,
        if file.finalized {
            "finalized"
        } else {
            "not finalized"
        }
    );
    line!(out, "length:  {}", clock(report.video.duration_secs));
    line!(out, "\nOVERALL: {}\n", report.overall.label());
    for verdict in &report.verdicts {
        line!(
            out,
            "[{:<7}] {:<22} {}",
            verdict.level.label(),
            verdict.area,
            verdict.text
        );
    }
    details(&mut out, report);
    out
}

fn details(out: &mut String, report: &Report) {
    let video = &report.video;
    line!(
        out,
        "\nFrame intervals (nominal {} fps):",
        video.nominal_fps
    );
    let total = video.frames.max(1);
    for bucket in &video.histogram {
        let bar = (bucket.count as f64 / total as f64 * HISTOGRAM_BAR as f64).round() as usize;
        line!(
            out,
            "  {:<15} {:>9}  {}",
            bucket.label,
            bucket.count,
            "#".repeat(bar.max(usize::from(bucket.count > 0)))
        );
    }
    if !video.gaps.listed.is_empty() {
        line!(
            out,
            "\nVideo gaps (showing {} of {}):",
            video.gaps.listed.len(),
            video.gaps.count
        );
        for gap in &video.gaps.listed {
            line!(
                out,
                "  at {}  {:>6.0} ms  {} frames missed",
                clock(gap.at_secs),
                gap.length_ms,
                gap.missing_frames
            );
        }
    }
    if let Some(audio) = &report.audio
        && !audio.gaps.is_empty()
    {
        line!(
            out,
            "\nAudio gaps (showing {} of {}):",
            audio.gaps.len(),
            audio.gap_count
        );
        for gap in &audio.gaps {
            line!(
                out,
                "  at {}  {:>6.0} ms",
                clock(gap.at_secs),
                gap.length_ms
            );
        }
    }
    bitrate_table(out, report);
    if let Some(diag) = &report.diagnostics {
        diagnostics(out, diag);
    }
    if report.health.is_none() {
        line!(out, "\n{}.", greyscreen_core::health::no_health_text());
    }
    if report.diagnostics.is_none() {
        line!(
            out,
            "No diagnostics.jsonl (turn on Detailed recording diagnostics in Settings to get one)."
        );
    }
}

fn bitrate_table(out: &mut String, report: &Report) {
    let rate = &report.video.bitrate;
    if rate.windows_kbps.len() < 2 {
        return;
    }
    line!(out, "\nBitrate per {} s (Mbps): ", rate.window_secs);
    let mut line = String::from("  ");
    for (index, kbps) in rate.windows_kbps.iter().enumerate() {
        put!(line, "{:.1} ", f64::from(*kbps) / 1000.0);
        if (index + 1) % 12 == 0 {
            line!(out, "{}", line.trim_end());
            line = String::from("  ");
        }
    }
    if !line.trim().is_empty() {
        line!(out, "{}", line.trim_end());
    }
}

fn diagnostics(out: &mut String, diag: &DiagReport) {
    line!(out, "\nDiagnostics ({} seconds):", diag.seconds);
    if let Some(setup) = &diag.setup {
        let text = |key: &str| setup[key].as_str().unwrap_or("?").to_owned();
        let number = |key: &str| setup[key].as_u64().unwrap_or(0);
        line!(
            out,
            "  GPU {} (driver {}), display {}x{} at {} Hz, window {}x{}, encode {}x{}",
            text("adapter"),
            text("driver_version"),
            number("display_width"),
            number("display_height"),
            number("display_hz"),
            number("window_width"),
            number("window_height"),
            number("encode_width"),
            number("encode_height"),
        );
    }
    if let (Some(avg), Some(max)) = (diag.cpu_pct_avg, diag.cpu_pct_max) {
        line!(
            out,
            "  Greyscreen CPU {avg:.0}% average, {max:.0}% peak (100% is one core)"
        );
    }
    if let Some(gpu) = &diag.gpu {
        let encoder = match (gpu.encode_avg, gpu.encode_peak) {
            (Some(avg), Some(peak)) => format!(", encoder {avg:.0}% average, {peak:.0}% peak"),
            _ => String::new(),
        };
        line!(
            out,
            "  GPU 3D busy {:.0}% average, {:.0}% peak, {} of {} seconds at 95% or more; Greyscreen's own 3D share {:.0}% average, {:.0}% peak{}; seconds with drops: {} of {} at 95% or more",
            gpu.busy_3d_avg,
            gpu.busy_3d_peak,
            gpu.seconds_3d_saturated,
            gpu.seconds,
            gpu.own_3d_avg,
            gpu.own_3d_peak,
            encoder,
            gpu.drop_seconds_3d_saturated,
            gpu.drop_seconds,
        );
    }
    if let Some(mem) = diag.mem_mb_max {
        line!(out, "  Greyscreen memory peak {mem} MB");
    }
    if diag.seconds_with_focus_info > 0 {
        line!(
            out,
            "  Game not in front for {} of {} seconds; seconds with drops: {} in the background, {} in front",
            diag.seconds_in_background,
            diag.seconds_with_focus_info,
            diag.drop_seconds_in_background,
            diag.drop_seconds_in_foreground,
        );
    }
}

pub(super) fn clock(secs: f64) -> String {
    let total = secs.max(0.0);
    let whole = total as u64;
    let millis = ((total - whole as f64) * 1000.0).round() as u64 % 1000;
    let (hours, minutes, seconds) = (whole / 3600, whole / 60 % 60, whole % 60);
    if hours > 0 {
        format!("{hours}:{minutes:02}:{seconds:02}.{millis:03}")
    } else {
        format!("{minutes:02}:{seconds:02}.{millis:03}")
    }
}
