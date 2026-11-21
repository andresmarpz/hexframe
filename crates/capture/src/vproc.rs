use std::collections::HashMap;
use std::mem::ManuallyDrop;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use greyscreen_core::crop::CropRect;
use windows::Win32::Foundation::{E_POINTER, RECT};
use windows::Win32::Graphics::Direct3D11::{
    D3D11_BIND_RENDER_TARGET, D3D11_BIND_VIDEO_ENCODER, D3D11_TEX2D_VPIV, D3D11_TEX2D_VPOV,
    D3D11_TEXTURE2D_DESC, D3D11_USAGE_DEFAULT, D3D11_VIDEO_COLOR, D3D11_VIDEO_COLOR_0,
    D3D11_VIDEO_COLOR_YCbCrA, D3D11_VIDEO_FRAME_FORMAT_PROGRESSIVE,
    D3D11_VIDEO_PROCESSOR_CONTENT_DESC, D3D11_VIDEO_PROCESSOR_INPUT_VIEW_DESC,
    D3D11_VIDEO_PROCESSOR_INPUT_VIEW_DESC_0, D3D11_VIDEO_PROCESSOR_OUTPUT_VIEW_DESC,
    D3D11_VIDEO_PROCESSOR_OUTPUT_VIEW_DESC_0, D3D11_VIDEO_PROCESSOR_STREAM,
    D3D11_VIDEO_USAGE_PLAYBACK_NORMAL, D3D11_VPIV_DIMENSION_TEXTURE2D,
    D3D11_VPOV_DIMENSION_TEXTURE2D, ID3D11Texture2D, ID3D11VideoContext1, ID3D11VideoDevice,
    ID3D11VideoProcessor, ID3D11VideoProcessorEnumerator, ID3D11VideoProcessorEnumerator1,
    ID3D11VideoProcessorInputView, ID3D11VideoProcessorOutputView,
};
use windows::Win32::Graphics::Dxgi::Common::{
    DXGI_COLOR_SPACE_RGB_FULL_G22_NONE_P709, DXGI_COLOR_SPACE_YCBCR_STUDIO_G22_LEFT_P709,
    DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_FORMAT_NV12, DXGI_RATIONAL, DXGI_SAMPLE_DESC,
};
use windows::core::Interface;

use crate::d3d::GpuDevice;

const POOL_SIZE: usize = 6;
const DEFAULT_INPUT: (u32, u32) = (1920, 1080);
const MAX_CACHED_INPUT_VIEWS: usize = 32;

#[derive(Debug, thiserror::Error)]
pub enum VprocError {
    #[error("every NV12 pool texture is still in use")]
    Busy,
    #[error("the video processor cannot convert BGRA to NV12 with BT.709")]
    Unsupported,
    #[error("video processor call failed: {0}")]
    Windows(#[from] windows::core::Error),
}

pub type Result<T> = std::result::Result<T, VprocError>;

#[derive(Debug)]
pub struct Nv12Frame {
    pub texture: ID3D11Texture2D,
    pub slot: usize,
}

#[derive(Debug, Clone, Default)]
pub struct SlotReleaser {
    busy: Arc<AtomicU32>,
}

impl SlotReleaser {
    pub fn release(&self, slot: usize) {
        if slot < POOL_SIZE {
            self.busy.fetch_and(!(1 << slot), Ordering::AcqRel);
        }
    }

