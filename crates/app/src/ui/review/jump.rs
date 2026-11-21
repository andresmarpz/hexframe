use greyscreen_core::library::EventEntry;

pub(crate) const SEEK_LEAD_SECS: f64 = 5.0;
const NEXT_MARGIN_SECS: f64 = 0.5;
const PREVIOUS_MARGIN_SECS: f64 = 1.0;

pub(crate) fn seek_target(rec_secs: f64) -> f64 {
    (rec_secs - SEEK_LEAD_SECS).max(0.0)
}

fn targets(events: &[EventEntry]) -> Vec<f64> {
    let mut targets: Vec<f64> = events.iter().map(|e| seek_target(e.rec_secs)).collect();
    targets.sort_by(f64::total_cmp);
    targets
}

pub(crate) fn next_target(events: &[EventEntry], now: f64) -> Option<f64> {
    targets(events)
        .into_iter()
        .find(|t| *t > now + NEXT_MARGIN_SECS)
}

pub(crate) fn previous_target(events: &[EventEntry], now: f64) -> Option<f64> {
    targets(events)
        .into_iter()
        .rev()
        .find(|t| *t < now - PREVIOUS_MARGIN_SECS)
}
