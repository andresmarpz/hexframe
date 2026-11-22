use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use windows::core::{Error, HRESULT};

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Point {
    AdapterOpen,
    CaptureStart,
    EncoderCandidate,
    FirstFrameWait,
    GpuCopyWait,
    DeviceRemovedCheck,
    FrameCopyTexture,
    ConverterSetup,
    ConversionSupport,
    ConverterConvert,
    EncoderSubmit,
    MftActivation,
    CaptureBorder,
    CaptureMinUpdateInterval,
    Mmcss,
    GpuPriority,
    StagingMap,
    SingleInstanceEvent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Fail(HRESULT),
    FailAfter(HRESULT, usize),
    Stall(Duration),
}

static ARMED: Mutex<Vec<(Point, Action)>> = Mutex::new(Vec::new());
static SERIAL: Mutex<()> = Mutex::new(());

fn armed() -> MutexGuard<'static, Vec<(Point, Action)>> {
    ARMED.lock().unwrap_or_else(PoisonError::into_inner)
}

pub fn hit(point: Point) -> windows::core::Result<()> {
    let action = armed()
        .iter_mut()
        .find(|(armed, _)| *armed == point)
        .map(|(_, action)| match action {
            Action::FailAfter(code, 0) => Action::Fail(*code),
            Action::FailAfter(_, skips) => {
                *skips -= 1;
                Action::Stall(Duration::ZERO)
            }
            other => *other,
        });
    match action {
        None => Ok(()),
        Some(Action::Fail(code) | Action::FailAfter(code, _)) => Err(Error::from(code)),
        Some(Action::Stall(duration)) => {
            std::thread::sleep(duration);
            Ok(())
        }
    }
}

pub fn pending(point: Point) -> bool {
    armed()
        .iter()
        .any(|(armed, action)| *armed == point && matches!(action, Action::Stall(_)))
}

#[derive(Debug)]
pub struct Armed {
    _serial: MutexGuard<'static, ()>,
}

impl Armed {
    pub fn new() -> Self {
        Self {
            _serial: SERIAL.lock().unwrap_or_else(PoisonError::into_inner),
        }
    }

    pub fn fail(self, point: Point, code: HRESULT) -> Self {
        armed().push((point, Action::Fail(code)));
        self
    }

    pub fn fail_after(self, point: Point, skips: usize, code: HRESULT) -> Self {
        armed().push((point, Action::FailAfter(code, skips)));
        self
    }

    pub fn stall(self, point: Point, duration: Duration) -> Self {
        armed().push((point, Action::Stall(duration)));
        self
    }
}

impl Default for Armed {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for Armed {
    fn drop(&mut self) {
        armed().clear();
    }
}
