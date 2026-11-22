use std::mem::size_of;
use std::time::{Duration, Instant};

use greyscreen_core::tear::{self, COPY_WAIT_BUDGET, RING_SLOTS, Wait};
use windows::Win32::Foundation::E_POINTER;
use windows::Win32::Graphics::Direct3D11::{
    D3D11_ASYNC_GETDATA_DONOTFLUSH, D3D11_BIND_RENDER_TARGET, D3D11_BIND_SHADER_RESOURCE,
    D3D11_BOX, D3D11_CPU_ACCESS_READ, D3D11_MAP_READ, D3D11_MAPPED_SUBRESOURCE, D3D11_QUERY_DESC,
    D3D11_QUERY_EVENT, D3D11_RESOURCE_MISC_GENERATE_MIPS, D3D11_TEXTURE2D_DESC, D3D11_USAGE,
    D3D11_USAGE_DEFAULT, D3D11_USAGE_STAGING, ID3D11Query, ID3D11ShaderResourceView,
    ID3D11Texture2D,
};
use windows::Win32::Graphics::Dxgi::Common::{DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_SAMPLE_DESC};
use windows::core::BOOL;

use crate::d3d::GpuDevice;

pub(crate) struct Copied {
    pub(crate) wait: Wait,
    pub(crate) thumbed: bool,
}

struct Thumb {
    source: ID3D11Texture2D,
    view: ID3D11ShaderResourceView,
    staging: ID3D11Texture2D,
    level: u32,
    size: (u32, u32),
}

pub(crate) struct FrameCopier {
    gpu: GpuDevice,
    query: ID3D11Query,
    size: (u32, u32),
    slots: Vec<ID3D11Texture2D>,
    thumb: Option<Thumb>,
    thumbs_off: bool,
}

impl FrameCopier {
    pub(crate) fn new(gpu: &GpuDevice) -> windows::core::Result<Self> {
        let desc = D3D11_QUERY_DESC {
            Query: D3D11_QUERY_EVENT,
            MiscFlags: 0,
        };
        let mut query = None;
        // SAFETY: `desc` and the out pointer refer to live locals.
        unsafe { gpu.device.CreateQuery(&desc, Some(&mut query))? };
        Ok(Self {
            gpu: gpu.clone(),
            query: query.ok_or_else(|| windows::core::Error::from(E_POINTER))?,
            size: (0, 0),
            slots: Vec::with_capacity(RING_SLOTS),
            thumb: None,
            thumbs_off: false,
        })
    }

    pub(crate) fn texture(&self, slot: usize) -> Option<ID3D11Texture2D> {
        self.slots.get(slot).cloned()
    }

    pub(crate) fn copy(
        &mut self,
        source: &ID3D11Texture2D,
        size: (u32, u32),
        slot: usize,
    ) -> windows::core::Result<Copied> {
        self.ensure(size)?;
        let target = self
            .slots
            .get(slot)
            .ok_or_else(|| windows::core::Error::from(E_POINTER))?;
        let region = whole(size);
        let context = &self.gpu.context;
        // SAFETY: both textures are live, the region lies inside both, and the context is multithread protected.
        unsafe { context.CopySubresourceRegion(target, 0, 0, 0, 0, source, 0, Some(&region)) };
        let thumbed = self.queue_thumb(source, &region);
        // SAFETY: the query and context are live; End and Flush take no other arguments.
        unsafe {
            context.End(&self.query);
            context.Flush();
        }
        fault!(GpuCopyWait)?;
        let started = Instant::now();
        let wait = tear::wait_bounded(
            COPY_WAIT_BUDGET,
            || started.elapsed(),
            || self.signaled(),
            pause,
        )?;
        Ok(Copied { wait, thumbed })
    }

    pub(crate) fn read_thumb(&self, out: &mut Vec<u8>) -> windows::core::Result<usize> {
        let thumb = self
            .thumb
            .as_ref()
            .ok_or_else(|| windows::core::Error::from(E_POINTER))?;
        let context = &self.gpu.context;
        let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
        fault!(StagingMap)?;
        // SAFETY: the staging texture is live, CPU readable and used only by this thread.
        unsafe { context.Map(&thumb.staging, 0, D3D11_MAP_READ, 0, Some(&mut mapped))? };
        let (width, height) = (thumb.size.0 as usize, thumb.size.1 as usize);
        out.clear();
        out.resize(width * height, 0);
        let base = mapped.pData.cast::<u8>().cast_const();
        for y in 0..height {
            // SAFETY: the mapping holds `height` rows of `RowPitch` bytes, each at least `width` BGRA pixels.
            let row = unsafe {
                std::slice::from_raw_parts(base.add(y * mapped.RowPitch as usize), width * 4)
            };
            for (x, pixel) in row.as_chunks::<4>().0.iter().enumerate() {
                out[y * width + x] = tear::luma(pixel[2], pixel[1], pixel[0]);
            }
        }
        // SAFETY: the texture was mapped above and `row` no longer borrows the mapping.
        unsafe { context.Unmap(&thumb.staging, 0) };
        Ok(width)
    }

