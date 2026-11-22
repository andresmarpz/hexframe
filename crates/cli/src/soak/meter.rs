use super::check::Sample;

#[cfg(windows)]
pub(super) struct Meter(greyscreen_capture::procstats::UsageMeter);

#[cfg(windows)]
impl Meter {
    pub(super) fn new() -> Self {
        Self(greyscreen_capture::procstats::UsageMeter::for_primary_adapter())
    }

    pub(super) fn sample(&self, t_secs: f64) -> Sample {
        let usage = self.0.sample();
        Sample {
            t_secs,
            private_bytes: usage.map(|u| u.private_bytes),
            handles: usage.map(|u| u.handles),
            threads: usage.map(|u| u.threads),
            gpu_bytes: usage.and_then(|u| u.gpu_dedicated_bytes),
            ..Sample::default()
        }
    }
}

#[cfg(not(windows))]
pub(super) struct Meter;

#[cfg(not(windows))]
impl Meter {
    pub(super) fn new() -> Self {
        Self
    }

    pub(super) fn sample(&self, t_secs: f64) -> Sample {
        Sample {
            t_secs,
            ..Sample::default()
        }
    }
}
