use std::cell::Cell;
use std::collections::{HashMap, VecDeque};
use std::mem::ManuallyDrop;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread::ThreadId;
use std::time::Duration;

use windows::Win32::Foundation::{E_NOTIMPL, E_POINTER, S_OK};
use windows::Win32::Media::MediaFoundation::{
    IMFAttributes, IMFDXGIDeviceManager, IMFMediaEvent, IMFMediaType, IMFSample, MEError,
    METransformDrainComplete, METransformHaveOutput, METransformNeedInput,
    MF_E_NO_EVENTS_AVAILABLE, MF_E_NOTACCEPTING, MF_E_TRANSFORM_NEED_MORE_INPUT, MF_SA_D3D11_AWARE,
    MF_TRANSFORM_ASYNC, MFCreateAttributes, MFCreateDXGIDeviceManager, MFCreateMediaEvent,
    MFCreateMemoryBuffer, MFCreateSample, MFT_MESSAGE_COMMAND_DRAIN, MFT_MESSAGE_COMMAND_FLUSH,
    MFT_MESSAGE_TYPE, MFT_OUTPUT_DATA_BUFFER, MFT_OUTPUT_STREAM_INFO,
    MFT_OUTPUT_STREAM_PROVIDES_SAMPLES,
};
use windows::Win32::System::Variant::VARIANT;
use windows::core::{GUID, HRESULT};

use crate::h264::DrainClock;
use crate::h264::transform::{Candidate, MfResult, Mft};

const IDR_PACKET: [u8; 7] = [0, 0, 0, 1, 0x65, 0x88, 0x84];
const SLICE_PACKET: [u8; 7] = [0, 0, 0, 1, 0x41, 0x9a, 0x01];
const INITIAL_NEED_INPUT: u32 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Step {
    Attributes,
    SetD3dManager,
    SetOutputType,
    SetInputType,
    ProcessMessage,
    OutputStreamInfo,
    ProcessInput,
    ProcessOutput,
    NextEvent,
    SetValue,
}

