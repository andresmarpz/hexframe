use crate::time::{HNS_PER_SEC, Hns};

const MAX_PLAN: Hns = 90 * HNS_PER_SEC;
const DUE_DELAY: Hns = HNS_PER_SEC;
const MAX_LABELS: usize = 3;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClipPlan {
    pub start: Hns,
    pub end: Hns,
    pub label: String,
    pub events: Vec<u32>,
}

#[derive(Debug)]
struct Open {
    start: Hns,
    end: Hns,
    labels: Vec<String>,
    events: Vec<u32>,
}

impl Open {
    fn into_plan(self) -> ClipPlan {
        let mut label = self
            .labels
            .iter()
            .take(MAX_LABELS)
            .map(String::as_str)
            .collect::<Vec<_>>()
            .join("+");
        if self.labels.len() > MAX_LABELS {
            label.push_str("+more");
        }
        ClipPlan {
            start: self.start,
            end: self.end,
            label,
            events: self.events,
        }
    }
}

#[derive(Debug)]
pub struct ClipPlanner {
    pre: Hns,
    post: Hns,
    open: Option<Open>,
    closed: Vec<ClipPlan>,
}

impl ClipPlanner {
    pub fn new(pre: Hns, post: Hns) -> Self {
        Self {
            pre,
            post,
            open: None,
            closed: Vec::new(),
        }
    }

    pub fn on_highlight(&mut self, at: Hns, label: &str, event_id: u32) {
        let start = at - self.pre;
        let end = at + self.post;
        if let Some(open) = self.open.as_mut()
            && start < open.end
            && end - open.start <= MAX_PLAN
        {
            open.end = open.end.max(end);
            open.labels.push(label.to_owned());
            open.events.push(event_id);
            return;
        }
        if let Some(open) = self.open.take() {
            self.closed.push(open.into_plan());
        }
        self.open = Some(Open {
            start,
            end: end.min(start + MAX_PLAN),
            labels: vec![label.to_owned()],
            events: vec![event_id],
        });
    }

    pub fn due(&mut self, now: Hns) -> Vec<ClipPlan> {
        let ready = self
            .closed
            .iter()
            .take_while(|plan| now >= plan.end + DUE_DELAY)
            .count();
        let mut out: Vec<ClipPlan> = self.closed.drain(..ready).collect();
        if self.closed.is_empty()
            && let Some(open) = self.open.take_if(|open| now >= open.end + DUE_DELAY)
        {
            out.push(open.into_plan());
        }
        out
    }

    pub fn flush(&mut self) -> Vec<ClipPlan> {
        let mut out = std::mem::take(&mut self.closed);
        out.extend(self.open.take().map(Open::into_plan));
        out
    }
}

#[cfg(test)]
mod tests;
