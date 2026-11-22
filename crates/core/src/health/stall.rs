use crate::time::{HNS_PER_SEC, Hns};

pub const STALL_MIN_SECS: i64 = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Mark {
    at: Hns,
    captured: u64,
    audio: u64,
}

#[derive(Debug, Clone, Default)]
pub struct StallWatch {
    since: Option<Mark>,
    latest: Option<Mark>,
    count: u32,
    longest: Hns,
}

impl StallWatch {
    pub fn observe(&mut self, at: Hns, captured: u64, audio: u64) {
        let now = Mark {
            at,
            captured,
            audio,
        };
        self.latest = Some(now);
        match self.since {
            None => self.since = Some(now),
            Some(since) if captured > since.captured => {
                if let Some(length) = stall_length(since, now) {
                    self.count += 1;
                    self.longest = self.longest.max(length);
                }
                self.since = Some(now);
            }
            Some(_) => {}
        }
    }

    pub fn summary(&self) -> (u32, f64) {
        let open = match (self.since, self.latest) {
            (Some(since), Some(latest)) => stall_length(since, latest),
            _ => None,
        };
        let count = self.count + u32::from(open.is_some());
        let longest = self.longest.max(open.unwrap_or(0));
        (count, longest as f64 / HNS_PER_SEC as f64)
    }
}

fn stall_length(since: Mark, now: Mark) -> Option<Hns> {
    let length = now.at - since.at;
    (length >= STALL_MIN_SECS * HNS_PER_SEC && now.audio > since.audio).then_some(length)
}
