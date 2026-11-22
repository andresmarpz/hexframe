use std::time::{Duration, Instant};

use crossbeam_channel::Sender;
use greyscreen_engine::EngineCmd;

use super::Soak;
use super::clips::RECORDING_WAIT;
use super::profiles::WindowPlan;
use super::rng::Rng;
use super::window::SIZES;

const RESIZE_EVERY: Duration = Duration::from_millis(250);
const MINIMISED_FOR: Duration = Duration::from_millis(1500);
const CLOSE_GRACE: Duration = Duration::from_secs(15);

impl Soak<'_> {
    pub(super) fn drive_window(&mut self, cmd: &Sender<EngineCmd>, index: u32, rng: &mut Rng) {
        let Some(plan) = self.args.window_plan else {
            return;
        };
        if !self.wait_for(RECORDING_WAIT, |soak| soak.recording) {
            return;
        }
        match plan {
            WindowPlan::Closes { min_secs, max_secs } => {
                let wait = Duration::from_secs(u64::from(rng.between(min_secs, max_secs)));
                if !self.pump(Instant::now() + wait, true) {
                    self.close_window(cmd, index);
                }
            }
            WindowPlan::Resizes { secs } => self.resize_for(Duration::from_secs(u64::from(secs))),
            WindowPlan::Minimises { count } => self.minimise_and_restore(count),
        }
    }

    fn resize_for(&mut self, total: Duration) {
        let end = Instant::now() + total;
        let mut step = 0;
        while Instant::now() < end {
            let (width, height) = SIZES[step % SIZES.len()];
            if let Some(window) = &self.window {
                window.resize(width, height);
            }
            step += 1;
            if self.pump(Instant::now() + RESIZE_EVERY, true) {
                return;
            }
        }
    }

    fn minimise_and_restore(&mut self, count: u32) {
        for _ in 0..count {
            if let Some(window) = &self.window {
                window.minimise();
            }
            let saved = self.pump(Instant::now() + MINIMISED_FOR, true);
            if let Some(window) = &self.window {
                window.restore();
            }
            if saved || self.pump(Instant::now() + MINIMISED_FOR, true) {
                return;
            }
        }
    }

    fn close_window(&mut self, cmd: &Sender<EngineCmd>, index: u32) {
        let Some(window) = &self.window else {
            return;
        };
        if !window.close() {
            self.window_failures
                .push(format!("recording {index}: the test window did not close"));
        }
        if self.pump(Instant::now() + CLOSE_GRACE, true) || self.stopped {
            return;
        }
        self.window_failures.push(format!(
            "recording {index}: still recording {} s after its window closed",
            CLOSE_GRACE.as_secs()
        ));
        self.send(cmd, EngineCmd::StopRecording);
    }
}
