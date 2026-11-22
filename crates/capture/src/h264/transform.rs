use windows::Win32::Foundation::E_NOTIMPL;
use windows::Win32::Media::MediaFoundation::{
    ICodecAPI, IMFActivate, IMFAttributes, IMFDXGIDeviceManager, IMFMediaEvent,
    IMFMediaEventGenerator, IMFMediaType, IMFSample, IMFShutdown, IMFTransform,
    MF_EVENT_FLAG_NO_WAIT, MFT_ENUM_HARDWARE_VENDOR_ID_Attribute, MFT_FRIENDLY_NAME_Attribute,
    MFT_MESSAGE_SET_D3D_MANAGER, MFT_MESSAGE_TYPE, MFT_OUTPUT_DATA_BUFFER, MFT_OUTPUT_STREAM_INFO,
};
use windows::Win32::System::Variant::VARIANT;
use windows::core::{GUID, HRESULT, Interface};

use crate::probe::string_attribute;

pub(crate) type MfResult<T> = windows::core::Result<T>;

pub(crate) trait Candidate {
    fn activate(&self) -> MfResult<Box<dyn Mft>>;
    fn shutdown_object(&self);
    fn describe(&self) -> String;
}

pub(super) struct RealCandidate(pub(super) IMFActivate);

impl Candidate for RealCandidate {
    fn activate(&self) -> MfResult<Box<dyn Mft>> {
        // SAFETY: the activate object is live.
        let transform: IMFTransform = unsafe { self.0.ActivateObject() }?;
        Ok(Box::new(RealMft::new(transform, Some(self.0.clone()))?))
    }

    fn shutdown_object(&self) {
        // SAFETY: the activate object is live; shutting it down releases the failed transform.
        if let Err(error) = unsafe { self.0.ShutdownObject() } {
            tracing::debug!(%error, "candidate shutdown failed");
        }
    }

    fn describe(&self) -> String {
        format!(
            "{} (vendor {})",
            string_attribute(&self.0, &MFT_FRIENDLY_NAME_Attribute),
            string_attribute(&self.0, &MFT_ENUM_HARDWARE_VENDOR_ID_Attribute)
        )
    }
}

pub(crate) trait Mft: std::fmt::Debug {
    fn attributes(&self) -> MfResult<IMFAttributes>;
    fn stream_ids(&self, inputs: &mut [u32], outputs: &mut [u32]) -> MfResult<()>;
    fn process_message(&self, message: MFT_MESSAGE_TYPE) -> MfResult<()>;
    fn set_d3d_manager(&self, manager: &IMFDXGIDeviceManager) -> MfResult<()>;
    fn process_input(&self, stream: u32, sample: &IMFSample) -> MfResult<()>;
    fn process_output(
        &self,
        buffers: &mut [MFT_OUTPUT_DATA_BUFFER],
        status: &mut u32,
    ) -> MfResult<()>;
    fn output_stream_info(&self, stream: u32) -> MfResult<MFT_OUTPUT_STREAM_INFO>;
    fn input_stream_attributes(&self, stream: u32) -> MfResult<IMFAttributes>;
    fn input_available_type(&self, stream: u32, index: u32) -> MfResult<IMFMediaType>;
    fn output_available_type(&self, stream: u32, index: u32) -> MfResult<IMFMediaType>;
    fn output_current_type(&self, stream: u32) -> MfResult<IMFMediaType>;
    fn set_input_type(&self, stream: u32, media_type: &IMFMediaType) -> MfResult<()>;
    fn set_output_type(&self, stream: u32, media_type: &IMFMediaType) -> MfResult<()>;
    fn next_event(&self) -> MfResult<IMFMediaEvent>;
    fn is_supported(&self, property: &GUID) -> HRESULT;
    fn set_value(&self, property: &GUID, value: &VARIANT) -> MfResult<()>;
    fn get_value(&self, property: &GUID) -> MfResult<VARIANT>;
    fn shutdown(&self);
}

#[derive(Debug)]
pub(super) struct RealMft {
    transform: IMFTransform,
    codec: ICodecAPI,
    events: Option<IMFMediaEventGenerator>,
    activate: Option<IMFActivate>,
}

impl RealMft {
    pub(super) fn new(transform: IMFTransform, activate: Option<IMFActivate>) -> MfResult<Self> {
        let codec = transform.cast()?;
        let events = match &activate {
            Some(_) => Some(transform.cast()?),
            None => None,
        };
        Ok(Self {
            transform,
            codec,
            events,
            activate,
        })
    }
}

impl Mft for RealMft {
    fn attributes(&self) -> MfResult<IMFAttributes> {
        // SAFETY: the transform is live.
        unsafe { self.transform.GetAttributes() }
    }

