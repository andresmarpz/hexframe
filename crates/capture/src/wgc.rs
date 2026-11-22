mod setup;
#[cfg(test)]
mod tests;

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use greyscreen_core::health::CaptureStats;
use greyscreen_core::media::CaptureTarget;
use greyscreen_core::tear::{Arrival, CopyWaits, HoldBack, Wait};
use greyscreen_core::time::{HNS_PER_SEC, Hns, mono_now};
use windows::Foundation::TypedEventHandler;
use windows::Graphics::Capture::{
    Direct3D11CaptureFramePool, GraphicsCaptureItem, GraphicsCaptureSession,
};
use windows::Graphics::DirectX::DirectXPixelFormat;
use windows::Graphics::SizeInt32;
use windows::Win32::Foundation::E_POINTER;
use windows::Win32::Graphics::Direct3D11::{D3D11_TEXTURE2D_DESC, ID3D11Texture2D};
use windows::Win32::System::WinRT::Direct3D11::IDirect3DDxgiInterfaceAccess;
use windows::core::{IInspectable, Interface};

use self::setup::{configure_session, create_item};
use crate::clock::QpcMap;
use crate::d3d::GpuDevice;
use crate::frame_copy::{Copied, FrameCopier};

const POOL_BUFFERS: i32 = 2;
const POOL_FORMAT: DirectXPixelFormat = DirectXPixelFormat::B8G8R8A8UIntNormalized;
const LOG_EVERY_FRAMES: u64 = 600;
const LOG_TIMEOUT_EVERY: u64 = 100;

#[derive(Debug, thiserror::Error)]
pub enum WgcError {
    #[error("windows graphics capture is not supported")]
    Unsupported,
    #[error("windows api error: {0}")]
    Windows(#[from] windows::core::Error),
}

pub type Result<T> = std::result::Result<T, WgcError>;

struct Latest {
    texture: ID3D11Texture2D,
    size: (u32, u32),
    at: Hns,
    seq: u64,
}

#[derive(Default)]
struct Published {
    current: Option<Latest>,
    pending: Option<Latest>,
    holdback: HoldBack,
    seq: u64,
}

struct Worker {
    copier: Option<FrameCopier>,
    thumb: Vec<u8>,
    pool_size: (i32, i32),
}

struct Taken {
    slot: usize,
    size: (u32, u32),
    copied: Copied,
}

struct Shared {
    gpu: GpuDevice,
    qpc: QpcMap,
    published: Mutex<Published>,
    worker: Mutex<Worker>,
    waits: Mutex<CopyWaits>,
    torn: AtomicU64,
    promote_after: Hns,
    closed: AtomicBool,
    frames: AtomicU64,
}

#[derive(Debug, Clone, Copy)]
pub struct WgcOptions {
    pub fps: u32,
    pub cursor: bool,
    pub display_hz: u32,
}

pub struct WgcCapture {
    shared: Arc<Shared>,
    item: Option<GraphicsCaptureItem>,
    pool: Option<Direct3D11CaptureFramePool>,
    session: Option<GraphicsCaptureSession>,
    frame_token: Option<i64>,
    closed_token: Option<i64>,
    border_hidden: bool,
}

impl std::fmt::Debug for WgcCapture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WgcCapture")
            .field("closed", &self.is_closed())
            .finish_non_exhaustive()
    }
}

impl WgcCapture {
    pub fn start(
        gpu: &GpuDevice,
        target: &CaptureTarget,
        qpc: QpcMap,
        options: WgcOptions,
    ) -> Result<Self> {
        if !GraphicsCaptureSession::IsSupported()? {
            return Err(WgcError::Unsupported);
        }
        fault!(CaptureStart)?;
        tracing::info!(step = "item", "graphics capture");
        let item = create_item(target)?;
        let item_size = item.Size()?;
        let winrt_device = gpu.winrt_device()?;
        tracing::info!(
            step = "frame pool",
            width = item_size.Width,
            height = item_size.Height,
            "graphics capture"
        );
        let pool = Direct3D11CaptureFramePool::CreateFreeThreaded(
            &winrt_device,
            POOL_FORMAT,
            POOL_BUFFERS,
            item_size,
        )?;
        let copier = FrameCopier::new(gpu)?;
        let shared = Arc::new(Shared {
            gpu: gpu.clone(),
            qpc,
            published: Mutex::new(Published::default()),
            worker: Mutex::new(Worker {
                copier: Some(copier),
                thumb: Vec::new(),
                pool_size: (item_size.Width, item_size.Height),
            }),
            waits: Mutex::new(CopyWaits::default()),
            torn: AtomicU64::new(0),
            promote_after: HNS_PER_SEC * 5 / 2 / i64::from(options.fps.max(1)),
            closed: AtomicBool::new(false),
            frames: AtomicU64::new(0),
        });

        let mut capture = Self {
            shared: Arc::clone(&shared),
            item: Some(item),
            pool: Some(pool),
            session: None,
            frame_token: None,
            closed_token: None,
            border_hidden: true,
        };
        tracing::info!(step = "session", "graphics capture");
        capture.attach(options)?;
        tracing::info!(
            width = item_size.Width,
            height = item_size.Height,
            "capture started"
        );
        Ok(capture)
    }