const STEP_COUNT: usize = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CallKind {
    Attributes,
    StreamIds,
    SetD3dManager,
    ProcessInput,
    ProcessOutput,
    Message(i32),
    OutputStreamInfo,
    InputStreamAttributes,
    InputAvailableType,
    OutputAvailableType,
    OutputCurrentType,
    SetInputType,
    SetOutputType,
    NextEvent,
    IsSupported,
    SetValue,
    GetValue,
    Shutdown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Call {
    pub(crate) kind: CallKind,
    pub(crate) thread: ThreadId,
    pub(crate) property: Option<GUID>,
    pub(crate) value: Option<u32>,
}

pub(crate) type Calls = Arc<Mutex<Vec<Call>>>;

#[derive(Debug, Default)]
struct Script {
    fail: Option<(Step, usize, HRESULT)>,
    fail_from: Option<(Step, usize, HRESULT)>,
    never_need_input: bool,
    never_drain_complete: bool,
    never_output: bool,
    flush_block: Duration,
    not_accepting: bool,
    refused: Vec<(GUID, Option<u32>, HRESULT)>,
    echo_values: bool,
    read_back: Vec<(GUID, u32)>,
}

#[derive(Debug, Default)]
struct State {
    values: HashMap<GUID, u32>,
    events: VecDeque<i32>,
    pending_pts: VecDeque<i64>,
    counts: [usize; STEP_COUNT],
    outputs: usize,
}

#[derive(Debug)]
pub(crate) struct FakeTransform {
    script: Script,
    state: Mutex<State>,
    calls: Calls,
}

impl FakeTransform {
    pub(crate) fn new() -> Self {
        let mut state = State::default();
        state
            .events
            .extend((0..INITIAL_NEED_INPUT).map(|_| METransformNeedInput.0));
        Self {
            script: Script::default(),
            state: Mutex::new(state),
            calls: Calls::default(),
        }
    }

    pub(crate) fn failing_at(mut self, step: Step, nth: usize, code: HRESULT) -> Self {
        self.script.fail = Some((step, nth, code));
        self
    }

    pub(crate) fn failing_from(mut self, step: Step, nth: usize, code: HRESULT) -> Self {
        self.script.fail_from = Some((step, nth, code));
        self
    }

    pub(crate) fn never_needing_input(mut self) -> Self {
        self.script.never_need_input = true;
        self.state().events.clear();
        self
    }

    pub(crate) fn never_draining(mut self) -> Self {
        self.script.never_drain_complete = true;
        self
    }

    pub(crate) fn never_outputting(mut self) -> Self {
        self.script.never_output = true;
        self
    }

    pub(crate) fn blocking_flush(mut self, duration: Duration) -> Self {
        self.script.flush_block = duration;
        self
    }

    pub(crate) fn not_accepting(mut self) -> Self {
        self.script.not_accepting = true;
        self
    }

    pub(crate) fn refusing(mut self, property: GUID, code: HRESULT) -> Self {
        self.script.refused.push((property, None, code));
        self
    }

    pub(crate) fn refusing_value(mut self, property: GUID, value: u32, code: HRESULT) -> Self {
        self.script.refused.push((property, Some(value), code));
        self
    }

    pub(crate) fn echoing_values(mut self) -> Self {
        self.script.echo_values = true;
        self
    }

    pub(crate) fn reading_back(mut self, property: GUID, value: u32) -> Self {
        self.script.read_back.push((property, value));
        self
    }

    pub(crate) fn calls(&self) -> Calls {
        Arc::clone(&self.calls)
    }

    fn state(&self) -> MutexGuard<'_, State> {
        self.state.lock().expect("state lock")
    }

    fn record(&self, kind: CallKind) {
        self.record_property(kind, None, None);
    }

    fn record_property(&self, kind: CallKind, property: Option<GUID>, value: Option<u32>) {
        self.calls.lock().expect("calls lock").push(Call {
            kind,
            thread: std::thread::current().id(),
            property,
            value,
        });
    }

    fn scripted_failure(&self, step: Step) -> MfResult<()> {
        let mut state = self.state();
        let nth = state.counts[step as usize];
        state.counts[step as usize] += 1;
        match self.script.fail {
            Some((at, when, code)) if at == step && when == nth => return Err(code.into()),
            _ => {}
        }
        match self.script.fail_from {
            Some((at, from, code)) if at == step && nth >= from => Err(code.into()),
            _ => Ok(()),
        }
    }

    fn queue(&self, kind: i32) {
        self.state().events.push_back(kind);
    }
}

pub(crate) fn device_manager() -> MfResult<IMFDXGIDeviceManager> {
    let mut token = 0u32;
    let mut manager = None;
    // SAFETY: the out pointers refer to live locals.
    unsafe { MFCreateDXGIDeviceManager(&mut token, &mut manager)? };
    manager.ok_or_else(|| E_NOTIMPL.into())
}

fn encoded_sample(bytes: &[u8], pts_hns: i64) -> MfResult<IMFSample> {
    // SAFETY: MFCreateMemoryBuffer has no pointer arguments.
    let buffer = unsafe { MFCreateMemoryBuffer(bytes.len() as u32)? };
    let mut data = std::ptr::null_mut();
    // SAFETY: the out pointer refers to a live local.
    unsafe { buffer.Lock(&mut data, None, None)? };
    // SAFETY: Lock returned a writable buffer of at least `bytes.len()` bytes until Unlock.
    unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), data, bytes.len()) };
    // SAFETY: balances the successful Lock above.
    unsafe { buffer.Unlock()? };
    // SAFETY: the buffer is live and not shared.
    unsafe { buffer.SetCurrentLength(bytes.len() as u32)? };
    // SAFETY: MFCreateSample has no pointer arguments.
    let sample = unsafe { MFCreateSample()? };
    // SAFETY: the sample and buffer are live and owned by this call.
    unsafe {
        sample.AddBuffer(&buffer)?;
        sample.SetSampleTime(pts_hns)?;
    }
    Ok(sample)
}

fn media_event(kind: i32, status: HRESULT) -> MfResult<IMFMediaEvent> {
    // SAFETY: the extended type pointer outlives the call and there is no value.
    unsafe { MFCreateMediaEvent(kind as u32, &GUID::zeroed(), status, None) }
}

