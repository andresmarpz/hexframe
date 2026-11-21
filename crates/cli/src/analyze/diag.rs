use std::path::Path;

use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, Serialize)]
pub(crate) struct DiagReport {
    pub seconds: u64,
    pub setup: Option<Value>,
    pub seconds_with_drops: u64,
    pub dropped_frames: u64,
    pub repeated_frames: u64,
    pub encode_ms_max: f64,
    pub step_ms_max: f64,
    pub queue_writer_max: u64,
    pub queue_media_max: u64,
    pub queue_encoder_max: u64,
    pub av_skew_ms_max: f64,
    pub cpu_pct_max: Option<f64>,
    pub cpu_pct_avg: Option<f64>,
    pub mem_mb_max: Option<u64>,
    pub bad_lines: u64,
}

pub(crate) fn read(path: &Path) -> Option<DiagReport> {
    let text = std::fs::read_to_string(path).ok()?;
    Some(summarize(&text))
}

pub(crate) fn summarize(text: &str) -> DiagReport {
    let mut report = DiagReport {
        seconds: 0,
        setup: None,
        seconds_with_drops: 0,
        dropped_frames: 0,
        repeated_frames: 0,
        encode_ms_max: 0.0,
        step_ms_max: 0.0,
        queue_writer_max: 0,
        queue_media_max: 0,
        queue_encoder_max: 0,
        av_skew_ms_max: 0.0,
        cpu_pct_max: None,
        cpu_pct_avg: None,
        mem_mb_max: None,
        bad_lines: 0,
    };
    let mut cpu_sum = 0.0;
    let mut cpu_n = 0u32;
    for line in text.lines().filter(|l| !l.trim().is_empty()) {
        let Ok(value) = serde_json::from_str::<Value>(line) else {
            report.bad_lines += 1;
            continue;
        };
        match value["type"].as_str() {
            Some("setup") => report.setup = Some(value),
            Some("second") => add_second(&mut report, &value, &mut cpu_sum, &mut cpu_n),
            _ => report.bad_lines += 1,
        }
    }
    if cpu_n > 0 {
        report.cpu_pct_avg = Some(cpu_sum / f64::from(cpu_n));
    }
    report
}

fn add_second(report: &mut DiagReport, line: &Value, cpu_sum: &mut f64, cpu_n: &mut u32) {
    report.seconds += 1;
    let dropped = line["dropped"].as_u64().unwrap_or(0);
    report.dropped_frames += dropped;
    report.seconds_with_drops += u64::from(dropped > 0);
    report.repeated_frames += line["repeated"].as_u64().unwrap_or(0);
    report.encode_ms_max = report
        .encode_ms_max
        .max(line["encode_ms_max"].as_f64().unwrap_or(0.0));
    report.step_ms_max = report
        .step_ms_max
        .max(line["step_ms_max"].as_f64().unwrap_or(0.0));
    report.queue_writer_max = report
        .queue_writer_max
        .max(line["queue_writer"].as_u64().unwrap_or(0));
    report.queue_media_max = report
        .queue_media_max
        .max(line["queue_media"].as_u64().unwrap_or(0));
    report.queue_encoder_max = report
        .queue_encoder_max
        .max(line["queue_encoder"].as_u64().unwrap_or(0));
    report.av_skew_ms_max = report
        .av_skew_ms_max
        .max(line["av_skew_ms"].as_f64().unwrap_or(0.0).abs());
    if let Some(cpu) = line["cpu_pct"].as_f64() {
        report.cpu_pct_max = Some(report.cpu_pct_max.unwrap_or(0.0).max(cpu));
        *cpu_sum += cpu;
        *cpu_n += 1;
    }
    if let Some(mem) = line["mem_mb"].as_u64() {
        report.mem_mb_max = Some(report.mem_mb_max.unwrap_or(0).max(mem));
    }
}
