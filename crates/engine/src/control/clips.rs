use greyscreen_core::clips::ClipPlan;
use greyscreen_core::highlight::Highlight;
use greyscreen_core::library::{self, ClipEntry};
use greyscreen_core::time::{Hns, hns_to_secs, mono_now, secs_to_hns};

use super::Control;
use crate::api::{self, NoticeLevel, UiMsg};
use crate::clipjob::{self, ClipJob, ClipResult};
use crate::library::CLIPS_DIR;

pub(super) const MANUAL_CLIP_SECS: u32 = 30;
const MANUAL_LABEL: &str = "Manual";

impl Control {
    pub(super) fn plan_clip(&mut self, at: Hns, event_id: u32, highlights: &[Highlight]) {
        let Some(recording) = self.recording.as_mut() else {
            return;
        };
        let Some(planner) = recording.planner.as_mut() else {
            return;
        };
        if at < recording.t0 {
            return;
        }
        if let Some(worthy) = highlights
            .iter()
            .find(|h| h.is_clip_worthy(&self.settings.clips))
        {
            planner.on_highlight(at, worthy.label(), event_id);
        }
    }

    pub(super) fn save_clip_now(&mut self) {
        let now = mono_now();
        let Some(recording) = self.recording.as_mut() else {
            self.notice(NoticeLevel::Info, "Nothing is recording, no clip saved.");
            return;
        };
        recording.queued.push(ClipPlan {
            start: now - secs_to_hns(f64::from(MANUAL_CLIP_SECS)),
            end: now,
            label: MANUAL_LABEL.to_owned(),
            events: Vec::new(),
        });
    }

    pub(super) fn run_due_clips(&mut self, now: Hns) {
        let Some(recording) = self.recording.as_mut() else {
            return;
        };
        let mut plans = std::mem::take(&mut recording.queued);
        if let Some(planner) = recording.planner.as_mut() {
            plans.extend(planner.due(now));
        }
        self.run_plans(plans);
    }

    pub(super) fn flush_clips(&mut self) {
        let Some(recording) = self.recording.as_mut() else {
            return;
        };
        if recording.clips_flushed || recording.result.is_none() || !recording.media_stopped {
            return;
        }
        recording.clips_flushed = true;
        let mut plans = std::mem::take(&mut recording.queued);
        if let Some(planner) = recording.planner.as_mut() {
            plans.extend(planner.flush());
        }
        self.run_plans(plans);
    }

    pub(super) fn on_clip_done(&mut self, result: ClipResult) {
        let Some(recording) = self.recording.as_mut() else {
            return;
        };
        recording.pending_clips = recording.pending_clips.saturating_sub(1);
        match result {
            Ok(clip) => {
                let at = recording
                    .meta
                    .clips
                    .partition_point(|c| c.start_secs <= clip.start_secs);
                recording.meta.clips.insert(at, clip.clone());
                recording.dirty = true;
                let folder = recording.folder.clone();
                self.send(UiMsg::ClipSaved(api::ClipEntry { clip, folder }));
            }
            Err(err) => {
                tracing::warn!(%err, "clip not saved");
                self.notice(
                    NoticeLevel::Warn,
                    format!("A clip could not be saved: {err}"),
                );
            }
        }
        self.try_complete();
    }

    fn run_plans(&mut self, plans: Vec<ClipPlan>) {
        for plan in plans {
            self.run_plan(plan);
        }
    }

    fn run_plan(&mut self, mut plan: ClipPlan) {
        let Some(recording) = self.recording.as_mut() else {
            return;
        };
        plan.start = plan.start.max(recording.t0);
        let packets = recording.ring.range(plan.start, plan.end);
        if plan.end <= plan.start || packets.is_empty() {
            tracing::warn!(label = %plan.label, "no recorded video for clip");
            self.notice(
                NoticeLevel::Warn,
                "A clip was skipped: no video was left for it.",
            );
            return;
        }
        let start_secs = hns_to_secs(plan.start - recording.t0);
        let base = library::clip_file_name(start_secs, &plan.label);
        let name = unique_name(&base, |candidate| {
            recording.clip_names.contains(candidate)
                || recording.folder.join(CLIPS_DIR).join(candidate).exists()
        });
        recording.clip_names.insert(name.clone());
        let job = ClipJob {
            path: recording.folder.join(CLIPS_DIR).join(&name),
            entry: ClipEntry {
                file: format!("{CLIPS_DIR}/{name}"),
                start_secs,
                end_secs: hns_to_secs(plan.end - recording.t0),
                label: plan.label,
            },
            video: recording.video.clone(),
            audio: recording.audio.clone(),
            packets,
        };
        let tx = self.internal_tx.clone();
        let spawned = clipjob::spawn(job, move |result| {
            if tx.send(super::Internal::ClipDone(result)).is_err() {
                tracing::debug!("engine stopped before a clip finished");
            }
        });
        if spawned {
            recording.pending_clips += 1;
        } else {
            self.notice(NoticeLevel::Warn, "A clip could not be started.");
        }
    }
}

fn unique_name(base: &str, taken: impl Fn(&str) -> bool) -> String {
    if !taken(base) {
        return base.to_owned();
    }
    let stem = base.strip_suffix(".mp4").unwrap_or(base);
    (2u32..)
        .map(|n| format!("{stem}_{n}.mp4"))
        .find(|candidate| !taken(candidate))
        .unwrap_or_else(|| base.to_owned())
}