    fn acquire(&self) -> Option<usize> {
        (0..POOL_SIZE).find(|&slot| {
            let bit = 1 << slot;
            self.busy.fetch_or(bit, Ordering::AcqRel) & bit == 0
        })
    }
}

#[derive(Debug)]
pub struct VideoConverter {
    gpu: GpuDevice,
    video_device: ID3D11VideoDevice,
    video_context: ID3D11VideoContext1,
    enumerator: ID3D11VideoProcessorEnumerator,
    processor: ID3D11VideoProcessor,
    out_size: (u32, u32),
    fps: u32,
    input_size: (u32, u32),
    crop: CropRect,
    textures: Vec<ID3D11Texture2D>,
    output_views: Vec<ID3D11VideoProcessorOutputView>,
    input_views: HashMap<usize, ID3D11VideoProcessorInputView>,
    releaser: SlotReleaser,
}

impl VideoConverter {
    pub fn new(gpu: &GpuDevice, out_w: u32, out_h: u32, fps: u32) -> Result<Self> {
        let video_device: ID3D11VideoDevice = gpu.device.cast()?;
        let video_context: ID3D11VideoContext1 = gpu.context.cast()?;
        let out_size = (out_w, out_h);
        let (enumerator, processor) =
            create_processor(&video_device, DEFAULT_INPUT, out_size, fps)?;
        let textures = create_pool(gpu, out_size)?;
        let output_views = create_output_views(&video_device, &enumerator, &textures)?;
        let converter = Self {
            gpu: gpu.clone(),
            video_device,
            video_context,
            enumerator,
            processor,
            out_size,
            fps,
            input_size: DEFAULT_INPUT,
            crop: CropRect::full(DEFAULT_INPUT),
            textures,
            output_views,
            input_views: HashMap::new(),
            releaser: SlotReleaser::default(),
        };
        converter.configure();
        Ok(converter)
    }

    pub fn releaser(&self) -> SlotReleaser {
        self.releaser.clone()
    }

    pub fn release(&self, slot: usize) {
        self.releaser.release(slot);
    }

    pub fn convert(
        &mut self,
        src: &ID3D11Texture2D,
        src_size: (u32, u32),
        crop: CropRect,
    ) -> Result<Nv12Frame> {
        let crop = fit_crop(crop, src_size);
        if src_size != self.input_size {
            self.crop = crop;
            self.rebuild_for_input(src_size)?;
        } else if crop != self.crop {
            self.crop = crop;
            self.apply_rects();
        }
        let input_view = self.input_view(src)?;
        let slot = self.releaser.acquire().ok_or(VprocError::Busy)?;
        if let Err(error) = self.blt(&input_view, slot) {
            self.releaser.release(slot);
            return Err(error.into());
        }
        // SAFETY: the immediate context is live and Flush takes no arguments.
        unsafe { self.gpu.context.Flush() };
        Ok(Nv12Frame {
            texture: self.textures[slot].clone(),
            slot,
        })
    }

    fn blt(
        &self,
        input_view: &ID3D11VideoProcessorInputView,
        slot: usize,
    ) -> windows::core::Result<()> {
        let stream = D3D11_VIDEO_PROCESSOR_STREAM {
            Enable: true.into(),
            pInputSurface: ManuallyDrop::new(Some(input_view.clone())),
            ..Default::default()
        };
        let mut streams = [stream];
        // SAFETY: the processor, output view and stream view are live; `streams` outlives the call.
        let result = unsafe {
            self.video_context.VideoProcessorBlt(
                &self.processor,
                &self.output_views[slot],
                0,
                &streams,
            )
        };
        for stream in &mut streams {
            // SAFETY: each stream owns the one reference cloned above and is not used afterwards.
            unsafe { ManuallyDrop::drop(&mut stream.pInputSurface) };
        }
        result
    }

    fn input_view(
        &mut self,
        src: &ID3D11Texture2D,
    ) -> windows::core::Result<ID3D11VideoProcessorInputView> {
        let key = src.as_raw() as usize;
        if let Some(view) = self.input_views.get(&key) {
            return Ok(view.clone());
        }
        if self.input_views.len() >= MAX_CACHED_INPUT_VIEWS {
            self.input_views.clear();
        }
        let desc = D3D11_VIDEO_PROCESSOR_INPUT_VIEW_DESC {
            FourCC: 0,
            ViewDimension: D3D11_VPIV_DIMENSION_TEXTURE2D,
            Anonymous: D3D11_VIDEO_PROCESSOR_INPUT_VIEW_DESC_0 {
                Texture2D: D3D11_TEX2D_VPIV {
                    MipSlice: 0,
                    ArraySlice: 0,
                },
            },
        };
        let mut view = None;
        // SAFETY: the texture and enumerator are live; `desc` and `view` outlive the call.
        unsafe {
            self.video_device.CreateVideoProcessorInputView(
                src,
                &self.enumerator,
                &desc,
                Some(&mut view),
            )?;
        }
        let view = view.ok_or_else(|| windows::core::Error::from(E_POINTER))?;
        self.input_views.insert(key, view.clone());
        Ok(view)
    }

