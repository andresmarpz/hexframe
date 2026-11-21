use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use greyscreen_core::media::CaptureTarget;
use greyscreen_core::time::{HNS_PER_SEC, Hns};
use windows::Foundation::{TimeSpan, TypedEventHandler};
use windows::Graphics::Capture::{
    Direct3D11CaptureFramePool, GraphicsCaptureAccess, GraphicsCaptureAccessKind,
    GraphicsCaptureItem, GraphicsCaptureSession,
};
use windows::Graphics::DirectX::DirectXPixelFormat;
use windows::Graphics::SizeInt32;
use windows::Security::Authorization::AppCapabilityAccess::AppCapabilityAccessStatus;
use windows::Win32::Foundation::{E_POINTER, HWND, POINT};
use windows::Win32::Graphics::Direct3D11::{
    D3D11_BIND_RENDER_TARGET, D3D11_BIND_SHADER_RESOURCE, D3D11_BOX, D3D11_TEXTURE2D_DESC,
    D3D11_USAGE_DEFAULT, ID3D11Texture2D,
};
use windows::Win32::Graphics::Dxgi::Common::{DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_SAMPLE_DESC};
use windows::Win32::Graphics::Gdi::{HMONITOR, MONITOR_DEFAULTTOPRIMARY, MonitorFromPoint};
use windows::Win32::System::WinRT::Direct3D11::IDirect3DDxgiInterfaceAccess;
use windows::Win32::System::WinRT::Graphics::Capture::IGraphicsCaptureItemInterop;
use windows::core::{IInspectable, Interface, factory};

use crate::clock::QpcMap;
use crate::d3d::GpuDevice;

const POOL_BUFFERS: i32 = 2;
const POOL_FORMAT: DirectXPixelFormat = DirectXPixelFormat::B8G8R8A8UIntNormalized;
const LOG_EVERY_FRAMES: u64 = 600;

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

struct Worker {
    texture: Option<ID3D11Texture2D>,
    texture_size: (u32, u32),
    pool_size: (i32, i32),
}

struct Shared {
    gpu: GpuDevice,
    qpc: QpcMap,
    latest: Mutex<Option<Latest>>,
    worker: Mutex<Worker>,
    closed: AtomicBool,
    frames: AtomicU64,
}

pub struct WgcCapture {
    shared: Arc<Shared>,
    item: Option<GraphicsCaptureItem>,
    pool: Option<Direct3D11CaptureFramePool>,
    session: Option<GraphicsCaptureSession>,
    frame_token: Option<i64>,
    closed_token: Option<i64>,
}

impl std::fmt::Debug for WgcCapture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WgcCapture")
            .field("closed", &self.is_closed())
            .finish_non_exhaustive()
    }
}

