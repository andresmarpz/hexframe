use std::time::{Duration, Instant};

use greyscreen_core::crop::CropRect;
use greyscreen_core::encoder::{self, Attempt, Candidate, EncoderKind, Probe, Reason, nv12_report};
use greyscreen_core::media::{CaptureConfig, CaptureTarget, encode_size};
use greyscreen_core::time::HNS_PER_SEC;
use windows::Win32::Foundation::E_POINTER;
use windows::Win32::Graphics::Direct3D11::{
    D3D11_BIND_RENDER_TARGET, D3D11_BIND_SHADER_RESOURCE, D3D11_SUBRESOURCE_DATA,
    D3D11_TEXTURE2D_DESC, D3D11_USAGE_DEFAULT, ID3D11Texture2D,
};
use windows::Win32::Graphics::Dxgi::Common::{DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_SAMPLE_DESC};
use windows::Win32::Graphics::Gdi::{GetMonitorInfoW, MONITORINFO};

use crate::adapters;
use crate::com::{ComGuard, MfGuard};
use crate::d3d::{GpuDevice, target_monitor};
use crate::h264::{EncodedVideo, H264Encoder, PlaneMeans, nv12_plane_means, take_property_log};
use crate::vproc::{VideoConverter, VprocError};

mod deep;

pub use deep::{DeepAdapter, DeepRun, deep};

const FRAMES: i64 = 10;
const INPUT_WAIT: Duration = Duration::from_secs(2);
const INPUT_POLL: Duration = Duration::from_millis(1);
const FALLBACK_SIZE: (u32, u32) = (1920, 1080);

pub fn fingerprint() -> String {
    encoder::fingerprint(&adapters::for_target(&CaptureTarget::PrimaryMonitor))
}

pub fn check(cfg: &CaptureConfig, every: bool) -> Vec<Attempt> {
    let guards = ComGuard::mta().and_then(|com| Ok((com, MfGuard::start()?)));
    let adapters = adapters::for_target(&CaptureTarget::PrimaryMonitor);
    let native = primary_size();
    let mut attempts = Vec::new();
    for candidate in encoder::plan(&adapters) {
        let adapter = adapters.iter().find(|a| a.luid == candidate.adapter);
        let result = match &guards {
            Ok(_) => encode_test_frames(candidate, cfg, native),
            Err(error) => Err(format!("COM or Media Foundation failed: {error}")),
        };
        let ok = result.is_ok();
        attempts.push(Attempt {
            kind: candidate.kind,
            adapter: adapter.map_or_else(String::new, |a| a.name.clone()),
            vendor_id: adapter.map_or(0, |a| a.vendor_id),
            result,
            reason: Reason::Failed,
        });
        if ok && !every {
            break;
        }
    }
    drop(guards);
    attempts
}

pub fn probes() -> Vec<Probe> {
    let adapters = adapters::for_target(&CaptureTarget::PrimaryMonitor);
    encoder::plan(&adapters)
        .into_iter()
        .map(|candidate| {
            let adapter = adapters.iter().find(|a| a.luid == candidate.adapter);
            Probe {
                kind: candidate.kind,
                adapter: adapter.map_or_else(String::new, |a| a.name.clone()),
                vendor_id: adapter.map_or(0, |a| a.vendor_id),
                driver_version: adapter.map_or_else(String::new, |a| a.driver_version.clone()),
                luid: candidate.adapter,
            }
        })
        .collect()
}

pub fn probe(cfg: &CaptureConfig, probe: &Probe) -> Result<String, String> {
    let _com = ComGuard::mta().map_err(|e| format!("COM failed: {e}"))?;
    let _mf = MfGuard::start().map_err(|e| format!("Media Foundation failed: {e}"))?;
    let candidate = Candidate {
        adapter: probe.luid,
        kind: probe.kind,
    };
    encode_test_frames(candidate, cfg, primary_size())
}