    fn rebuild_for_input(&mut self, src_size: (u32, u32)) -> Result<()> {
        let (enumerator, processor) =
            create_processor(&self.video_device, src_size, self.out_size, self.fps)?;
        self.output_views = create_output_views(&self.video_device, &enumerator, &self.textures)?;
        self.enumerator = enumerator;
        self.processor = processor;
        self.input_size = src_size;
        self.input_views.clear();
        self.configure();
        Ok(())
    }

    fn configure(&self) {
        let black = D3D11_VIDEO_COLOR {
            Anonymous: D3D11_VIDEO_COLOR_0 {
                YCbCr: D3D11_VIDEO_COLOR_YCbCrA {
                    Y: 16.0 / 255.0,
                    Cb: 128.0 / 255.0,
                    Cr: 128.0 / 255.0,
                    A: 1.0,
                },
            },
        };
        let target = rect(0, 0, self.out_size.0, self.out_size.1);
        // SAFETY: the processor is live and the colour and rects outlive the calls.
        unsafe {
            self.video_context.VideoProcessorSetStreamColorSpace1(
                &self.processor,
                0,
                DXGI_COLOR_SPACE_RGB_FULL_G22_NONE_P709,
            );
            self.video_context.VideoProcessorSetOutputColorSpace1(
                &self.processor,
                DXGI_COLOR_SPACE_YCBCR_STUDIO_G22_LEFT_P709,
            );
            self.video_context.VideoProcessorSetOutputBackgroundColor(
                &self.processor,
                true,
                &black,
            );
            self.video_context.VideoProcessorSetStreamFrameFormat(
                &self.processor,
                0,
                D3D11_VIDEO_FRAME_FORMAT_PROGRESSIVE,
            );
            // The target is the whole output so the background colour fills the borders.
            self.video_context.VideoProcessorSetOutputTargetRect(
                &self.processor,
                true,
                Some(&target),
            );
        }
        self.apply_rects();
    }

