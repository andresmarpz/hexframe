use greyscreen_engine::RecordingEntry;

pub(crate) const RECENT_LIMIT: usize = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct LibraryTotals {
    pub bytes: u64,
}

impl LibraryTotals {
    pub(crate) fn of(recordings: &[RecordingEntry]) -> LibraryTotals {
        LibraryTotals {
            bytes: recordings.iter().map(|rec| rec.size_bytes).sum(),
        }
    }

    pub(crate) fn used_fraction(&self, limit_bytes: Option<u64>) -> Option<f32> {
        let limit = limit_bytes.filter(|limit| *limit > 0)?;
        Some((self.bytes as f64 / limit as f64).clamp(0.0, 1.0) as f32)
    }
}

#[cfg(test)]
mod tests;