    fn attach(&mut self, options: WgcOptions) -> windows::core::Result<()> {
        let (Some(item), Some(pool)) = (self.item.clone(), self.pool.clone()) else {
            return Err(windows::core::Error::from(E_POINTER));
        };
        let closed_shared = Arc::clone(&self.shared);
        self.closed_token = Some(item.Closed(&TypedEventHandler::<
            GraphicsCaptureItem,
            IInspectable,
        >::new(move |_, _| {
            closed_shared.closed.store(true, Ordering::Release);
            tracing::info!("capture item closed");
            Ok(())
        }))?);
        let frame_shared = Arc::clone(&self.shared);
        self.frame_token = Some(pool.FrameArrived(&TypedEventHandler::<
            Direct3D11CaptureFramePool,
            IInspectable,
        >::new(move |sender, _| {
            let Some(pool) = sender.as_ref() else {
                return Ok(());
            };
            if let Err(error) = frame_shared.on_frame(pool) {
                tracing::warn!(%error, "capture frame failed");
            }
            Ok(())
        }))?);
        let session = pool.CreateCaptureSession(&item)?;
        self.session = Some(session.clone());
        self.border_hidden = configure_session(&session, options);
        tracing::info!(step = "start", "graphics capture");
        session.StartCapture()
    }

    pub fn latest(&self) -> Option<(ID3D11Texture2D, (u32, u32), u64)> {
        let mut published = lock(&self.shared.published);
        let stale = published
            .pending
            .as_ref()
            .is_some_and(|pending| mono_now() - pending.at >= self.shared.promote_after);
        if stale {
            published.holdback.promote();
            published.current = published.pending.take();
        }
        let latest = published
            .current
            .as_ref()
            .map(|latest| (latest.texture.clone(), latest.size, latest.seq))?;
        published.holdback.start_reading();
        Some(latest)
    }

    pub fn has_frame(&self) -> bool {
        let published = lock(&self.shared.published);
        published.current.is_some() || published.pending.is_some()
    }

    pub fn done_reading(&self) {
        lock(&self.shared.published).holdback.done_reading();
    }

    pub fn fill_stats(&self, stats: &mut CaptureStats) {
        let mut waits = lock(&self.shared.waits);
        stats.copy_wait_slow = waits.slow;
        stats.copy_wait_timeouts = waits.timeouts;
        stats.window_copy_wait_max_us = waits.max_us;
        waits.max_us = 0;
        stats.torn_dropped = self.shared.torn.load(Ordering::Relaxed);
    }

    pub fn is_closed(&self) -> bool {
        self.shared.closed.load(Ordering::Acquire)
    }

    pub fn border_hidden(&self) -> bool {
        self.border_hidden
    }

    pub fn stop(mut self) {
        self.teardown();
    }

    fn teardown(&mut self) {
        if self.pool.is_none() && self.session.is_none() {
            return;
        }
        if let (Some(pool), Some(token)) = (&self.pool, self.frame_token.take())
            && let Err(error) = pool.RemoveFrameArrived(token)
        {
            tracing::warn!(%error, "remove frame handler failed");
        }
        if let (Some(item), Some(token)) = (self.item.take(), self.closed_token.take())
            && let Err(error) = item.RemoveClosed(token)
        {
            tracing::warn!(%error, "remove closed handler failed");
        }
        if let Some(session) = self.session.take()
            && let Err(error) = session.Close()
        {
            tracing::warn!(%error, "close capture session failed");
        }
        if let Some(pool) = self.pool.take()
            && let Err(error) = pool.Close()
        {
            tracing::warn!(%error, "close frame pool failed");
        }
        *lock(&self.shared.published) = Published::default();
        let mut worker = lock(&self.shared.worker);
        if let Some(copier) = &mut worker.copier {
            copier.release();
        }
        let waits = *lock(&self.shared.waits);
        tracing::info!(
            frames = self.shared.frames.load(Ordering::Relaxed),
            torn_dropped = self.shared.torn.load(Ordering::Relaxed),
            copy_wait_slow = waits.slow,
            copy_wait_timeouts = waits.timeouts,
            "capture stopped"
        );
    }
}

impl Drop for WgcCapture {
    fn drop(&mut self) {
        self.teardown();
    }
}

