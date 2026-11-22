use std::time::Duration;

use greyscreen_core::tear::{RING_SLOTS, Wait};
use windows::Win32::Foundation::{E_FAIL, E_OUTOFMEMORY};
use windows::Win32::Graphics::Direct3D11::{
    D3D11_CPU_ACCESS_READ, D3D11_MAP_READ, D3D11_MAPPED_SUBRESOURCE, D3D11_TEXTURE2D_DESC,
    D3D11_USAGE_STAGING, ID3D11Texture2D,
};
use windows::core::Interface;

use super::{FrameCopier, frame_desc, luma_rows, plain_desc, texture, whole};
use crate::fault::{Armed, Point};

const SIZE: (u32, u32) = (64, 64);

#[test]
fn a_copy_finishes_with_nothing_armed() {
    let _armed = Armed::new();
    let gpu = crate::testing::warp::device().unwrap();
    let source = texture(&gpu, frame_desc(SIZE)).unwrap();
    let mut copier = FrameCopier::new(&gpu).unwrap();
    let copied = copier.copy(&source, SIZE, 0).unwrap();
    assert!(matches!(copied.wait, Wait::Done(_)));
}

#[test]
fn an_armed_stall_on_the_copy_wait_times_the_copy_out() {
    let _armed = Armed::new().stall(Point::GpuCopyWait, Duration::from_millis(1));
    let gpu = crate::testing::warp::device().unwrap();
    let source = texture(&gpu, frame_desc(SIZE)).unwrap();
    let mut copier = FrameCopier::new(&gpu).unwrap();
    let copied = copier.copy(&source, SIZE, 0).unwrap();
    assert!(matches!(copied.wait, Wait::TimedOut(_)));
}

fn desc_of(
    texture: &windows::Win32::Graphics::Direct3D11::ID3D11Texture2D,
) -> D3D11_TEXTURE2D_DESC {
    let mut desc = D3D11_TEXTURE2D_DESC::default();
    // SAFETY: `desc` is a live out value and the texture is live.
    unsafe { texture.GetDesc(&mut desc) };
    desc
}

fn assert_ring_matches(copier: &FrameCopier, size: (u32, u32)) {
    for slot in 0..RING_SLOTS {
        let desc = desc_of(&copier.texture(slot).unwrap());
        assert_eq!((desc.Width, desc.Height), size);
    }
    let region = whole(size);
    let desc = desc_of(&copier.texture(0).unwrap());
    assert!(region.right <= desc.Width && region.bottom <= desc.Height);
}

#[test]
fn a_change_in_either_dimension_recreates_the_ring_and_the_region_stays_inside() {
    let _armed = Armed::new();
    let gpu = crate::testing::warp::device().unwrap();
    let mut copier = FrameCopier::new(&gpu).unwrap();
    for size in [(1920, 1080), (1920, 1081), (1921, 1080), (1921, 1080)] {
        let source = plain_texture(&gpu, size);
        let copied = copier.copy(&source, size, 0).unwrap();
        assert!(matches!(copied.wait, Wait::Done(_)));
        assert_eq!(copier.size, size);
        assert_ring_matches(&copier, size);
    }
}

const RED: [f32; 4] = [0.0, 0.0, 1.0, 1.0];
const BLUE: [f32; 4] = [1.0, 0.0, 0.0, 1.0];
const RED_PIXEL: [u8; 4] = [0, 0, 255, 255];

fn filled_texture(
    gpu: &crate::d3d::GpuDevice,
    size: (u32, u32),
    colour: [f32; 4],
) -> ID3D11Texture2D {
    let source = plain_texture(gpu, size);
    let mut view = None;
    // SAFETY: the device and texture are live and the out pointer outlives the call.
    unsafe {
        gpu.device
            .CreateRenderTargetView(&source, None, Some(&mut view))
            .unwrap()
    };
    // SAFETY: the view is live and the colour is four floats.
    unsafe { gpu.context.ClearRenderTargetView(&view.unwrap(), &colour) };
    source
}

