use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use crossbeam_channel::Sender;
use greyscreen_engine::EngineCmd;

use super::Soak;
use super::check::Verdict;

pub(super) const CLIP_FAILED_NOTICE: &str = "A clip could not be saved";
const CLIPS_DIR: &str = "clips";
pub(super) const RECORDING_WAIT: Duration = Duration::from_secs(30);
const CLIP_WAIT: Duration = Duration::from_secs(20);
const CLIP_AFTER: Duration = Duration::from_millis(500);
const CLIP_ATTEMPTS: u32 = 10;
const CLIP_SKIPPED_NOTICES: [&str; 2] = ["Nothing is recording", "A clip was skipped"];

pub(super) fn is_clip_skipped(text: &str) -> bool {
    CLIP_SKIPPED_NOTICES.iter().any(|n| text.starts_with(n))
}
const POLL: Duration = Duration::from_millis(200);

impl Soak<'_> {
    pub(super) fn fail_a_clip(&mut self, cmd: &Sender<EngineCmd>, known: &HashSet<PathBuf>) {
        if !self.wait_for(RECORDING_WAIT, |soak| {
            soak.recording && soak.block_clips_folder(known)
        }) {
            return;
        }
        self.pump(Instant::now() + CLIP_AFTER, false);
        let before = self.clip_saves_failed;
        for _ in 0..CLIP_ATTEMPTS {
            let skipped = self.clip_saves_skipped;
            if !self.send(cmd, EngineCmd::SaveClipNow) {
                return;
            }
            self.wait_for(CLIP_WAIT, |soak| {
                soak.clip_saves_failed > before || soak.clip_saves_skipped > skipped
            });
            if self.clip_saves_failed > before || !self.recording {
                return;
            }
            self.pump(Instant::now() + CLIP_AFTER, false);
        }
    }

    pub(super) fn judge_extras(&self, verdict: &mut Verdict) {
        if self.gpu.encoder_sessions_live > 0 {
            verdict.failures.push(format!(
                "{} encoder sessions still open after the engine stopped",
                self.gpu.encoder_sessions_live
            ));
        }
        if self.args.failed_clip_saves && self.clip_saves_failed < self.args.cycles {
            verdict.failures.push(format!(
                "{} of {} planned clip saves failed, the rest did not report a failure",
                self.clip_saves_failed, self.args.cycles
            ));
        }
        verdict
            .failures
            .extend(self.window_failures.iter().cloned());
        verdict.passed = verdict.failures.is_empty();
    }

    pub(super) fn wait_for(&mut self, limit: Duration, done: impl Fn(&Self) -> bool) -> bool {
        let deadline = Instant::now() + limit;
        while !done(self) {
            let now = Instant::now();
            if self.stopped || now >= deadline {
                return false;
            }
            self.pump((now + POLL).min(deadline), false);
        }
        true
    }

    fn block_clips_folder(&self, known: &HashSet<PathBuf>) -> bool {
        let Ok(entries) = fs::read_dir(&self.output_dir) else {
            return false;
        };
        let fresh = entries
            .filter_map(Result::ok)
            .filter(|e| e.metadata().is_ok_and(|meta| meta.is_dir()))
            .map(|e| e.path())
            .find(|path| !known.contains(path) && !self.saved_folders.contains(path));
        fresh.is_some_and(|folder| fs::write(folder.join(CLIPS_DIR), b"").is_ok())
    }

    pub(super) fn recording_folders(&self) -> HashSet<PathBuf> {
        fs::read_dir(&self.output_dir)
            .into_iter()
            .flatten()
            .filter_map(Result::ok)
            .map(|e| e.path())
            .collect()
    }
}
