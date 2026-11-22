use crate::d3d::GpuDevice;
use windows::Win32::Foundation::E_POINTER;
use windows::Win32::Graphics::Direct3D11::{
    D3D11_CPU_ACCESS_READ, D3D11_MAP_READ, D3D11_MAPPED_SUBRESOURCE, D3D11_TEXTURE2D_DESC,
    D3D11_USAGE_STAGING, ID3D11Texture2D,
};

pub const COUNTER_BITS: usize = 16;
pub(super) const BAND_FROM_PERCENT: i32 = 45;
pub(super) const BAND_TO_PERCENT: i32 = 55;
pub(super) const LEVEL_TOLERANCE: u8 = 40;

pub(super) const RED: [u8; 3] = [255, 0, 0];
pub(super) const GREEN: [u8; 3] = [0, 255, 0];
pub(super) const BLUE: [u8; 3] = [0, 0, 255];
pub(super) const YELLOW: [u8; 3] = [255, 255, 0];
pub(super) const WHITE: [u8; 3] = [255, 255, 255];
pub(super) const BLACK: [u8; 3] = [0, 0, 0];
pub(super) const QUADRANTS: [(&str, [u8; 3]); 4] = [
    ("top left", RED),
    ("top right", GREEN),
    ("bottom left", BLUE),
    ("bottom right", YELLOW),
];

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum PatternError {
    #[error("the frame is {0}x{1}, too small to hold the pattern")]
    TooSmall(usize, usize),
    #[error("the {0} quadrant is not its colour")]
    Quadrant(&'static str),
    #[error("counter bit {0} is neither black nor white")]
    Bit(usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    pub width: usize,
    pub height: usize,
    pub bgra: Vec<u8>,
}

#[derive(Debug)]
pub struct FrameReader {
    gpu: GpuDevice,
    staging: Option<(ID3D11Texture2D, u32, u32)>,
}

impl FrameReader {
    pub fn new(gpu: &GpuDevice) -> Self {
        Self {
            gpu: gpu.clone(),
            staging: None,
        }
    }

    pub fn read(
        &mut self,
        texture: &ID3D11Texture2D,
        size: (u32, u32),
    ) -> windows::core::Result<Frame> {
        let mut desc = D3D11_TEXTURE2D_DESC::default();
        // SAFETY: `desc` is a live out value and `texture` is a live texture.
        unsafe { texture.GetDesc(&mut desc) };
        let reusable = self
            .staging
            .as_ref()
            .is_some_and(|(_, width, height)| (*width, *height) == (desc.Width, desc.Height));
        if !reusable {
            let staging = new_staging(&self.gpu, desc)?;
            self.staging = Some((staging, desc.Width, desc.Height));
        }
        let Some((staging, _, _)) = &self.staging else {
            return Err(windows::core::Error::from(E_POINTER));
        };
        copy_out(&self.gpu, staging, texture, desc, size)
    }
}

fn new_staging(
    gpu: &GpuDevice,
    mut desc: D3D11_TEXTURE2D_DESC,
) -> windows::core::Result<ID3D11Texture2D> {
    desc.Usage = D3D11_USAGE_STAGING;
    desc.BindFlags = 0;
    desc.MiscFlags = 0;
    desc.CPUAccessFlags = D3D11_CPU_ACCESS_READ.0 as u32;
    let mut staging = None;
    // SAFETY: the device is live; `desc` and `staging` outlive the call.
    unsafe {
        gpu.device
            .CreateTexture2D(&desc, None, Some(&mut staging))?
    };
    staging.ok_or_else(|| windows::core::Error::from(E_POINTER))
}

fn copy_out(
    gpu: &GpuDevice,
    staging: &ID3D11Texture2D,
    texture: &ID3D11Texture2D,
    desc: D3D11_TEXTURE2D_DESC,
    size: (u32, u32),
) -> windows::core::Result<Frame> {
    // SAFETY: both textures are live, of the same size and format, on this device.
    unsafe { gpu.context.CopyResource(staging, texture) };
    let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
    // SAFETY: the staging texture is live and `mapped` is a live out value.
    unsafe {
        gpu.context
            .Map(staging, 0, D3D11_MAP_READ, 0, Some(&mut mapped))?
    };
    let width = (size.0.min(desc.Width)) as usize;
    let height = (size.1.min(desc.Height)) as usize;
    let stride = mapped.RowPitch as usize;
    let mut bgra = Vec::with_capacity(width * height * 4);
    for row in 0..height {
        // SAFETY: the mapping holds `desc.Height` rows of `RowPitch` bytes, and each row has at least `width * 4`.
        let line = unsafe {
            std::slice::from_raw_parts(mapped.pData.cast::<u8>().add(row * stride), width * 4)
        };
        bgra.extend_from_slice(line);
    }
    // SAFETY: balances the successful Map above.
    unsafe { gpu.context.Unmap(staging, 0) };
    Ok(Frame {
        width,
        height,
        bgra,
    })
}

pub fn read_counter(frame: &Frame) -> Result<u16, PatternError> {
    let (width, height) = (frame.width, frame.height);
    if width < COUNTER_BITS * 2 || height < 8 {
        return Err(PatternError::TooSmall(width, height));
    }
    let quadrant_points = [
        (width / 4, height / 4),
        (width * 3 / 4, height / 4),
        (width / 4, height * 3 / 4),
        (width * 3 / 4, height * 3 / 4),
    ];
    for ((name, expected), (x, y)) in QUADRANTS.iter().zip(quadrant_points) {
        if !near(pixel(frame, x, y), *expected) {
            return Err(PatternError::Quadrant(name));
        }
    }
    let mut counter = 0u16;
    for bit in 0..COUNTER_BITS {
        let seen = pixel(
            frame,
            (bit * 2 + 1) * width / (COUNTER_BITS * 2),
            height / 2,
        );
        let value = if near(seen, WHITE) {
            1
        } else if near(seen, BLACK) {
            0
        } else {
            return Err(PatternError::Bit(bit));
        };
        counter = (counter << 1) | value;
    }
    Ok(counter)
}

fn pixel(frame: &Frame, x: usize, y: usize) -> [u8; 3] {
    let at = (y * frame.width + x) * 4;
    [frame.bgra[at + 2], frame.bgra[at + 1], frame.bgra[at]]
}

fn near(seen: [u8; 3], expected: [u8; 3]) -> bool {
    seen.iter()
        .zip(expected)
        .all(|(a, b)| a.abs_diff(b) <= LEVEL_TOLERANCE)
}