    pub(crate) fn release(&mut self) {
        self.slots.clear();
        self.thumb = None;
        self.size = (0, 0);
    }

    fn signaled(&self) -> windows::core::Result<bool> {
        let mut done = BOOL(0);
        // SAFETY: the query is live and `done` is a BOOL, the data type of an event query.
        unsafe {
            self.gpu.context.GetData(
                &self.query,
                Some((&raw mut done).cast()),
                size_of::<BOOL>() as u32,
                D3D11_ASYNC_GETDATA_DONOTFLUSH.0 as u32,
            )?;
        }
        Ok(done.as_bool())
    }

    fn queue_thumb(&self, source: &ID3D11Texture2D, region: &D3D11_BOX) -> bool {
        let Some(thumb) = &self.thumb else {
            return false;
        };
        let small = whole(thumb.size);
        let context = &self.gpu.context;
        // SAFETY: all resources are live, the regions lie inside them and the context is multithread protected.
        unsafe {
            context.CopySubresourceRegion(&thumb.source, 0, 0, 0, 0, source, 0, Some(region));
            context.GenerateMips(&thumb.view);
            context.CopySubresourceRegion(
                &thumb.staging,
                0,
                0,
                0,
                0,
                &thumb.source,
                thumb.level,
                Some(&small),
            );
        }
        true
    }

    fn ensure(&mut self, size: (u32, u32)) -> windows::core::Result<()> {
        if self.size == size && self.slots.len() == RING_SLOTS {
            return Ok(());
        }
        self.release();
        for _ in 0..RING_SLOTS {
            let texture = texture(&self.gpu, frame_desc(size))?;
            self.slots.push(texture);
        }
        self.size = size;
        if !self.thumbs_off {
            match create_thumb(&self.gpu, size) {
                Ok(thumb) => self.thumb = Some(thumb),
                Err(error) => {
                    tracing::warn!(%error, "torn frame check is off: no thumbnail textures");
                    self.thumbs_off = true;
                }
            }
        }
        Ok(())
    }
}

fn pause(step: Duration) {
    if step.is_zero() {
        std::thread::yield_now();
    } else {
        std::thread::sleep(step);
    }
}

fn whole(size: (u32, u32)) -> D3D11_BOX {
    D3D11_BOX {
        left: 0,
        top: 0,
        front: 0,
        right: size.0,
        bottom: size.1,
        back: 1,
    }
}

fn create_thumb(gpu: &GpuDevice, size: (u32, u32)) -> windows::core::Result<Thumb> {
    let level = tear::thumb_level(size);
    let small = tear::thumb_size(size, level);
    let source = texture(
        gpu,
        D3D11_TEXTURE2D_DESC {
            MipLevels: level + 1,
            MiscFlags: D3D11_RESOURCE_MISC_GENERATE_MIPS.0 as u32,
            ..frame_desc(size)
        },
    )?;
    let mut view = None;
    // SAFETY: the texture is live and has the shader resource bind flag; the out pointer is a live local.
    unsafe {
        gpu.device
            .CreateShaderResourceView(&source, None, Some(&mut view))?
    };
    let staging = texture(
        gpu,
        D3D11_TEXTURE2D_DESC {
            CPUAccessFlags: D3D11_CPU_ACCESS_READ.0 as u32,
            ..plain_desc(small, D3D11_USAGE_STAGING)
        },
    )?;
    tracing::info!(
        level,
        width = small.0,
        height = small.1,
        "torn frame check thumbnail"
    );
    Ok(Thumb {
        source,
        view: view.ok_or_else(|| windows::core::Error::from(E_POINTER))?,
        staging,
        level,
        size: small,
    })
}

fn frame_desc(size: (u32, u32)) -> D3D11_TEXTURE2D_DESC {
    D3D11_TEXTURE2D_DESC {
        BindFlags: (D3D11_BIND_SHADER_RESOURCE.0 | D3D11_BIND_RENDER_TARGET.0) as u32,
        ..plain_desc(size, D3D11_USAGE_DEFAULT)
    }
}

fn plain_desc(size: (u32, u32), usage: D3D11_USAGE) -> D3D11_TEXTURE2D_DESC {
    D3D11_TEXTURE2D_DESC {
        Width: size.0,
        Height: size.1,
        MipLevels: 1,
        ArraySize: 1,
        Format: DXGI_FORMAT_B8G8R8A8_UNORM,
        SampleDesc: DXGI_SAMPLE_DESC {
            Count: 1,
            Quality: 0,
        },
        Usage: usage,
        BindFlags: 0,
        CPUAccessFlags: 0,
        MiscFlags: 0,
    }
}

fn texture(gpu: &GpuDevice, desc: D3D11_TEXTURE2D_DESC) -> windows::core::Result<ID3D11Texture2D> {
    fault!(FrameCopyTexture)?;
    let mut texture = None;
    // SAFETY: `desc` and the out pointer refer to live locals.
    unsafe {
        gpu.device
            .CreateTexture2D(&desc, None, Some(&mut texture))?
    };
    texture.ok_or_else(|| windows::core::Error::from(E_POINTER))
}
