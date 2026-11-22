use std::time::{SystemTime, UNIX_EPOCH};

pub(super) struct Rng {
    pub(super) seed: u64,
    state: u64,
}

impl Rng {
    pub(super) fn new(seed: u64) -> Self {
        Self {
            seed,
            state: seed | 1,
        }
    }

    pub(super) fn from_clock() -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(1, |d| d.as_nanos() as u64);
        Self::new(nanos)
    }

    fn next(&mut self) -> u64 {
        self.state ^= self.state << 13;
        self.state ^= self.state >> 7;
        self.state ^= self.state << 17;
        self.state
    }

    pub(super) fn between(&mut self, min: u32, max: u32) -> u32 {
        let span = u64::from(max.saturating_sub(min)) + 1;
        min + (self.next() % span) as u32
    }
}