fn pixels_of(gpu: &crate::d3d::GpuDevice, source: &ID3D11Texture2D, size: (u32, u32)) -> Vec<u8> {
    let staging_desc = D3D11_TEXTURE2D_DESC {
        CPUAccessFlags: D3D11_CPU_ACCESS_READ.0 as u32,
        ..plain_desc(size, D3D11_USAGE_STAGING)
    };
    let staging = texture(gpu, staging_desc).unwrap();
    let region = whole(size);
    let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
    // SAFETY: both textures are live, the region lies inside both, the staging texture is CPU readable and the mapping is released before return.
    unsafe {
        gpu.context
            .CopySubresourceRegion(&staging, 0, 0, 0, 0, source, 0, Some(&region));
        gpu.context
            .Map(&staging, 0, D3D11_MAP_READ, 0, Some(&mut mapped))
            .unwrap();
    }
    let pitch = mapped.RowPitch as usize;
    let mut out = Vec::new();
    for row in 0..size.1 as usize {
        // SAFETY: the mapping holds `size.1` rows of `RowPitch` bytes, each at least `size.0` pixels.
        let line = unsafe {
            std::slice::from_raw_parts(
                mapped.pData.cast::<u8>().cast_const().add(row * pitch),
                size.0 as usize * 4,
            )
        };
        out.extend_from_slice(line);
    }
    // SAFETY: the texture was mapped above and `out` no longer borrows the mapping.
    unsafe { gpu.context.Unmap(&staging, 0) };
    out
}

#[test]
fn a_size_change_rebuilds_every_slot_and_a_held_old_slot_keeps_its_pixels() {
    let _armed = Armed::new();
    let gpu = crate::testing::warp::device().unwrap();
    let mut copier = FrameCopier::new(&gpu).unwrap();
    copier
        .copy(&filled_texture(&gpu, SIZE, RED), SIZE, 0)
        .unwrap();
    let held: Vec<_> = (0..RING_SLOTS)
        .map(|slot| copier.texture(slot).unwrap())
        .collect();

    let new_size = (SIZE.0 + 2, SIZE.1 + 2);
    copier
        .copy(&filled_texture(&gpu, new_size, BLUE), new_size, 0)
        .unwrap();

    for (slot, old) in held.iter().enumerate() {
        assert_ne!(old.as_raw(), copier.texture(slot).unwrap().as_raw());
    }
    let pixels = pixels_of(&gpu, &held[0], SIZE);
    assert_eq!(pixels.len(), (SIZE.0 * SIZE.1 * 4) as usize);
    assert!(pixels.chunks(4).all(|pixel| pixel == RED_PIXEL));
    assert_ring_matches(&copier, new_size);
}

#[test]
fn a_failed_texture_creation_returns_the_error_and_publishes_nothing() {
    let _armed = Armed::new().fail(Point::FrameCopyTexture, E_OUTOFMEMORY);
    let gpu = crate::testing::warp::device().unwrap();
    let source = plain_texture(&gpu, SIZE);
    let mut copier = FrameCopier::new(&gpu).unwrap();
    let error = copier.copy(&source, SIZE, 0).unwrap_err();
    assert_eq!(error.code(), E_OUTOFMEMORY);
    assert!(copier.texture(0).is_none());
}

fn plain_texture(
    gpu: &crate::d3d::GpuDevice,
    size: (u32, u32),
) -> windows::Win32::Graphics::Direct3D11::ID3D11Texture2D {
    let mut texture = None;
    // SAFETY: the device is live; the descriptor and out pointer outlive the call.
    unsafe {
        gpu.device
            .CreateTexture2D(&frame_desc(size), None, Some(&mut texture))
            .unwrap()
    };
    texture.unwrap()
}

#[test]
fn a_failed_resize_leaves_no_stale_textures_and_the_next_copy_recovers() {
    let gpu = {
        let _armed = Armed::new();
        crate::testing::warp::device().unwrap()
    };
    let wider = (SIZE.0 + 1, SIZE.1);
    let source = plain_texture(&gpu, wider);
    let mut copier = FrameCopier::new(&gpu).unwrap();
    {
        let _armed = Armed::new();
        copier.copy(&plain_texture(&gpu, SIZE), SIZE, 0).unwrap();
    }
    {
        let _armed = Armed::new().fail(Point::FrameCopyTexture, E_FAIL);
        assert!(copier.copy(&source, wider, 0).is_err());
        assert!(copier.texture(0).is_none());
    }
    let _armed = Armed::new();
    copier.copy(&source, wider, 0).unwrap();
    assert_ring_matches(&copier, wider);
}

