use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use anyhow::Result;
use crossbeam_channel::Sender;
use greyscreen_core::encoder::{Attempt, Probe};
use greyscreen_core::media::{CaptureConfig, CaptureTarget, MediaEvent, MediaSession, MediaSource};

#[cfg(all(windows, feature = "cloaked-window"))]
const WIDTH: i32 = 960;
#[cfg(all(windows, feature = "cloaked-window"))]
const HEIGHT: i32 = 540;
pub(super) const SIZES: [(i32, i32); 2] = [(960, 540), (1280, 720)];

pub(super) trait TestWindow: Send {
    fn target(&self) -> CaptureTarget;
    fn resize(&self, width: i32, height: i32);
    fn minimise(&self);
    fn restore(&self);
    fn close(self: Box<Self>) -> bool;
}

pub(super) type WindowFactory = Box<dyn Fn() -> Result<Box<dyn TestWindow>, String> + Send + Sync>;

type Slot = Arc<Mutex<Option<Box<dyn TestWindow>>>>;

#[derive(Clone)]
pub(super) struct WindowHandle(Slot);

impl WindowHandle {
    fn slot(&self) -> MutexGuard<'_, Option<Box<dyn TestWindow>>> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub(super) fn resize(&self, width: i32, height: i32) {
        if let Some(window) = self.slot().as_ref() {
            window.resize(width, height);
        }
    }

    pub(super) fn minimise(&self) {
        if let Some(window) = self.slot().as_ref() {
            window.minimise();
        }
    }

    pub(super) fn restore(&self) {
        if let Some(window) = self.slot().as_ref() {
            window.restore();
        }
    }

    pub(super) fn close(&self) -> bool {
        let window = self.slot().take();
        window.is_none_or(|window| window.close())
    }
}

pub(super) struct WindowSource {
    inner: Arc<dyn MediaSource>,
    factory: WindowFactory,
    slot: Slot,
}

impl WindowSource {
    pub(super) fn wrap(
        inner: Arc<dyn MediaSource>,
        factory: WindowFactory,
    ) -> (Arc<dyn MediaSource>, WindowHandle) {
        let slot = Slot::default();
        let source = Self {
            inner,
            factory,
            slot: Arc::clone(&slot),
        };
        (Arc::new(source), WindowHandle(slot))
    }

    fn window_target(&self) -> Result<CaptureTarget, String> {
        let mut slot = self.slot.lock().unwrap_or_else(PoisonError::into_inner);
        if slot.is_none() {
            *slot = Some((self.factory)()?);
        }
        slot.as_ref()
            .map(|window| window.target())
            .ok_or_else(|| "the test window is gone".to_owned())
    }
}

impl MediaSource for WindowSource {
    fn start(
        &self,
        target: CaptureTarget,
        cfg: CaptureConfig,
        out: Sender<MediaEvent>,
    ) -> Result<Box<dyn MediaSession>, String> {
        let target = match target {
            CaptureTarget::PrimaryMonitor => self.window_target()?,
            other => other,
        };
        self.inner.start(target, cfg, out)
    }

    fn encoder_fingerprint(&self) -> Option<String> {
        self.inner.encoder_fingerprint()
    }

    fn check_encoders(&self, cfg: &CaptureConfig, every: bool) -> Vec<Attempt> {
        self.inner.check_encoders(cfg, every)
    }

    fn encoder_probes(&self) -> Vec<Probe> {
        self.inner.encoder_probes()
    }

    fn probe_encoder(&self, cfg: &CaptureConfig, probe: &Probe) -> Result<String, String> {
        self.inner.probe_encoder(cfg, probe)
    }
}

#[cfg(all(windows, feature = "cloaked-window"))]
mod cloaked {
    use greyscreen_capture::testing::cloaked_window::{CloakedWindow, Reveal};

    use super::{CaptureTarget, HEIGHT, TestWindow, WIDTH, WindowFactory};

    struct Cloaked(CloakedWindow);

    impl TestWindow for Cloaked {
        fn target(&self) -> CaptureTarget {
            self.0.target()
        }

        fn resize(&self, width: i32, height: i32) {
            self.0.resize(width, height).ok();
        }

        fn minimise(&self) {
            self.0.minimise();
        }

        fn restore(&self) {
            self.0.restore();
        }

        fn close(self: Box<Self>) -> bool {
            self.0.close()
        }
    }

    pub(super) fn factory() -> WindowFactory {
        Box::new(|| {
            CloakedWindow::open(WIDTH, HEIGHT, Reveal::ShownCloaked)
                .map(|window| Box::new(Cloaked(window)) as Box<dyn TestWindow>)
                .map_err(|error| format!("cannot open the cloaked test window: {error}"))
        })
    }
}

#[cfg(all(windows, feature = "cloaked-window"))]
pub(super) fn cloaked_factory() -> Result<WindowFactory> {
    Ok(cloaked::factory())
}

#[cfg(not(all(windows, feature = "cloaked-window")))]
pub(super) fn cloaked_factory() -> Result<WindowFactory> {
    anyhow::bail!(
        "the cloaked test window needs Windows and a build with --features cloaked-window"
    )
}