impl WgcCapture {
    pub fn start(gpu: &GpuDevice, target: &CaptureTarget, qpc: QpcMap, fps: u32) -> Result<Self> {
        if !GraphicsCaptureSession::IsSupported()? {
            return Err(WgcError::Unsupported);
        }
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
        let shared = Arc::new(Shared {
            gpu: gpu.clone(),

            qpc,
            latest: Mutex::new(None),
            worker: Mutex::new(Worker {
                texture: None,
                texture_size: (0, 0),
                pool_size: (item_size.Width, item_size.Height),
            }),
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
        };
        tracing::info!(step = "session", "graphics capture");
        capture.attach(fps)?;
        tracing::info!(
            width = item_size.Width,
            height = item_size.Height,
            "capture started"
        );
        Ok(capture)
    }

    fn attach(&mut self, fps: u32) -> windows::core::Result<()> {
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
        configure_session(&session, fps);
        tracing::info!(step = "start", "graphics capture");
        session.StartCapture()
    }

    pub fn latest(&self) -> Option<(ID3D11Texture2D, (u32, u32), u64)> {
        lock(&self.shared.latest)
            .as_ref()
            .map(|latest| (latest.texture.clone(), latest.size, latest.seq))
    }

    pub fn latest_at(&self) -> Option<Hns> {
        lock(&self.shared.latest).as_ref().map(|latest| latest.at)
    }

    pub fn is_closed(&self) -> bool {
        self.shared.closed.load(Ordering::Acquire)
    }

    pub fn stop(mut self) {
        self.teardown();
    }

    fn teardown(&mut self) {
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
        *lock(&self.shared.latest) = None;
        let mut worker = lock(&self.shared.worker);
        worker.texture = None;
        worker.texture_size = (0, 0);
        tracing::info!(
            frames = self.shared.frames.load(Ordering::Relaxed),
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
        if content.Width > 0 && content.Height > 0 {
            let access: IDirect3DDxgiInterfaceAccess = frame.Surface()?.cast()?;
            // SAFETY: the surface is a live D3D11 texture owned by the checked-out frame.
            let source: ID3D11Texture2D = unsafe { access.GetInterface()? };
            self.copy_and_publish(&source, content, at)?;
        }
        drop(frame);
        self.resize_pool_if_needed(pool, content)?;
        let count = self.frames.fetch_add(1, Ordering::Relaxed) + 1;
        if count.is_multiple_of(LOG_EVERY_FRAMES) {
            tracing::debug!(count, "capture frames");
        }
        Ok(())
    }

    fn copy_and_publish(
        &self,
        source: &ID3D11Texture2D,
        content: SizeInt32,
        at: Hns,
    ) -> windows::core::Result<()> {
        let mut desc = D3D11_TEXTURE2D_DESC::default();
        // SAFETY: `desc` is a live out value and `source` is a live texture.
        unsafe { source.GetDesc(&mut desc) };
        let size = (
            (content.Width as u32).min(desc.Width),
            (content.Height as u32).min(desc.Height),
        );
        let texture = {
            let mut worker = lock(&self.worker);
            match &worker.texture {
                Some(texture) if worker.texture_size == size => texture.clone(),
                _ => {
                    let texture = self.create_texture(size)?;
                    worker.texture = Some(texture.clone());
                    worker.texture_size = size;
                    texture
                }
            }
        };
        let region = D3D11_BOX {
            left: 0,
            top: 0,
            front: 0,
            right: size.0,
            bottom: size.1,
            back: 1,
        };
        // SAFETY: both textures are live, the region lies inside both, and the context is multithread protected.
        unsafe {
            self.gpu
                .context
                .CopySubresourceRegion(&texture, 0, 0, 0, 0, source, 0, Some(&region));
        }
        let mut latest = lock(&self.latest);
        let seq = latest.as_ref().map_or(1, |previous| previous.seq + 1);
        *latest = Some(Latest {
            texture,
            size,
            at,
            seq,
        });
        Ok(())
    }

    fn create_texture(&self, size: (u32, u32)) -> windows::core::Result<ID3D11Texture2D> {
        let desc = D3D11_TEXTURE2D_DESC {
            Width: size.0,
            Height: size.1,
            MipLevels: 1,
            ArraySize: 1,
            Format: DXGI_FORMAT_B8G8R8A8_UNORM,
            SampleDesc: DXGI_SAMPLE_DESC {
                Count: 1,
                Quality: 0,
            },
            Usage: D3D11_USAGE_DEFAULT,
            BindFlags: (D3D11_BIND_SHADER_RESOURCE.0 | D3D11_BIND_RENDER_TARGET.0) as u32,
            CPUAccessFlags: 0,
            MiscFlags: 0,
        };
        let mut texture = None;
        // SAFETY: `desc` and the out pointer refer to live locals.
        unsafe {
            self.gpu
                .device
                .CreateTexture2D(&desc, None, Some(&mut texture))?;
        }
        texture.ok_or_else(|| windows::core::Error::from(E_POINTER))
    }

    fn resize_pool_if_needed(
        &self,
        pool: &Direct3D11CaptureFramePool,
        content: SizeInt32,
    ) -> windows::core::Result<()> {
        if content.Width <= 0 || content.Height <= 0 {
            return Ok(());
        }
        let wanted = (content.Width, content.Height);
        let mut worker = lock(&self.worker);
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

fn create_item(target: &CaptureTarget) -> windows::core::Result<GraphicsCaptureItem> {
    let interop = factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>()?;
    match target {
        CaptureTarget::GameWindow { hwnd, .. } => {
            let hwnd = HWND(*hwnd as *mut core::ffi::c_void);
            // SAFETY: CreateForWindow validates the handle and returns an error for an invalid one.
            unsafe { interop.CreateForWindow(hwnd) }
        }
        CaptureTarget::PrimaryMonitor => {
            // SAFETY: MonitorFromPoint takes a value point and always returns a monitor with DEFAULTTOPRIMARY.
            let monitor: HMONITOR =
                unsafe { MonitorFromPoint(POINT { x: 0, y: 0 }, MONITOR_DEFAULTTOPRIMARY) };
            // SAFETY: CreateForMonitor validates the handle and returns an error for an invalid one.
            unsafe { interop.CreateForMonitor(monitor) }
        }
    }
}

fn configure_session(session: &GraphicsCaptureSession, fps: u32) {
    if let Err(error) = session.SetIsCursorCaptureEnabled(false) {
        tracing::warn!(%error, "disable cursor capture failed");
    }
    tracing::info!(step = "borderless access", "graphics capture");
    request_borderless();
    if let Err(error) = session.SetIsBorderRequired(false) {
        tracing::warn!(%error, "disable capture border failed");
    }
    let interval = TimeSpan {
        Duration: HNS_PER_SEC * 9 / 10 / i64::from(fps.max(1)),
    };
    if let Err(error) = session.SetMinUpdateInterval(interval) {
        tracing::warn!(%error, "set minimum update interval failed");
    }
}

fn request_borderless() {
    let status = GraphicsCaptureAccess::RequestAccessAsync(GraphicsCaptureAccessKind::Borderless)
        .and_then(|operation| operation.join());
    match status {
        Ok(AppCapabilityAccessStatus::Allowed) => {}
        Ok(status) => tracing::warn!(status = status.0, "borderless capture access not allowed"),
        Err(error) => tracing::warn!(%error, "borderless capture access request failed"),
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}
