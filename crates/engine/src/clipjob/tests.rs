use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;

use super::*;

fn job() -> ClipJob {
    ClipJob {
        path: PathBuf::from("clips/a.mp4"),
        entry: ClipEntry {
            file: "clips/a.mp4".to_owned(),
            start_secs: 0.0,
            end_secs: 1.0,
            label: "Manual".to_owned(),
        },
        video: VideoFormat {
            width: 2,
            height: 2,
            fps: 60,
            sps: Vec::new(),
            pps: Vec::new(),
        },
        audio: None,
        packets: Vec::new(),
    }
}

fn run(write: impl FnOnce(&ClipJob) -> ClipResult + Send + 'static) -> ClipResult {
    let (tx, rx) = mpsc::channel();
    assert!(spawn_with(job(), write, move |result| {
        tx.send(result).ok();
    }));
    rx.recv_timeout(Duration::from_secs(5))
        .expect("on_done ran")
}

#[test]
fn a_panic_in_the_clip_job_reports_a_failure_not_a_clip() {
    let result = run(|_| panic!("encoder blew up"));
    assert_eq!(result, Err("clip writer panicked".to_owned()));
}

#[test]
fn a_finished_clip_job_reports_its_entry() {
    let result = run(|job| Ok(job.entry.clone()));
    assert_eq!(result.expect("entry").file, "clips/a.mp4");
}
