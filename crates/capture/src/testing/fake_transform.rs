use std::cell::Cell;
use std::collections::VecDeque;
use std::mem::ManuallyDrop;
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread::ThreadId;
use std::time::Duration;

use windows::Win32::Foundation::{E_NOTIMPL, S_OK};
use windows::Win32::Media::MediaFoundation::{
    IMFAttributes, IMFDXGIDeviceManager, IMFMediaEvent, IMFMediaType, IMFSample, MEError,
    METransformDrainComplete, METransformHaveOutput, METransformNeedInput,
    MF_E_NO_EVENTS_AVAILABLE, MF_E_NOTACCEPTING, MF_E_TRANSFORM_NEED_MORE_INPUT,
    MFCreateDXGIDeviceManager, MFCreateMediaEvent, MFCreateMemoryBuffer, MFCreateSample,
    MFT_MESSAGE_COMMAND_DRAIN, MFT_MESSAGE_COMMAND_FLUSH, MFT_MESSAGE_TYPE, MFT_OUTPUT_DATA_BUFFER,
    MFT_OUTPUT_STREAM_INFO, MFT_OUTPUT_STREAM_PROVIDES_SAMPLES,
};
use windows::Win32::System::Variant::VARIANT;
use windows::core::{GUID, HRESULT};

use crate::h264::DrainClock;
use crate::h264::transform::{MfResult, Mft};

const IDR_PACKET: [u8; 7] = [0, 0, 0, 1, 0x65, 0x88, 0x84];
const SLICE_PACKET: [u8; 7] = [0, 0, 0, 1, 0x41, 0x9a, 0x01];
const INITIAL_NEED_INPUT: u32 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Step {
    ProcessInput,
    ProcessOutput,
    NextEvent,
    SetValue,
}

impl Step {
    fn index(self) -> usize {
        match self {
            Step::ProcessInput => 0,
            Step::ProcessOutput => 1,
            Step::NextEvent => 2,
            Step::SetValue => 3,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CallKind {
    ProcessInput,
    ProcessOutput,
    Message(i32),
    NextEvent,
    SetValue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Call {
    pub(crate) kind: CallKind,
    pub(crate) thread: ThreadId,
}

pub(crate) type Calls = Arc<Mutex<Vec<Call>>>;

#[derive(Debug, Default)]
struct Script {
    fail: Option<(Step, usize, HRESULT)>,
    never_need_input: bool,
    never_drain_complete: bool,
    flush_block: Duration,
    not_accepting: bool,
}

#[derive(Debug, Default)]
struct State {
    events: VecDeque<i32>,
    pending_pts: VecDeque<i64>,
    counts: [usize; 4],
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

    pub(crate) fn never_needing_input(mut self) -> Self {
        self.script.never_need_input = true;
        self.state().events.clear();
        self
    }

    pub(crate) fn never_draining(mut self) -> Self {
        self.script.never_drain_complete = true;
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

    pub(crate) fn calls(&self) -> Calls {
        Arc::clone(&self.calls)
    }

    fn state(&self) -> MutexGuard<'_, State> {
        self.state.lock().expect("state lock")
    }

    fn record(&self, kind: CallKind) {
        self.calls.lock().expect("calls lock").push(Call {
            kind,
            thread: std::thread::current().id(),
        });
    }

    fn scripted_failure(&self, step: Step) -> MfResult<()> {
        let mut state = self.state();
        let nth = state.counts[step.index()];
        state.counts[step.index()] += 1;
        match self.script.fail {
            Some((at, when, code)) if at == step && when == nth => Err(code.into()),
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
        Err(E_NOTIMPL.into())
    }

    fn stream_ids(&self, _inputs: &mut [u32], _outputs: &mut [u32]) -> MfResult<()> {
        Err(E_NOTIMPL.into())
    }

    fn process_message(&self, message: MFT_MESSAGE_TYPE) -> MfResult<()> {
        self.record(CallKind::Message(message.0));
        if message == MFT_MESSAGE_COMMAND_FLUSH {
            std::thread::sleep(self.script.flush_block);
        }
        if message == MFT_MESSAGE_COMMAND_DRAIN && !self.script.never_drain_complete {
            self.queue(METransformDrainComplete.0);
        }
        Ok(())
    }

    fn set_d3d_manager(&self, _manager: &IMFDXGIDeviceManager) -> MfResult<()> {
        Ok(())
    }

    fn process_input(&self, _stream: u32, sample: &IMFSample) -> MfResult<()> {
        self.record(CallKind::ProcessInput);
        self.scripted_failure(Step::ProcessInput)?;
        if self.script.not_accepting {
            return Err(MF_E_NOTACCEPTING.into());
        }
        // SAFETY: the sample is live.
        let pts = unsafe { sample.GetSampleTime()? };
        self.state().pending_pts.push_back(pts);
        self.queue(METransformHaveOutput.0);
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
        Ok(MFT_OUTPUT_STREAM_INFO {
            dwFlags: MFT_OUTPUT_STREAM_PROVIDES_SAMPLES.0 as u32,
            cbSize: 0,
            cbAlignment: 0,
        })
    }

    fn input_stream_attributes(&self, _stream: u32) -> MfResult<IMFAttributes> {
        Err(E_NOTIMPL.into())
    }

    fn input_available_type(&self, _stream: u32, _index: u32) -> MfResult<IMFMediaType> {
        Err(E_NOTIMPL.into())
    }

    fn output_available_type(&self, _stream: u32, _index: u32) -> MfResult<IMFMediaType> {
        Err(E_NOTIMPL.into())
    }

    fn output_current_type(&self, _stream: u32) -> MfResult<IMFMediaType> {
        Err(E_NOTIMPL.into())
    }

    fn set_input_type(&self, _stream: u32, _media_type: &IMFMediaType) -> MfResult<()> {
        Ok(())
    }

    fn set_output_type(&self, _stream: u32, _media_type: &IMFMediaType) -> MfResult<()> {
        Ok(())
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

    fn is_supported(&self, _property: &GUID) -> HRESULT {
        S_OK
    }

    fn set_value(&self, _property: &GUID, _value: &VARIANT) -> MfResult<()> {
        self.record(CallKind::SetValue);
        self.scripted_failure(Step::SetValue)
    }

    fn get_value(&self, _property: &GUID) -> MfResult<VARIANT> {
        Err(E_NOTIMPL.into())
    }

    fn shutdown(&self) {}
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