impl Shared {
    fn on_frame(&self, pool: &Direct3D11CaptureFramePool) -> windows::core::Result<()> {
        let Ok(frame) = pool.TryGetNextFrame() else {
            return Ok(());
        };
        let content = frame.ContentSize()?;
        let at = self.qpc.to_mono(frame.SystemRelativeTime()?.Duration);
        let mut worker = lock(&self.worker);
        let copy = if content.Width > 0 && content.Height > 0 {
            let access: IDirect3DDxgiInterfaceAccess = frame.Surface()?.cast()?;
            // SAFETY: the surface is a live D3D11 texture owned by the checked-out frame.
            let source: ID3D11Texture2D = unsafe { access.GetInterface()? };
            let slot = lock(&self.published).holdback.write_slot();
            Some(worker.copy(&source, content, slot)?)
        } else {
            None
        };
        // The copy has finished on the GPU, so the pool may now reuse this buffer.
        drop(frame);
        self.resize_pool_if_needed(pool, content, &mut worker)?;
        if let Some(copy) = copy {
            self.publish(&mut worker, copy, at);
        }
        let count = self.frames.fetch_add(1, Ordering::Relaxed) + 1;
        if count.is_multiple_of(LOG_EVERY_FRAMES) {
            tracing::debug!(count, "capture frames");
        }
        Ok(())
    }

    fn publish(&self, worker: &mut Worker, copy: Taken, at: Hns) {
        let timeouts = {
            let mut waits = lock(&self.waits);
            waits.record(copy.copied.wait);
            waits.timeouts
        };
        if let Wait::TimedOut(waited) = copy.copied.wait {
            if timeouts % LOG_TIMEOUT_EVERY == 1 {
                tracing::warn!(
                    timeouts,
                    waited_ms = waited.as_secs_f64() * 1000.0,
                    "capture copy did not finish in time, frame skipped"
                );
            }
            return;
        }
        let Worker { copier, thumb, .. } = worker;
        let Some(texture) = copier.as_ref().and_then(|c| c.texture(copy.slot)) else {
            return;
        };
        let width = copier
            .as_ref()
            .filter(|_| copy.copied.thumbed)
            .and_then(|copier| {
                copier
                    .read_thumb(thumb)
                    .inspect_err(|error| tracing::debug!(%error, "thumbnail read failed"))
                    .ok()
            });
        let checked = width.is_some();
        let mut guard = lock(&self.published);
        let published = &mut *guard;
        let holdback = &mut published.holdback;
        if let Some(width) = width {
            holdback.store_thumb(copy.slot, thumb, width);
        }
        published.seq += 1;
        let entry = Latest {
            texture,
            size: copy.size,
            at,
            seq: published.seq,
        };
        if !checked {
            holdback.publish_now(copy.slot);
            published.pending = None;
            published.current = Some(entry);
            return;
        }
        match holdback.arrive(copy.slot) {
            Arrival::Held => {}
            Arrival::Published(_) => {
                if let Some(pending) = published.pending.take() {
                    published.current = Some(pending);
                }
            }
            Arrival::Dropped(_) => {
                published.pending = None;
                let torn = self.torn.fetch_add(1, Ordering::Relaxed) + 1;
                tracing::debug!(torn, "torn capture frame replaced by the previous one");
            }
        }
        published.pending = Some(entry);
    }

    fn resize_pool_if_needed(
        &self,
        pool: &Direct3D11CaptureFramePool,
        content: SizeInt32,
        worker: &mut Worker,
    ) -> windows::core::Result<()> {
        if content.Width <= 0 || content.Height <= 0 {
            return Ok(());
        }
        let wanted = (content.Width, content.Height);
        if worker.pool_size == wanted {
            return Ok(());
        }
        pool.Recreate(
            &self.gpu.winrt_device()?,
            POOL_FORMAT,
            POOL_BUFFERS,
            content,
        )?;
        worker.pool_size = wanted;
        tracing::info!(width = wanted.0, height = wanted.1, "capture pool resized");
        Ok(())
    }
}

impl Worker {
    fn copy(
        &mut self,
        source: &ID3D11Texture2D,
        content: SizeInt32,
        slot: usize,
    ) -> windows::core::Result<Taken> {
        let mut desc = D3D11_TEXTURE2D_DESC::default();
        // SAFETY: `desc` is a live out value and `source` is a live texture.
        unsafe { source.GetDesc(&mut desc) };
        let size = (
            (content.Width as u32).min(desc.Width),
            (content.Height as u32).min(desc.Height),
        );
        let copier = self
            .copier
            .as_mut()
            .ok_or_else(|| windows::core::Error::from(E_POINTER))?;
        let copied = copier.copy(source, size, slot)?;
        Ok(Taken { slot, size, copied })
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}
