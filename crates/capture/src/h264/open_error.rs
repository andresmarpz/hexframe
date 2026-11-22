use greyscreen_core::encoder::OpenFailure;
use windows::core::Error;

use super::H264Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Step {
    Activate,
    OutputType,
    InputType,
    Start,
}

impl Step {
    fn name(self) -> &'static str {
        match self {
            Step::Activate => "activating the encoder",
            Step::OutputType => "SetOutputType",
            Step::InputType => "SetInputType(NV12)",
            Step::Start => "starting the stream",
        }
    }

    fn fallback(self) -> OpenFailure {
        match self {
            Step::Activate | Step::OutputType => OpenFailure::Other,
            Step::InputType => OpenFailure::InputRefused,
            Step::Start => OpenFailure::StartFailed,
        }
    }
}

pub(super) fn classify(step: Step, code: i32) -> OpenFailure {
    OpenFailure::from_code(code as u32, step.fallback())
}

pub(super) fn at(step: Step, error: &Error) -> H264Error {
    let code = error.code().0;
    let failure = classify(step, code);
    H264Error::NoHardwareEncoder(failure.tag(&format!(
        "{} failed: {error} (code {:#010x})",
        step.name(),
        code as u32
    )))
}

pub(super) fn unsupported(reason: &str) -> H264Error {
    H264Error::NoHardwareEncoder(OpenFailure::Unsupported.tag(reason))
}
