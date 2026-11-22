use serde::Serialize;

const BYTES_PER_MB: f64 = 1024.0 * 1024.0;
const MIN_POINTS: usize = 10;
const MAX_WINDOW: usize = 10;

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub(crate) struct Sample {
    pub(crate) t_secs: f64,
    pub(crate) private_bytes: Option<u64>,
    pub(crate) handles: Option<u32>,
    pub(crate) threads: Option<u32>,
    pub(crate) gpu_bytes: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub(crate) struct Limits {
    pub(crate) mem_mb_per_min: f64,
    pub(crate) gpu_mb_per_min: f64,
    pub(crate) handle_growth: u32,
    pub(crate) thread_growth: u32,
    pub(crate) warmup_secs: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub(crate) struct CycleResult {
    pub(crate) index: u32,
    pub(crate) folder: String,
    pub(crate) video_secs: f64,
    pub(crate) dropped_pct: f64,
    pub(crate) capture_errors: u32,
    pub(crate) problem: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub(crate) struct Verdict {
    pub(crate) passed: bool,
    pub(crate) failures: Vec<String>,
    pub(crate) notes: Vec<String>,
}

pub(crate) struct Run<'a> {
    pub(crate) samples: &'a [Sample],
    pub(crate) cycles: &'a [CycleResult],
    pub(crate) engine_errors: &'a [String],
    pub(crate) min_saved: u32,
    pub(crate) require_usage: bool,
}

pub(crate) fn slope_per_min(points: &[(f64, f64)]) -> Option<f64> {
    if points.len() < 2 {
        return None;
    }
    let n = points.len() as f64;
    let mean_t = points.iter().map(|p| p.0).sum::<f64>() / n;
    let mean_v = points.iter().map(|p| p.1).sum::<f64>() / n;
    let spread: f64 = points.iter().map(|p| (p.0 - mean_t).powi(2)).sum();
    if spread <= f64::EPSILON {
        return None;
    }
    let cross: f64 = points.iter().map(|p| (p.0 - mean_t) * (p.1 - mean_v)).sum();
    Some(cross / spread * 60.0)
}

fn median(values: &[f64]) -> f64 {
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let mid = sorted.len() / 2;
    if sorted.len().is_multiple_of(2) {
        (sorted[mid - 1] + sorted[mid]) / 2.0
    } else {
        sorted[mid]
    }
}

pub(crate) fn growth(values: &[f64]) -> Option<f64> {
    if values.len() < 2 {
        return None;
    }
    let window = (values.len() / 4).clamp(1, MAX_WINDOW);
    let first = median(&values[..window]);
    let last = median(&values[values.len() - window..]);
    Some(last - first)
}

fn series(samples: &[Sample], warmup: f64, pick: fn(&Sample) -> Option<f64>) -> Vec<(f64, f64)> {
    samples
        .iter()
        .filter(|s| s.t_secs >= warmup)
        .filter_map(|s| pick(s).map(|v| (s.t_secs, v)))
        .collect()
}

fn megabytes(bytes: Option<u64>) -> Option<f64> {
    bytes.map(|b| b as f64 / BYTES_PER_MB)
}

#[derive(Default)]
struct Judge {
    failures: Vec<String>,
    notes: Vec<String>,
}

impl Judge {
    fn slope(&mut self, name: &str, points: &[(f64, f64)], limit: f64) {
        if points.is_empty() {
            return;
        }
        if points.len() < MIN_POINTS {
            self.notes.push(format!(
                "{name}: only {} samples after warmup, slope not checked",
                points.len()
            ));
            return;
        }
        if let Some(slope) = slope_per_min(points)
            && slope > limit
        {
            self.failures.push(format!(
                "{name} grows {slope:.2} MB/min, limit {limit:.2} MB/min"
            ));
        }
    }

    fn growth(&mut self, name: &str, points: &[(f64, f64)], limit: u32) {
        if points.len() < MIN_POINTS {
            return;
        }
        let values: Vec<f64> = points.iter().map(|p| p.1).collect();
        if let Some(delta) = growth(&values)
            && delta > f64::from(limit)
        {
            self.failures
                .push(format!("{name} grew by {delta:.0}, limit {limit}"));
        }
    }
}

pub(crate) fn evaluate(run: &Run, limits: &Limits) -> Verdict {
    let mut judge = Judge::default();
    let warmup = limits.warmup_secs;
    let private = series(run.samples, warmup, |s| megabytes(s.private_bytes));
    let gpu = series(run.samples, warmup, |s| megabytes(s.gpu_bytes));
    let handles = series(run.samples, warmup, |s| s.handles.map(f64::from));
    let threads = series(run.samples, warmup, |s| s.threads.map(f64::from));
    if run.require_usage && private.is_empty() {
        judge
            .failures
            .push("no process memory samples after warmup".to_owned());
    }
    judge.slope("private memory", &private, limits.mem_mb_per_min);
    judge.slope("GPU dedicated memory", &gpu, limits.gpu_mb_per_min);
    judge.growth("handles", &handles, limits.handle_growth);
    judge.growth("threads", &threads, limits.thread_growth);
    if (run.cycles.len() as u32) < run.min_saved {
        judge.failures.push(format!(
            "{} recordings saved, expected {}",
            run.cycles.len(),
            run.min_saved
        ));
    }
    for cycle in run.cycles {
        if let Some(problem) = &cycle.problem {
            judge
                .failures
                .push(format!("recording {}: {problem}", cycle.index));
        }
        if cycle.capture_errors > 0 {
            judge.failures.push(format!(
                "recording {}: {} capture errors",
                cycle.index, cycle.capture_errors
            ));
        }
    }
    if let Some(first) = run.engine_errors.first() {
        judge.failures.push(format!(
            "engine reported {} errors, first: {first}",
            run.engine_errors.len()
        ));
    }
    Verdict {
        passed: judge.failures.is_empty(),
        failures: judge.failures,
        notes: judge.notes,
    }
}