fn primary_size() -> (u32, u32) {
    let mut info = MONITORINFO {
        cbSize: size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    let monitor = target_monitor(&CaptureTarget::PrimaryMonitor);
    // SAFETY: `info` is a live MONITORINFO with its size field set.
    if !unsafe { GetMonitorInfoW(monitor, &mut info) }.as_bool() {
        return FALLBACK_SIZE;
    }
    let rect = info.rcMonitor;
    let size = (
        (rect.right - rect.left) as u32,
        (rect.bottom - rect.top) as u32,
    );
    if size.0 == 0 || size.1 == 0 {
        FALLBACK_SIZE
    } else {
        size
    }
}

fn encode_test_frames(
    candidate: Candidate,
    cfg: &CaptureConfig,
    native: (u32, u32),
) -> Result<String, String> {
    let started = Instant::now();
    let run = encode_frames(candidate, cfg, native, FRAMES)?;
    if !run.packets.first().is_some_and(|p| p.key) {
        return Err(format!(
            "{} packets, none starts with a key frame",
            run.packets.len()
        ));
    }
    let nv12 = nv12_report(run.nv12.map(Into::into)).summary();
    Ok(format!(
        "{}, {} of {FRAMES} frames at {}x{} {} fps in {} ms, {nv12}",
        run.encoder,
        run.packets.len(),
        run.size.0,
        run.size.1,
        run.fps,
        started.elapsed().as_millis()
    ))
}

fn encode_frames(
    candidate: Candidate,
    cfg: &CaptureConfig,
    native: (u32, u32),
    frames: i64,
) -> Result<DeepRun, String> {
    let gpu = GpuDevice::for_luid(candidate.adapter).map_err(|e| format!("no device: {e}"))?;
    let cfg = match candidate.kind {
        EncoderKind::Hardware => cfg.clone(),
        EncoderKind::Software => encoder::software_config(cfg),
    };
    let size = encode_size(native, cfg.max_height);
    let video_cfg = cfg.for_encode_size(size.0, size.1);
    let source = test_pattern(&gpu, size).map_err(|e| format!("test frame: {e}"))?;
    let mut converter = VideoConverter::new(&gpu, size.0, size.1, cfg.fps)
        .map_err(|e| format!("video converter: {e}"))?;
    take_property_log();
    let mut encoder = H264Encoder::new(&gpu, candidate.kind, size.0, size.1, cfg.fps, &video_cfg)
        .map_err(|e| e.to_string())?
        .with_releaser(converter.releaser());
    let properties = take_property_log();
    let nv12 = plane_check(&gpu, &mut converter, size);
    let frame_hns = HNS_PER_SEC / i64::from(cfg.fps);
    let mut packets = Vec::new();
    for index in 0..frames {
        wait_for_input(&mut encoder, &mut packets)?;
        let busy_deadline = Instant::now() + INPUT_WAIT;
        let frame = loop {
            match converter.convert(&source, size, CropRect::full(size)) {
                Ok(frame) => break frame,
                Err(VprocError::Busy) => {
                    if Instant::now() >= busy_deadline {
                        return Err(format!(
                            "the converter stayed busy: the encoder holds {index} frames without output"
                        ));
                    }
                    encoder
                        .drain_output(&mut packets)
                        .map_err(|e| e.to_string())?;
                    std::thread::sleep(INPUT_POLL);
                }
                Err(error) => return Err(format!("video converter: {error}")),
            }
        };
        if let Err(error) = encoder.submit(&frame, index * frame_hns, frame_hns) {
            converter.release(frame.slot);
            return Err(error.to_string());
        }
        encoder
            .drain_output(&mut packets)
            .map_err(|e| e.to_string())?;
    }
    let (encoder_name, cpu_readback) = (encoder.encoder_name(), encoder.uses_cpu_readback());
    encoder.finish(&mut packets).map_err(|e| e.to_string())?;
    Ok(DeepRun {
        encoder: encoder_name,
        properties,
        cpu_readback,
        size,
        fps: cfg.fps,
        frames_in: frames,
        packets,
        nv12,
    })
}

fn plane_check(
    gpu: &GpuDevice,
    converter: &mut VideoConverter,
    size: (u32, u32),
) -> Option<PlaneMeans> {
    let ramp = ramp_pattern(gpu, size).ok()?;
    let frame = converter.convert(&ramp, size, CropRect::full(size)).ok()?;
    let means = nv12_plane_means(gpu, &frame.texture, size.0, size.1);
    converter.release(frame.slot);
    match means {
        Ok(means) => {
            tracing::info!(?means, "NV12 plane check through the converter");
            Some(means)
        }
        Err(error) => {
            tracing::warn!(%error, "NV12 plane check could not read the frame back");
            None
        }
    }
}

fn wait_for_input(
    encoder: &mut H264Encoder,
    packets: &mut Vec<EncodedVideo>,
) -> Result<(), String> {
    let deadline = Instant::now() + INPUT_WAIT;
    loop {
        if encoder.need_input().map_err(|e| e.to_string())? {
            return Ok(());
        }
        if Instant::now() >= deadline {
            return Err("the encoder never asked for a frame".to_owned());
        }
        encoder.drain_output(packets).map_err(|e| e.to_string())?;
        std::thread::sleep(INPUT_POLL);
    }
}

fn test_pattern(gpu: &GpuDevice, size: (u32, u32)) -> windows::core::Result<ID3D11Texture2D> {
    let (width, height) = (size.0 as usize, size.1 as usize);
    pattern(gpu, size, |x, y| {
        [(x * 255 / width) as u8, (y * 255 / height) as u8, 96, 255]
    })
}

fn ramp_pattern(gpu: &GpuDevice, size: (u32, u32)) -> windows::core::Result<ID3D11Texture2D> {
    let width = size.0 as usize;
    pattern(gpu, size, |x, _| {
        let grey = (x * 255 / width) as u8;
        [grey, grey, grey, 255]
    })
}

fn pattern(
    gpu: &GpuDevice,
    size: (u32, u32),
    pixel_at: impl Fn(usize, usize) -> [u8; 4],
) -> windows::core::Result<ID3D11Texture2D> {
    let (width, height) = (size.0 as usize, size.1 as usize);
    let mut pixels = vec![0u8; width * height * 4];
    for (index, pixel) in pixels.as_chunks_mut::<4>().0.iter_mut().enumerate() {
        *pixel = pixel_at(index % width, index / width);
    }
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
    let data = D3D11_SUBRESOURCE_DATA {
        pSysMem: pixels.as_ptr().cast(),
        SysMemPitch: (width * 4) as u32,
        SysMemSlicePitch: 0,
    };
    let mut texture = None;
    // SAFETY: the device is live; `desc`, `data` and the pixels it points to outlive the call.
    unsafe {
        gpu.device
            .CreateTexture2D(&desc, Some(&data), Some(&mut texture))?
    };
    texture.ok_or_else(|| windows::core::Error::from(E_POINTER))
}