    fn stream_ids(&self, inputs: &mut [u32], outputs: &mut [u32]) -> MfResult<()> {
        // SAFETY: both slices are live.
        unsafe { self.transform.GetStreamIDs(inputs, outputs) }
    }

    fn process_message(&self, message: MFT_MESSAGE_TYPE) -> MfResult<()> {
        // SAFETY: the transform is live and the message takes no parameter.
        unsafe { self.transform.ProcessMessage(message, 0) }
    }

    fn set_d3d_manager(&self, manager: &IMFDXGIDeviceManager) -> MfResult<()> {
        // SAFETY: the transform is live and the borrowed manager outlives the call.
        unsafe {
            self.transform
                .ProcessMessage(MFT_MESSAGE_SET_D3D_MANAGER, manager.as_raw() as usize)
        }
    }

    fn process_input(&self, stream: u32, sample: &IMFSample) -> MfResult<()> {
        // SAFETY: the transform is live and the sample outlives the call.
        unsafe { self.transform.ProcessInput(stream, sample, 0) }
    }

    fn process_output(
        &self,
        buffers: &mut [MFT_OUTPUT_DATA_BUFFER],
        status: &mut u32,
    ) -> MfResult<()> {
        // SAFETY: the transform is live; the buffers and status outlive the call.
        unsafe { self.transform.ProcessOutput(0, buffers, status) }
    }

    fn output_stream_info(&self, stream: u32) -> MfResult<MFT_OUTPUT_STREAM_INFO> {
        // SAFETY: the transform is live.
        unsafe { self.transform.GetOutputStreamInfo(stream) }
    }

    fn input_stream_attributes(&self, stream: u32) -> MfResult<IMFAttributes> {
        // SAFETY: the transform is live.
        unsafe { self.transform.GetInputStreamAttributes(stream) }
    }

    fn input_available_type(&self, stream: u32, index: u32) -> MfResult<IMFMediaType> {
        // SAFETY: the transform is live.
        unsafe { self.transform.GetInputAvailableType(stream, index) }
    }

    fn output_available_type(&self, stream: u32, index: u32) -> MfResult<IMFMediaType> {
        // SAFETY: the transform is live.
        unsafe { self.transform.GetOutputAvailableType(stream, index) }
    }

    fn output_current_type(&self, stream: u32) -> MfResult<IMFMediaType> {
        // SAFETY: the transform is live.
        unsafe { self.transform.GetOutputCurrentType(stream) }
    }

    fn set_input_type(&self, stream: u32, media_type: &IMFMediaType) -> MfResult<()> {
        // SAFETY: the transform and type are live.
        unsafe { self.transform.SetInputType(stream, media_type, 0) }
    }

    fn set_output_type(&self, stream: u32, media_type: &IMFMediaType) -> MfResult<()> {
        // SAFETY: the transform and type are live.
        unsafe { self.transform.SetOutputType(stream, media_type, 0) }
    }

    fn next_event(&self) -> MfResult<IMFMediaEvent> {
        let Some(events) = &self.events else {
            return Err(E_NOTIMPL.into());
        };
        // SAFETY: the event generator is live; NO_WAIT makes the call non-blocking.
        unsafe { events.GetEvent(MF_EVENT_FLAG_NO_WAIT) }
    }

    fn is_supported(&self, property: &GUID) -> HRESULT {
        // SAFETY: the vtable call receives the live interface pointer and a GUID that outlives it.
        unsafe {
            (Interface::vtable(&self.codec).IsSupported)(Interface::as_raw(&self.codec), property)
        }
    }

    fn set_value(&self, property: &GUID, value: &VARIANT) -> MfResult<()> {
        // SAFETY: the codec interface is live and the value outlives the call.
        unsafe { self.codec.SetValue(property, value) }
    }

    fn get_value(&self, property: &GUID) -> MfResult<VARIANT> {
        // SAFETY: the codec interface is live.
        unsafe { self.codec.GetValue(property) }
    }

    fn shutdown(&self) {
        let Some(activate) = &self.activate else {
            return;
        };
        if let Ok(shutdown) = self.transform.cast::<IMFShutdown>() {
            // SAFETY: the interface is live; Shutdown ends the event queue.
            if let Err(error) = unsafe { shutdown.Shutdown() } {
                tracing::debug!(%error, "encoder IMFShutdown failed");
            }
        }
        // SAFETY: the activate object created this transform and is still live.
        if let Err(error) = unsafe { activate.ShutdownObject() } {
            tracing::debug!(%error, "encoder activate shutdown failed");
        }
    }
}
