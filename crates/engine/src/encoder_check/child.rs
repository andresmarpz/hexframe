use std::io::Read;
use std::path::PathBuf;
use std::process::{Command, ExitStatus, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use greyscreen_core::encoder::{
    PROBE_CRASH_FLAG, PROBE_FLAG, PROBE_SELF_DEADLINE_MS, Probe, ProbeLine, ProbeRequest,
};
use greyscreen_core::media::{CaptureConfig, MediaSource};

use super::runner::{ProbeResult, ProbeRunner};

const POLL: Duration = Duration::from_millis(20);

pub struct ChildRunner {
    media: Arc<dyn MediaSource>,
    exe: PathBuf,
    leading_args: Vec<String>,
    deadline: Duration,
}

impl std::fmt::Debug for ChildRunner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChildRunner")
            .field("exe", &self.exe)
            .finish_non_exhaustive()
    }
}

impl ChildRunner {
    pub fn new(media: Arc<dyn MediaSource>, exe: PathBuf, deadline: Duration) -> Self {
        Self::with_args(media, exe, vec![PROBE_FLAG.to_owned()], deadline)
    }

    pub fn with_args(
        media: Arc<dyn MediaSource>,
        exe: PathBuf,
        leading_args: Vec<String>,
        deadline: Duration,
    ) -> Self {
        Self {
            media,
            exe,
            leading_args,
            deadline,
        }
    }

    fn command(&self, cfg: &CaptureConfig, probe: &Probe) -> Command {
        let request = ProbeRequest {
            luid: probe.luid,
            kind: probe.kind,
            cfg: cfg.clone(),
        };
        let mut command = Command::new(&self.exe);
        command
            .args(&self.leading_args)
            .arg(request.to_arg())
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            command.creation_flags(CREATE_NO_WINDOW);
        }
        command
    }
}

impl ProbeRunner for ChildRunner {
    fn probes(&self) -> Vec<Probe> {
        self.media.encoder_probes()
    }

    fn run(&self, cfg: &CaptureConfig, probe: &Probe) -> ProbeResult {
        let mut child = match self.command(cfg, probe).spawn() {
            Ok(child) => child,
            Err(err) => return ProbeResult::Failed(format!("cannot start the probe: {err}")),
        };
        let reader = child.stdout.take().map(|mut out| {
            std::thread::spawn(move || {
                let mut text = String::new();
                out.read_to_string(&mut text).ok();
                text
            })
        });
        let started = Instant::now();
        let status = loop {
            match child.try_wait() {
                Ok(Some(status)) => break Some(status),
                Ok(None) if started.elapsed() >= self.deadline => break None,
                Ok(None) => std::thread::sleep(POLL),
                Err(err) => {
                    child.kill().ok();
                    child.wait().ok();
                    return ProbeResult::Crashed(format!("cannot wait for the probe: {err}"));
                }
            }
        };
        let Some(status) = status else {
            child.kill().ok();
            child.wait().ok();
            return ProbeResult::TimedOut(format!(
                "stopped after {} s",
                self.deadline.as_secs_f32()
            ));
        };
        let output = reader.and_then(|r| r.join().ok()).unwrap_or_default();
        classify(&output, status)
    }
}

fn classify(output: &str, status: ExitStatus) -> ProbeResult {
    match ProbeLine::parse(output) {
        Some(ProbeLine::Passed(detail)) => ProbeResult::Passed(detail),
        Some(ProbeLine::Failed(detail)) => ProbeResult::Failed(detail),
        Some(ProbeLine::TimedOut(detail)) => ProbeResult::TimedOut(detail),
        None if status.success() => ProbeResult::Crashed("the probe printed nothing".into()),
        None => ProbeResult::Crashed(format!("the probe ended abnormally: {status}")),
    }
}

pub fn run_probe_child(args: &[String], media: &dyn MediaSource) -> Option<i32> {
    if args.first().map(String::as_str) != Some(PROBE_FLAG) {
        return None;
    }
    if args.iter().any(|arg| arg == PROBE_CRASH_FLAG) {
        std::process::abort();
    }
    let Some(request) = args.get(1).and_then(|arg| ProbeRequest::from_arg(arg)) else {
        println!("{}", ProbeLine::Failed("bad probe request".into()).format());
        return Some(2);
    };
    let answered = Arc::new(AtomicBool::new(false));
    let guard = answered.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(PROBE_SELF_DEADLINE_MS));
        if !guard.swap(true, Ordering::SeqCst) {
            let line = ProbeLine::TimedOut(format!("no answer in {PROBE_SELF_DEADLINE_MS} ms"));
            println!("{}", line.format());
            std::process::exit(3);
        }
    });
    let line = match media.probe_encoder(&request.cfg, &request.probe()) {
        Ok(detail) => ProbeLine::Passed(detail),
        Err(error) => ProbeLine::Failed(error),
    };
    if answered.swap(true, Ordering::SeqCst) {
        loop {
            std::thread::park();
        }
    }
    println!("{}", line.format());
    Some(0)
}
