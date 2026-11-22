use crate::time::{HNS_PER_SEC, Hns};

const START_TIMEOUT: Hns = 15 * HNS_PER_SEC;
const START_RETRY_DELAY: Hns = 5 * HNS_PER_SEC;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionTiming {
    pub start_timeout: Hns,
    pub retry_delay: Hns,
}

impl Default for SessionTiming {
    fn default() -> Self {
        Self {
            start_timeout: START_TIMEOUT,
            retry_delay: START_RETRY_DELAY,
        }
    }
}