    fn apply_rects(&self) {
        let source = rect(
            self.crop.left,
            self.crop.top,
            self.crop.left + self.crop.width,
            self.crop.top + self.crop.height,
        );
        let dest = letterbox(self.crop.size(), self.out_size);
        // SAFETY: the processor is live and the rects outlive the calls.
        unsafe {
            self.video_context.VideoProcessorSetStreamSourceRect(
                &self.processor,
                0,
                true,
                Some(&source),
            );
            self.video_context.VideoProcessorSetStreamDestRect(
                &self.processor,
                0,
                true,
                Some(&dest),
            );
        }
    }
}

fn fit_crop(crop: CropRect, frame: (u32, u32)) -> CropRect {
    let fits = crop.width > 0
        && crop.height > 0
        && crop.left.saturating_add(crop.width) <= frame.0
        && crop.top.saturating_add(crop.height) <= frame.1;
    if fits { crop } else { CropRect::full(frame) }
}

fn rect(left: u32, top: u32, right: u32, bottom: u32) -> RECT {
    RECT {
        left: left as i32,
        top: top as i32,
        right: right as i32,
        bottom: bottom as i32,
    }
}

fn letterbox(src: (u32, u32), out: (u32, u32)) -> RECT {
    let (src_w, src_h) = (u64::from(src.0.max(1)), u64::from(src.1.max(1)));
    let (out_w, out_h) = (u64::from(out.0), u64::from(out.1));
    let (width, height) = if out_w * src_h <= out_h * src_w {
        (out_w, (out_w * src_h / src_w).max(2))
    } else {
        ((out_h * src_w / src_h).max(2), out_h)
    };
    let width = (width & !1).min(out_w) as u32;
    let height = (height & !1).min(out_h) as u32;
    let left = ((out.0 - width) / 2) & !1;
    let top = ((out.1 - height) / 2) & !1;
    rect(left, top, left + width, top + height)
}

fn create_processor(
    video_device: &ID3D11VideoDevice,
    input: (u32, u32),
    output: (u32, u32),
    fps: u32,
) -> Result<(ID3D11VideoProcessorEnumerator, ID3D11VideoProcessor)> {
    let rate = DXGI_RATIONAL {
        Numerator: fps,
        Denominator: 1,
    };
    let desc = D3D11_VIDEO_PROCESSOR_CONTENT_DESC {
        InputFrameFormat: D3D11_VIDEO_FRAME_FORMAT_PROGRESSIVE,
        InputFrameRate: rate,
        InputWidth: input.0,
        InputHeight: input.1,
        OutputFrameRate: rate,
        OutputWidth: output.0,
        OutputHeight: output.1,
        Usage: D3D11_VIDEO_USAGE_PLAYBACK_NORMAL,
    };
    // SAFETY: the video device is live and `desc` outlives the call.
    let enumerator = unsafe { video_device.CreateVideoProcessorEnumerator(&desc)? };
    let checker: ID3D11VideoProcessorEnumerator1 = enumerator.cast()?;
    // SAFETY: the enumerator is live and the call only reads its arguments.
    let supported = unsafe {
        checker.CheckVideoProcessorFormatConversion(
            DXGI_FORMAT_B8G8R8A8_UNORM,
            DXGI_COLOR_SPACE_RGB_FULL_G22_NONE_P709,
            DXGI_FORMAT_NV12,
            DXGI_COLOR_SPACE_YCBCR_STUDIO_G22_LEFT_P709,
        )?
    };
    if !supported.as_bool() {
        return Err(VprocError::Unsupported);
    }
    // SAFETY: the video device and enumerator are live.
    let processor = unsafe { video_device.CreateVideoProcessor(&enumerator, 0)? };
    Ok((enumerator, processor))
}

fn create_pool(gpu: &GpuDevice, size: (u32, u32)) -> windows::core::Result<Vec<ID3D11Texture2D>> {
    let render_flags = D3D11_BIND_RENDER_TARGET.0 as u32;
    let encoder_flags = render_flags | D3D11_BIND_VIDEO_ENCODER.0 as u32;
    let (first, flags) = match create_nv12_texture(gpu, size, encoder_flags) {
        Ok(texture) => (texture, encoder_flags),
        Err(error) => {
            tracing::warn!(%error, "NV12 texture with VIDEO_ENCODER bind failed, using RENDER_TARGET");
            (create_nv12_texture(gpu, size, render_flags)?, render_flags)
        }
    };
    let mut textures = vec![first];
    for _ in 1..POOL_SIZE {
        textures.push(create_nv12_texture(gpu, size, flags)?);
    }
    Ok(textures)
}

fn create_nv12_texture(
    gpu: &GpuDevice,
    size: (u32, u32),
    bind_flags: u32,
) -> windows::core::Result<ID3D11Texture2D> {
    let desc = D3D11_TEXTURE2D_DESC {
        Width: size.0,
        Height: size.1,
        MipLevels: 1,
        ArraySize: 1,
        Format: DXGI_FORMAT_NV12,
        SampleDesc: DXGI_SAMPLE_DESC {
            Count: 1,
            Quality: 0,
        },
        Usage: D3D11_USAGE_DEFAULT,
        BindFlags: bind_flags,
        CPUAccessFlags: 0,
        MiscFlags: 0,
    };
    let mut texture = None;
    // SAFETY: the device is live; `desc` and `texture` outlive the call.
    unsafe {
        gpu.device
            .CreateTexture2D(&desc, None, Some(&mut texture))?
    };
    texture.ok_or_else(|| windows::core::Error::from(E_POINTER))
}

fn create_output_views(
    video_device: &ID3D11VideoDevice,
    enumerator: &ID3D11VideoProcessorEnumerator,
    textures: &[ID3D11Texture2D],
) -> windows::core::Result<Vec<ID3D11VideoProcessorOutputView>> {
    textures
        .iter()
        .map(|texture| {
            let desc = D3D11_VIDEO_PROCESSOR_OUTPUT_VIEW_DESC {
                ViewDimension: D3D11_VPOV_DIMENSION_TEXTURE2D,
                Anonymous: D3D11_VIDEO_PROCESSOR_OUTPUT_VIEW_DESC_0 {
                    Texture2D: D3D11_TEX2D_VPOV { MipSlice: 0 },
                },
            };
            let mut view = None;
            // SAFETY: the texture and enumerator are live; `desc` and `view` outlive the call.
            unsafe {
                video_device.CreateVideoProcessorOutputView(
                    texture,
                    enumerator,
                    &desc,
                    Some(&mut view),
                )?;
            }
            view.ok_or_else(|| windows::core::Error::from(E_POINTER))
        })
        .collect()
}
