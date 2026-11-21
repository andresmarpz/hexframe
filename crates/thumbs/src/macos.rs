use std::path::Path;

use objc2_av_foundation::{AVAssetImageGenerator, AVURLAsset};
use objc2_core_foundation::{CGPoint, CGRect, CGSize};
use objc2_core_graphics::{
    CGBitmapContextCreate, CGColorSpace, CGContext, CGImage, CGImageAlphaInfo, kCGColorSpaceSRGB,
};
use objc2_core_media::{CMTime, CMTimeFlags};
use objc2_foundation::{NSString, NSURL};

use crate::{Frame, ThumbError};

const TIMESCALE: i32 = 600_000;

fn zero() -> CMTime {
    CMTime {
        value: 0,
        timescale: 1,
        flags: CMTimeFlags::Valid,
        epoch: 0,
    }
}

pub(super) fn grab(video: &Path, at_secs: f64, max_width: u32) -> Result<Frame, ThumbError> {
    if !video.is_file() {
        return Err(ThumbError::Decode(format!(
            "no such file: {}",
            video.display()
        )));
    }
    let path = std::path::absolute(video).unwrap_or_else(|_| video.to_path_buf());
    let url = NSURL::fileURLWithPath(&NSString::from_str(&path.to_string_lossy()));
    // SAFETY: plain Objective-C object construction and property writes on objects this thread owns.
    let generator = unsafe {
        let asset = AVURLAsset::URLAssetWithURL_options(&url, None);
        let generator = AVAssetImageGenerator::assetImageGeneratorWithAsset(&asset);
        generator.setAppliesPreferredTrackTransform(true);
        generator.setRequestedTimeToleranceBefore(zero());
        generator.setRequestedTimeToleranceAfter(zero());
        let edge = f64::from(max_width);
        generator.setMaximumSize(CGSize::new(edge, edge));
        generator
    };
    // SAFETY: a blocking decode; the actual-time out pointer may be null.
    #[allow(deprecated)]
    let image = unsafe {
        let at = CMTime::with_seconds(at_secs, TIMESCALE);
        generator.copyCGImageAtTime_actualTime_error(at, std::ptr::null_mut())
    }
    .map_err(|err| ThumbError::Decode(err.localizedDescription().to_string()))?;
    rgba(&image)
}

fn rgba(image: &CGImage) -> Result<Frame, ThumbError> {
    let (width, height) = (CGImage::width(Some(image)), CGImage::height(Some(image)));
    if width == 0 || height == 0 {
        return Err(ThumbError::Decode("decoded an empty frame".into()));
    }
    let mut bytes = vec![0u8; width * height * 4];
    // SAFETY: a constant CoreGraphics colour space name.
    let space = CGColorSpace::with_name(Some(unsafe { kCGColorSpaceSRGB }));
    let info = CGImageAlphaInfo::PremultipliedLast.0;
    // SAFETY: the context draws only inside `bytes`, which holds width * height * 4 bytes and outlives it.
    let context = unsafe {
        CGBitmapContextCreate(
            bytes.as_mut_ptr().cast(),
            width,
            height,
            8,
            width * 4,
            space.as_deref(),
            info,
        )
    }
    .ok_or_else(|| ThumbError::Decode("cannot create a bitmap context".into()))?;
    let area = CGRect::new(
        CGPoint::new(0.0, 0.0),
        CGSize::new(width as f64, height as f64),
    );
    CGContext::draw_image(Some(&context), area, Some(image));
    drop(context);
    Ok(Frame {
        width: width as u32,
        height: height as u32,
        rgba: bytes,
    })
}