impl Mft for FakeTransform {
    fn attributes(&self) -> MfResult<IMFAttributes> {
        self.record(CallKind::Attributes);
        self.scripted_failure(Step::Attributes)?;
        let mut store = None;
        // SAFETY: the out pointer refers to a live local.
        unsafe { MFCreateAttributes(&mut store, 2)? };
        let store: IMFAttributes = store.ok_or_else(|| windows::core::Error::from(E_POINTER))?;
        // SAFETY: the attribute store is live.
        unsafe {
            store.SetUINT32(&MF_TRANSFORM_ASYNC, 1)?;
            store.SetUINT32(&MF_SA_D3D11_AWARE, 1)?;
        }
        Ok(store)
    }

    fn stream_ids(&self, _inputs: &mut [u32], _outputs: &mut [u32]) -> MfResult<()> {
        self.record(CallKind::StreamIds);
        Err(E_NOTIMPL.into())
    }

    fn process_message(&self, message: MFT_MESSAGE_TYPE) -> MfResult<()> {
        self.record(CallKind::Message(message.0));
        self.scripted_failure(Step::ProcessMessage)?;
        if message == MFT_MESSAGE_COMMAND_FLUSH {
            std::thread::sleep(self.script.flush_block);
        }
        if message == MFT_MESSAGE_COMMAND_DRAIN && !self.script.never_drain_complete {
            self.queue(METransformDrainComplete.0);
        }
        Ok(())
    }

    fn set_d3d_manager(&self, _manager: &IMFDXGIDeviceManager) -> MfResult<()> {
        self.record(CallKind::SetD3dManager);
        self.scripted_failure(Step::SetD3dManager)
    }

    fn process_input(&self, _stream: u32, sample: &IMFSample) -> MfResult<()> {
        self.record(CallKind::ProcessInput);
        self.scripted_failure(Step::ProcessInput)?;
        if self.script.not_accepting {
            return Err(MF_E_NOTACCEPTING.into());
        }
        // SAFETY: the sample is live.
        let pts = unsafe { sample.GetSampleTime()? };
        if !self.script.never_output {
            self.state().pending_pts.push_back(pts);
            self.queue(METransformHaveOutput.0);
        }
        if !self.script.never_need_input {
            self.queue(METransformNeedInput.0);
        }
        Ok(())
    }

    fn process_output(
        &self,
        buffers: &mut [MFT_OUTPUT_DATA_BUFFER],
        _status: &mut u32,
    ) -> MfResult<()> {
        self.record(CallKind::ProcessOutput);
        self.scripted_failure(Step::ProcessOutput)?;
        let (pts, first) = {
            let mut state = self.state();
            let Some(pts) = state.pending_pts.pop_front() else {
                return Err(MF_E_TRANSFORM_NEED_MORE_INPUT.into());
            };
            let first = state.outputs == 0;
            state.outputs += 1;
            (pts, first)
        };
        let bytes = if first { IDR_PACKET } else { SLICE_PACKET };
        let sample = encoded_sample(&bytes, pts)?;
        if let Some(slot) = buffers.first_mut() {
            slot.pSample = ManuallyDrop::new(Some(sample));
        }
        Ok(())
    }

    fn output_stream_info(&self, _stream: u32) -> MfResult<MFT_OUTPUT_STREAM_INFO> {
        self.record(CallKind::OutputStreamInfo);
        self.scripted_failure(Step::OutputStreamInfo)?;
        Ok(MFT_OUTPUT_STREAM_INFO {
            dwFlags: MFT_OUTPUT_STREAM_PROVIDES_SAMPLES.0 as u32,
            cbSize: 0,
            cbAlignment: 0,
        })
    }

    fn input_stream_attributes(&self, _stream: u32) -> MfResult<IMFAttributes> {
        self.record(CallKind::InputStreamAttributes);
        Err(E_NOTIMPL.into())
    }

    fn input_available_type(&self, _stream: u32, _index: u32) -> MfResult<IMFMediaType> {
        self.record(CallKind::InputAvailableType);
        Err(E_NOTIMPL.into())
    }

    fn output_available_type(&self, _stream: u32, _index: u32) -> MfResult<IMFMediaType> {
        self.record(CallKind::OutputAvailableType);
        Err(E_NOTIMPL.into())
    }

    fn output_current_type(&self, _stream: u32) -> MfResult<IMFMediaType> {
        self.record(CallKind::OutputCurrentType);
        Err(E_NOTIMPL.into())
    }