#[test]
fn a_thumbnail_texture_failure_turns_thumbnails_off_for_good_and_the_copy_still_works() {
    let gpu = {
        let _armed = Armed::new();
        crate::testing::warp::device().unwrap()
    };
    let source = plain_texture(&gpu, SIZE);
    let mut copier = FrameCopier::new(&gpu).unwrap();
    {
        let _armed = Armed::new().fail_after(Point::FrameCopyTexture, RING_SLOTS, E_OUTOFMEMORY);
        let copied = copier.copy(&source, SIZE, 0).unwrap();
        assert!(!copied.thumbed);
        assert!(copier.thumbs_off && copier.thumb.is_none());
    }
    let _armed = Armed::new();
    let wider = (SIZE.0 + 2, SIZE.1);
    let source = plain_texture(&gpu, wider);
    let copied = copier.copy(&source, wider, 0).unwrap();
    assert!(!copied.thumbed);
    assert!(copier.thumb.is_none());
}

#[test]
fn repeated_thumbnail_reads_leave_the_staging_texture_mappable() {
    let _armed = Armed::new();
    let gpu = crate::testing::warp::device().unwrap();
    let source = plain_texture(&gpu, SIZE);
    let mut copier = FrameCopier::new(&gpu).unwrap();
    let mut luma = Vec::new();
    for _ in 0..5 {
        let copied = copier.copy(&source, SIZE, 0).unwrap();
        assert!(copied.thumbed);
        let width = copier.read_thumb(&mut luma).unwrap();
        assert_eq!(luma.len() % width, 0);
    }
}

#[test]
fn a_failed_thumbnail_map_returns_an_error_and_the_next_read_works() {
    let gpu = {
        let _armed = Armed::new();
        crate::testing::warp::device().unwrap()
    };
    let source = plain_texture(&gpu, SIZE);
    let mut copier = FrameCopier::new(&gpu).unwrap();
    let mut luma = Vec::new();
    {
        let _armed = Armed::new();
        copier.copy(&source, SIZE, 0).unwrap();
    }
    {
        let _armed = Armed::new().fail(Point::StagingMap, E_FAIL);
        assert_eq!(copier.read_thumb(&mut luma).unwrap_err().code(), E_FAIL);
    }
    let _armed = Armed::new();
    assert!(copier.read_thumb(&mut luma).is_ok());
}

#[test]
fn a_stalled_copy_wait_still_reports_that_a_thumbnail_was_queued() {
    let _armed = Armed::new().stall(Point::GpuCopyWait, Duration::from_millis(1));
    let gpu = crate::testing::warp::device().unwrap();
    let source = plain_texture(&gpu, SIZE);
    let mut copier = FrameCopier::new(&gpu).unwrap();
    let copied = copier.copy(&source, SIZE, 0).unwrap();
    assert!(matches!(copied.wait, Wait::TimedOut(_)));
    assert!(copied.thumbed);
}

#[test]
fn luma_rows_skips_the_padding_when_the_row_pitch_is_larger_than_the_width() {
    let (width, height, pitch) = (3usize, 4usize, 3 * 4 + 20);
    let mut bytes = vec![0xEE; (height - 1) * pitch + width * 4];
    for y in 0..height {
        for x in 0..width {
            let at = y * pitch + x * 4;
            let level = (y * 40 + x * 10) as u8;
            bytes[at..at + 4].copy_from_slice(&[level, level, level, 255]);
        }
    }
    let mut out = vec![9; 1];
    luma_rows(&bytes, pitch, (width, height), &mut out);
    assert_eq!(out.len(), width * height);
    for y in 0..height {
        for x in 0..width {
            let level = (y * 40 + x * 10) as u8;
            let want = greyscreen_core::tear::luma(level, level, level);
            assert_eq!(out[y * width + x], want, "row {y} column {x}");
        }
    }
}
