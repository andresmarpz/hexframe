//! The device side of a clip's sound: a shared-mode render stream that lends buffer space, plays
//! what it gets and reports how far it played. `Wasapi` is the default output device; tests
//! built with the `qa` feature play into `fake` instead.

use super::super::mf::Sound;
use super::BUFFER;
use std::ops::DerefMut;
use std::time::Duration;
use windows::Win32::Foundation::{CloseHandle, HANDLE, WAIT_OBJECT_0};
use windows::Win32::Media::Audio::*;
use windows::Win32::System::Com::{CLSCTX_ALL, CoCreateInstance};
use windows::Win32::System::Threading::{CreateEventW, WaitForSingleObject};

pub(super) trait Endpoint {
    type Lease<'a>: DerefMut<Target = [f32]>
    where
        Self: 'a;
    /// Frames the device's buffer holds.
    fn frames(&self) -> u32;
    /// The stream's latency as the device reports it.
    fn latency(&self) -> Duration;
    /// Frames written and not played yet; None when the device can't tell.
    fn padding(&self) -> Option<u32>;
    /// Room for `frames` interleaved frames, written until `release`.
    fn lease(&self, frames: u32) -> Option<Self::Lease<'_>>;
    fn release(&self, frames: u32);
    fn start(&self) -> bool;
    /// Stops the stream and drops what it hasn't played.
    fn halt(&self);
    /// Waits up to `patience` for the device to ask for more; false when it didn't.
    fn wait(&self, patience: Duration) -> bool;
    /// Seconds the device has played, and how long ago it read that; None without a usable
    /// position.
    fn played(&self) -> Option<(f64, f64)>;
}

#[cfg_attr(feature = "qa", allow(dead_code))]
pub(super) struct Wasapi {
    client: IAudioClient,
    render: IAudioRenderClient,
    event: HANDLE,
    frames: u32,
    channels: usize,
    latency: Duration,
    /// The device's position counter and its ticks per second, when it has one.
    position: Option<(IAudioClock, u64)>,
}

#[cfg_attr(feature = "qa", allow(dead_code))]
impl Wasapi {
    pub(super) fn new(sound: Sound) -> Result<Self, String> {
        // SAFETY: COM calls on this thread's objects; the format describes interleaved float.
        unsafe {
            let devices: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).map_err(|e| e.message())?;
            let device = devices.GetDefaultAudioEndpoint(eRender, eConsole).map_err(|_| "no sound output device".to_string())?;
            let client: IAudioClient = device.Activate(CLSCTX_ALL, None).map_err(|e| e.message())?;
            let block = 4 * sound.channels;
            let format = WAVEFORMATEX {
                wFormatTag: 3, // WAVE_FORMAT_IEEE_FLOAT
                nChannels: sound.channels,
                nSamplesPerSec: sound.rate,
                nAvgBytesPerSec: sound.rate * u32::from(block),
                nBlockAlign: block,
                wBitsPerSample: 32,
                cbSize: 0,
            };
            let flags = AUDCLNT_STREAMFLAGS_EVENTCALLBACK | AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM | AUDCLNT_STREAMFLAGS_SRC_DEFAULT_QUALITY;
            client.Initialize(AUDCLNT_SHAREMODE_SHARED, flags, BUFFER.as_nanos() as i64 / 100, 0, &format, None).map_err(|e| e.message())?;
            let event = CreateEventW(None, false, false, None).map_err(|e| e.message())?;
            client.SetEventHandle(event).map_err(|e| e.message())?;
            let render: IAudioRenderClient = client.GetService().map_err(|e| e.message())?;
            let frames = client.GetBufferSize().map_err(|e| e.message())?;
            let latency = Duration::from_nanos(client.GetStreamLatency().unwrap_or(0).max(0) as u64 * 100);
            let position = client.GetService::<IAudioClock>().ok().and_then(|c| c.GetFrequency().ok().filter(|f| *f > 0).map(|f| (c, f)));
            Ok(Self { client, render, event, frames, channels: usize::from(sound.channels), latency, position })
        }
    }
}

impl Endpoint for Wasapi {
    type Lease<'a> = &'a mut [f32];

    fn frames(&self) -> u32 {
        self.frames
    }

    fn latency(&self) -> Duration {
        self.latency
    }

    fn padding(&self) -> Option<u32> {
        // SAFETY: a plain query on this stream.
        unsafe { self.client.GetCurrentPadding() }.ok()
    }

    fn lease(&self, frames: u32) -> Option<&mut [f32]> {
        // SAFETY: the device lends `frames` frames of interleaved float until `ReleaseBuffer`, and
        // the sound thread holds one lease at a time.
        unsafe {
            let data = self.render.GetBuffer(frames).ok()?;
            Some(std::slice::from_raw_parts_mut(data.cast::<f32>(), frames as usize * self.channels))
        }
    }

    fn release(&self, frames: u32) {
        // SAFETY: gives back the frames `lease` lent, all written.
        let _ = unsafe { self.render.ReleaseBuffer(frames, 0) };
    }

    fn start(&self) -> bool {
        // SAFETY: starts this stream.
        unsafe { self.client.Start() }.is_ok()
    }

    fn halt(&self) {
        // SAFETY: stops and empties this stream.
        unsafe {
            let _ = self.client.Stop();
            let _ = self.client.Reset();
        }
    }

    fn wait(&self, patience: Duration) -> bool {
        // SAFETY: a wait on this stream's own event.
        unsafe { WaitForSingleObject(self.event, patience.as_millis() as u32) == WAIT_OBJECT_0 }
    }

    fn played(&self) -> Option<(f64, f64)> {
        let (clock, frequency) = self.position.as_ref()?;
        let (mut position, mut counter) = (0u64, 0u64);
        // SAFETY: reads into locals from this stream's clock.
        unsafe { clock.GetPosition(&mut position, Some(&mut counter)) }.ok()?;
        Some((position as f64 / *frequency as f64, performance_seconds() - counter as f64 / 1e7))
    }
}

impl Drop for Wasapi {
    fn drop(&mut self) {
        // SAFETY: stops this stream and closes its event.
        unsafe {
            let _ = self.client.Stop();
            let _ = CloseHandle(self.event);
        }
    }
}

/// The performance counter now, in seconds: the clock the device's position is stamped with.
#[cfg_attr(feature = "qa", allow(dead_code))]
fn performance_seconds() -> f64 {
    let (mut count, mut frequency) = (0i64, 0i64);
    // SAFETY: plain reads of the performance counter into locals.
    unsafe {
        windows_sys::Win32::System::Performance::QueryPerformanceCounter(&mut count);
        windows_sys::Win32::System::Performance::QueryPerformanceFrequency(&mut frequency);
    }
    count as f64 / frequency.max(1) as f64
}
