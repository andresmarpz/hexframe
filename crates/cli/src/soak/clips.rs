use std::fs;
use std::time::{Duration, Instant};

use crossbeam_channel::Sender;
use greyscreen_engine::EngineCmd;

use super::Soak;
use super::check::Verdict;

pub(super) const CLIP_FAILED_NOTICE: &str = "A clip could not be saved";
const CLIPS_DIR: &str = "clips";
const RECORDING_WAIT: Duration = Duration::from_secs(30);
const CLIP_WAIT: Duration = Duration::from_secs(20);
const CLIP_AFTER: Duration = Duration::from_secs(2);
const POLL: Duration = Duration::from_millis(200);

impl Soak<'_> {
    pub(super) fn fail_a_clip(&mut self, cmd: &Sender<EngineCmd>) {
        if !self.wait_for(RECORDING_WAIT, |soak| soak.recording) {
            return;
        }
        self.pump(Instant::now() + CLIP_AFTER, false);
        if !self.block_clips_folder() {
            return;
        }
        let before = self.clip_saves_failed;
        if self.send(cmd, EngineCmd::SaveClipNow) {
            self.wait_for(CLIP_WAIT, |soak| soak.clip_saves_failed > before);
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
        verdict.passed = verdict.failures.is_empty();
    }

    fn wait_for(&mut self, limit: Duration, done: impl Fn(&Self) -> bool) -> bool {
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

    fn block_clips_folder(&self) -> bool {
        let Ok(entries) = fs::read_dir(&self.output_dir) else {
            return false;
        };
        let newest = entries
            .filter_map(Result::ok)
            .filter_map(|e| Some((e.metadata().ok()?, e.path())))
            .filter(|(meta, _)| meta.is_dir())
            .filter_map(|(meta, path)| Some((meta.modified().ok()?, path)))
            .max();
        newest.is_some_and(|(_, folder)| fs::write(folder.join(CLIPS_DIR), b"").is_ok())
    }
}
