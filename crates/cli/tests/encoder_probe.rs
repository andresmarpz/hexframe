#![cfg(windows)]
#![allow(clippy::expect_used)]

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use greyscreen_capture::WindowsMediaSource;
use greyscreen_core::encoder::{PROBE_CRASH_FLAG, PROBE_FLAG};
use greyscreen_core::media::{CaptureConfig, MediaSource};
use greyscreen_core::settings::Settings;
use greyscreen_engine::{ChildRunner, ProbeResult, ProbeRunner};

const DEADLINE: Duration = Duration::from_secs(10);

fn config() -> CaptureConfig {
    Settings::default()
        .capture_config(1080)
        .for_encode_size(1920, 1080)
}

fn exe() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_greyscreen-cli"))
}

#[test]
fn a_crashing_child_is_reported_and_the_next_probe_still_runs() {
    let media: Arc<dyn MediaSource> = Arc::new(WindowsMediaSource::new());
    let probes = media.encoder_probes();
    let first = probes.first().expect("at least one encoder probe");
    let cfg = config();

    let crashing = ChildRunner::with_args(
        media.clone(),
        exe(),
        vec![PROBE_FLAG.to_owned(), PROBE_CRASH_FLAG.to_owned()],
        DEADLINE,
    );
    let started = Instant::now();
    let result = crashing.run(&cfg, first);
    assert!(
        matches!(result, ProbeResult::Crashed(_)),
        "expected a crash, got {result:?}"
    );
    assert!(started.elapsed() < DEADLINE);

    let normal = ChildRunner::new(media, exe(), DEADLINE);
    let next = normal.run(&cfg, probes.get(1).unwrap_or(first));
    assert!(
        matches!(next, ProbeResult::Passed(_) | ProbeResult::Failed(_)),
        "expected an answer after the crash, got {next:?}"
    );
}

#[test]
fn child_results_match_the_in_process_results() {
    let media = Arc::new(WindowsMediaSource::new());
    let runner = ChildRunner::new(media.clone(), exe(), DEADLINE);
    let cfg = config();
    for probe in media.encoder_probes() {
        let in_process = media.probe_encoder(&cfg, &probe);
        let child = runner.run(&cfg, &probe);
        eprintln!(
            "{} {:?}: in process {in_process:?}, child {child:?}",
            probe.adapter, probe.kind
        );
        match child {
            ProbeResult::Passed(_) => assert!(in_process.is_ok(), "{}", probe.adapter),
            ProbeResult::Failed(_) => assert!(in_process.is_err(), "{}", probe.adapter),
            other => panic!("{} child gave {other:?}", probe.adapter),
        }
    }
}
