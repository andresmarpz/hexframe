use greyscreen_core::colour::{self, BLACK, Ycbcr};
use greyscreen_core::crop::CropRect;
use windows::Win32::Foundation::E_POINTER;
use windows::Win32::Graphics::Direct3D11::{
    D3D11_BIND_RENDER_TARGET, D3D11_BIND_SHADER_RESOURCE, D3D11_CPU_ACCESS_READ, D3D11_MAP_READ,
    D3D11_MAPPED_SUBRESOURCE, D3D11_SUBRESOURCE_DATA, D3D11_TEXTURE2D_DESC, D3D11_USAGE_DEFAULT,
    D3D11_USAGE_STAGING, ID3D11Texture2D,
};
use windows::Win32::Graphics::Dxgi::Common::{
    DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_FORMAT_NV12, DXGI_SAMPLE_DESC,
};

use super::{Nv12Frame, VideoConverter};
use crate::d3d::GpuDevice;
use crate::testing::warp;

const TOLERANCE: i16 = 2;
const FPS: u32 = 30;

struct Plane {
    luma: Vec<u8>,
    chroma: Vec<u8>,
    width: usize,
}

impl Plane {
    fn luma_at(&self, x: usize, y: usize) -> u8 {
        self.luma[y * self.width + x]
    }

    fn at(&self, x: usize, y: usize) -> Ycbcr {
        let chroma = (y / 2) * self.width + (x / 2) * 2;
        Ycbcr {
            y: self.luma_at(x, y),
            cb: self.chroma[chroma],
            cr: self.chroma[chroma + 1],
        }
    }
}

fn close(a: u8, b: u8) -> bool {
    (i16::from(a) - i16::from(b)).abs() <= TOLERANCE
}

fn assert_near(actual: Ycbcr, expected: Ycbcr, what: &str) {
    let near = close(actual.y, expected.y)
        && close(actual.cb, expected.cb)
        && close(actual.cr, expected.cr);
    assert!(near, "{what}: got {actual:?}, want {expected:?}");
}

fn bgra(red: u8, green: u8, blue: u8, alpha: u8) -> [u8; 4] {
    [blue, green, red, alpha]
}

fn filled(width: u32, height: u32, pixel: [u8; 4]) -> Vec<u8> {
    pixel.repeat((width * height) as usize)
}

fn upload(gpu: &GpuDevice, size: (u32, u32), pixels: &[u8]) -> ID3D11Texture2D {
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
        SysMemPitch: size.0 * 4,
        SysMemSlicePitch: 0,
    };
    let mut texture = None;
    // SAFETY: the device is live; `desc`, `data` and the pixels outlive the call.
    unsafe {
        gpu.device
            .CreateTexture2D(&desc, Some(&data), Some(&mut texture))
            .unwrap()
    };
    texture.ok_or(E_POINTER).unwrap()
}

fn read(gpu: &GpuDevice, frame: &Nv12Frame, size: (u32, u32)) -> Plane {
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
        Usage: D3D11_USAGE_STAGING,
        BindFlags: 0,
        CPUAccessFlags: D3D11_CPU_ACCESS_READ.0 as u32,
        MiscFlags: 0,
    };
    let mut staging = None;
    // SAFETY: the device is live; `desc` and `staging` outlive the call.
    unsafe {
        gpu.device
            .CreateTexture2D(&desc, None, Some(&mut staging))
            .unwrap()
    };
    let staging = staging.ok_or(E_POINTER).unwrap();
    let (width, height) = (size.0 as usize, size.1 as usize);
    let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
    // SAFETY: both textures are live NV12 textures of the same size; `mapped` outlives the call.
    unsafe {
        gpu.context.CopyResource(&staging, &frame.texture);
        gpu.context
            .Map(&staging, 0, D3D11_MAP_READ, 0, Some(&mut mapped))
            .unwrap();
    }
    let pitch = mapped.RowPitch as usize;
    let rows = |first: usize, count: usize| {
        let mut out = Vec::with_capacity(count * width);
        for row in first..first + count {
            // SAFETY: an NV12 mapping holds `height * 3 / 2` rows of `pitch` bytes, chroma right after luma.
            let line = unsafe {
                std::slice::from_raw_parts(mapped.pData.cast::<u8>().add(row * pitch), width)
            };
            out.extend_from_slice(line);
        }
        out
    };
    let plane = Plane {
        luma: rows(0, height),
        chroma: rows(height, height / 2),
        width,
    };
    // SAFETY: balances the successful Map above.
    unsafe { gpu.context.Unmap(&staging, 0) };
    plane
}

