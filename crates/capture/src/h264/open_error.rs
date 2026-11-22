use greyscreen_core::encoder::{OpenFailure, OpenStep};
use windows::core::Error;

use super::H264Error;

fn name(step: OpenStep) -> &'static str {
    match step {
        OpenStep::Setup => "configuring the encoder",
        OpenStep::Activate => "activating the encoder",
        OpenStep::D3dManager => "SET_D3D_MANAGER",
        OpenStep::OutputType => "SetOutputType",
        OpenStep::InputType => "SetInputType(NV12)",
        OpenStep::Start => "starting the stream",
    }
}

pub(super) fn classify(step: OpenStep, vendor_id: u32, code: i32) -> OpenFailure {
    OpenFailure::at_step(step, vendor_id, code as u32)
}

pub(super) fn at(step: OpenStep, vendor_id: u32, error: &Error) -> H264Error {
    let code = error.code().0;
    let failure = classify(step, vendor_id, code);
    H264Error::NoHardwareEncoder(failure.tag(&format!(
        "{} failed: {error} (code {:#010x})",
        name(step),
        code as u32
    )))
}

pub(super) fn tag_untagged(error: H264Error, vendor_id: u32) -> H264Error {
    match error {
        H264Error::Windows(e) | H264Error::Property { source: e, .. } | H264Error::Stopped(e) => {
            at(OpenStep::Setup, vendor_id, &e)
        }
        other => other,
    }
}

pub(super) fn unsupported(reason: &str) -> H264Error {
    H264Error::NoHardwareEncoder(OpenFailure::Unsupported.tag(reason))
}
