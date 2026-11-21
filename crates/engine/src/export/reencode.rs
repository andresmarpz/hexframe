use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use greyscreen_core::export::EncodePlan;

#[derive(Debug, Clone, Copy)]
#[cfg_attr(not(any(windows, test)), allow(dead_code))]
pub(crate) struct ReencodeRequest<'a> {
    pub(crate) src: &'a Path,
    pub(crate) dst: &'a Path,
    pub(crate) start_secs: f64,
    pub(crate) end_secs: f64,
    pub(crate) plan: EncodePlan,
}

#[derive(Debug)]
#[cfg_attr(not(any(windows, test)), allow(dead_code))]
pub(crate) enum ReencodeError {
    Cancelled,
    #[cfg_attr(windows, allow(dead_code))]
    Unsupported,
    Failed(String),
}

pub(crate) trait Reencode: Send + Sync {
    fn run(
        &self,
        request: &ReencodeRequest<'_>,
        progress: &mut dyn FnMut(f32),
        cancel: &AtomicBool,
    ) -> Result<(), ReencodeError>;
}

#[cfg(windows)]
pub(super) const AVAILABLE: bool = true;
#[cfg(not(windows))]
pub(super) const AVAILABLE: bool = false;

#[cfg(windows)]
struct MediaFoundation;

#[cfg(windows)]
impl Reencode for MediaFoundation {
    fn run(
        &self,
        request: &ReencodeRequest<'_>,
        progress: &mut dyn FnMut(f32),
        cancel: &AtomicBool,
    ) -> Result<(), ReencodeError> {
        use greyscreen_capture::transcode::{self, TranscodeError};
        transcode::transcode(
            &transcode::Request {
                src: request.src,
                dst: request.dst,
                start_secs: request.start_secs,
                end_secs: request.end_secs,
                plan: request.plan,
            },
            progress,
            cancel,
        )
        .map_err(|err| match err {
            TranscodeError::Cancelled => ReencodeError::Cancelled,
            TranscodeError::Failed(why) => ReencodeError::Failed(why),
        })
    }
}

#[cfg(not(windows))]
struct Unsupported;

#[cfg(not(windows))]
impl Reencode for Unsupported {
    fn run(
        &self,
        _request: &ReencodeRequest<'_>,
        _progress: &mut dyn FnMut(f32),
        _cancel: &AtomicBool,
    ) -> Result<(), ReencodeError> {
        Err(ReencodeError::Unsupported)
    }
}

pub(crate) fn default_reencoder() -> Arc<dyn Reencode> {
    #[cfg(windows)]
    {
        Arc::new(MediaFoundation)
    }
    #[cfg(not(windows))]
    {
        Arc::new(Unsupported)
    }
}