fn convert(
    converter: &mut VideoConverter,
    gpu: &GpuDevice,
    src: &ID3D11Texture2D,
    src_size: (u32, u32),
    out: (u32, u32),
) -> Plane {
    let frame = converter
        .convert(src, src_size, CropRect::full(src_size))
        .unwrap();
    let plane = read(gpu, &frame, out);
    converter.release(frame.slot);
    plane
}

fn solid_through_converter(red: u8, green: u8, blue: u8, alpha: u8) -> Plane {
    let gpu = warp::device().unwrap();
    let mut converter = VideoConverter::new(&gpu, 64, 64, FPS).unwrap();
    let pixels = filled(64, 64, bgra(red, green, blue, alpha));
    let src = upload(&gpu, (64, 64), &pixels);
    convert(&mut converter, &gpu, &src, (64, 64), (64, 64))
}

#[test]
fn solid_patches_convert_to_the_bt709_studio_codes() {
    for (red, green, blue) in [
        (0, 0, 0),
        (255, 255, 255),
        (128, 128, 128),
        (255, 0, 0),
        (0, 0, 255),
    ] {
        let plane = solid_through_converter(red, green, blue, 255);
        assert_near(
            plane.at(32, 32),
            colour::bt709_studio(red, green, blue),
            &format!("patch {red},{green},{blue}"),
        );
    }
}

#[test]
fn black_lands_on_luma_16_and_white_on_luma_235() {
    assert_eq!(solid_through_converter(0, 0, 0, 255).luma_at(32, 32), 16);
    assert_eq!(
        solid_through_converter(255, 255, 255, 255).luma_at(32, 32),
        235
    );
}

#[test]
fn a_hard_red_blue_edge_keeps_chroma_on_the_left_sited_column() {
    let gpu = warp::device().unwrap();
    let mut converter = VideoConverter::new(&gpu, 64, 64, FPS).unwrap();
    let mut pixels = filled(64, 64, bgra(255, 0, 0, 255));
    for row in 0..64 {
        for column in 32..64 {
            let at = (row * 64 + column) * 4;
            pixels[at..at + 4].copy_from_slice(&bgra(0, 0, 255, 255));
        }
    }
    let src = upload(&gpu, (64, 64), &pixels);
    let plane = convert(&mut converter, &gpu, &src, (64, 64), (64, 64));
    let red = colour::bt709_studio(255, 0, 0);
    let blue = colour::bt709_studio(0, 0, 255);
    assert_near(plane.at(30, 32), red, "last red chroma column");
    assert_near(plane.at(32, 32), blue, "first blue chroma column");
    assert!(close(plane.luma_at(31, 32), red.y), "red luma at the edge");
    assert!(
        close(plane.luma_at(32, 32), blue.y),
        "blue luma at the edge"
    );
}

#[test]
fn downscaling_keeps_the_same_grey() {
    let gpu = warp::device().unwrap();
    let mut converter = VideoConverter::new(&gpu, 1280, 720, FPS).unwrap();
    let pixels = filled(2560, 1440, bgra(128, 128, 128, 255));
    let src = upload(&gpu, (2560, 1440), &pixels);
    let plane = convert(&mut converter, &gpu, &src, (2560, 1440), (1280, 720));
    assert_near(
        plane.at(640, 360),
        colour::bt709_studio(128, 128, 128),
        "scaled grey",
    );
}