    fn set_input_type(&self, _stream: u32, _media_type: &IMFMediaType) -> MfResult<()> {
        self.record(CallKind::SetInputType);
        self.scripted_failure(Step::SetInputType)
    }

    fn set_output_type(&self, _stream: u32, _media_type: &IMFMediaType) -> MfResult<()> {
        self.record(CallKind::SetOutputType);
        self.scripted_failure(Step::SetOutputType)
    }

    fn next_event(&self) -> MfResult<IMFMediaEvent> {
        self.record(CallKind::NextEvent);
        if let Err(error) = self.scripted_failure(Step::NextEvent) {
            return media_event(MEError.0, error.code());
        }
        let Some(kind) = self.state().events.pop_front() else {
            return Err(MF_E_NO_EVENTS_AVAILABLE.into());
        };
        media_event(kind, S_OK)
    }

    fn is_supported(&self, property: &GUID) -> HRESULT {
        self.record_property(CallKind::IsSupported, Some(*property), None);
        S_OK
    }

    fn set_value(&self, property: &GUID, value: &VARIANT) -> MfResult<()> {
        let number = u32::try_from(value)
            .ok()
            .or_else(|| bool::try_from(value).ok().map(u32::from));
        self.record_property(CallKind::SetValue, Some(*property), number);
        self.scripted_failure(Step::SetValue)?;
        let refusal =
            self.script.refused.iter().find(|(id, only, _)| {
                id == property && only.is_none_or(|only| Some(only) == number)
            });
        if let Some((_, _, code)) = refusal {
            return Err((*code).into());
        }
        if let Some(number) = number {
            self.state().values.insert(*property, number);
        }
        Ok(())
    }

    fn get_value(&self, property: &GUID) -> MfResult<VARIANT> {
        self.record_property(CallKind::GetValue, Some(*property), None);
        if let Some((_, value)) = self.script.read_back.iter().find(|(id, _)| id == property) {
            return Ok(VARIANT::from(*value));
        }
        let stored = self.state().values.get(property).copied();
        match stored {
            Some(value) if self.script.echo_values => Ok(VARIANT::from(value)),
            _ => Err(E_NOTIMPL.into()),
        }
    }

    fn shutdown(&self) {
        self.record(CallKind::Shutdown);
    }
}

#[derive(Debug)]
pub(crate) struct FakeCandidate {
    transform: Mutex<Option<FakeTransform>>,
    activation_failure: Option<HRESULT>,
    shutdown_objects: Arc<AtomicUsize>,
    name: String,
}

impl FakeCandidate {
    pub(crate) fn new(transform: FakeTransform) -> Self {
        Self {
            transform: Mutex::new(Some(transform)),
            activation_failure: None,
            shutdown_objects: Arc::default(),
            name: "fake encoder".to_string(),
        }
    }

    pub(crate) fn named(mut self, name: &str) -> Self {
        self.name = name.to_string();
        self
    }

    pub(crate) fn failing_activation(mut self, code: HRESULT) -> Self {
        self.activation_failure = Some(code);
        self
    }

    pub(crate) fn shutdown_objects(&self) -> Arc<AtomicUsize> {
        Arc::clone(&self.shutdown_objects)
    }
}

impl Candidate for FakeCandidate {
    fn activate(&self) -> MfResult<Box<dyn Mft>> {
        if let Some(code) = self.activation_failure {
            return Err(code.into());
        }
        let taken = self.transform.lock().expect("transform lock").take();
        taken
            .map(|transform| Box::new(transform) as Box<dyn Mft>)
            .ok_or_else(|| E_NOTIMPL.into())
    }

    fn shutdown_object(&self) {
        self.shutdown_objects.fetch_add(1, Ordering::SeqCst);
    }

    fn describe(&self) -> String {
        self.name.clone()
    }
}

#[derive(Debug, Default)]
pub(crate) struct SimulatedClock {
    elapsed: Cell<Duration>,
}

impl SimulatedClock {
    pub(crate) fn elapsed(&self) -> Duration {
        self.elapsed.get()
    }
}

impl DrainClock for SimulatedClock {
    fn now(&self) -> Duration {
        self.elapsed.get()
    }

    fn wait(&self, duration: Duration) {
        self.elapsed.set(self.elapsed.get() + duration);
    }
}