#[test]
fn a_half_transparent_edge_equals_the_colour_over_black() {
    let (red, green, blue) = (200, 120, 40);
    let weighed = |channel: u8| ((u16::from(channel) * 128 + 127) / 255) as u8;
    let plane = solid_through_converter(weighed(red), weighed(green), weighed(blue), 128);
    assert_near(
        plane.at(32, 32),
        colour::bt709_studio_over_black(red, green, blue, 128),
        "half-transparent pixel",
    );
}

#[test]
fn zero_alpha_with_a_known_colour_converts_to_that_colour() {
    let plane = solid_through_converter(200, 100, 50, 0);
    assert_near(
        plane.at(32, 32),
        colour::bt709_studio(200, 100, 50),
        "alpha 0",
    );
    assert!(plane.luma_at(32, 32) > 16);
}

#[test]
fn a_source_of_a_new_size_rebuilds_the_converter_and_letterboxes() {
    let gpu = warp::device().unwrap();
    let mut converter = VideoConverter::new(&gpu, 64, 64, FPS).unwrap();
    let first = upload(&gpu, (64, 64), &filled(64, 64, bgra(255, 255, 255, 255)));
    convert(&mut converter, &gpu, &first, (64, 64), (64, 64));
    assert_eq!(converter.input_views.len(), 1);
    let wide = upload(&gpu, (128, 64), &filled(128, 64, bgra(255, 255, 255, 255)));
    let plane = convert(&mut converter, &gpu, &wide, (128, 64), (64, 64));
    assert_eq!(converter.input_size, (128, 64));
    assert_eq!(converter.input_views.len(), 1);
    assert_near(
        plane.at(32, 32),
        colour::bt709_studio(255, 255, 255),
        "picture",
    );
    assert_near(plane.at(32, 2), BLACK, "top border");
    assert_near(plane.at(32, 61), BLACK, "bottom border");
}

#[test]
fn a_small_source_in_a_slot_that_held_a_bright_frame_leaves_black_borders() {
    let gpu = warp::device().unwrap();
    let mut converter = VideoConverter::new(&gpu, 64, 64, FPS).unwrap();
    let bright = upload(&gpu, (64, 64), &filled(64, 64, bgra(255, 255, 255, 255)));
    let first = converter
        .convert(&bright, (64, 64), CropRect::full((64, 64)))
        .unwrap();
    let slot = first.slot;
    converter.release(slot);
    let small = upload(&gpu, (32, 32), &filled(32, 32, bgra(255, 255, 255, 255)));
    let frame = converter
        .convert(&small, (32, 32), CropRect::full((32, 32)))
        .unwrap();
    assert_eq!(frame.slot, slot);
    let plane = read(&gpu, &frame, (64, 64));
    assert_eq!(plane.luma_at(2, 2), 16);
    assert_eq!(plane.luma_at(61, 61), 16);
    assert_near(
        plane.at(32, 32),
        colour::bt709_studio(255, 255, 255),
        "picture",
    );
}

#[test]
fn recreating_the_copy_ring_at_the_same_size_reads_the_new_texture() {
    let gpu = warp::device().unwrap();
    let mut converter = VideoConverter::new(&gpu, 64, 64, FPS).unwrap();
    for round in 0..8 {
        let (red, green, blue) = if round % 2 == 0 {
            (255, 0, 0)
        } else {
            (0, 0, 255)
        };
        let src = upload(&gpu, (64, 64), &filled(64, 64, bgra(red, green, blue, 255)));
        let plane = convert(&mut converter, &gpu, &src, (64, 64), (64, 64));
        assert_near(
            plane.at(32, 32),
            colour::bt709_studio(red, green, blue),
            &format!("round {round}"),
        );
        drop(src);
    }
}
