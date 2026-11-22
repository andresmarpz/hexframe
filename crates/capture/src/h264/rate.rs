use windows::Win32::Media::MediaFoundation::{
    CODECAPI_AVEncCommonBufferSize, CODECAPI_AVEncCommonMaxBitRate,
    CODECAPI_AVEncCommonMeanBitRate, CODECAPI_AVEncCommonQuality,
    CODECAPI_AVEncCommonRateControlMode, eAVEncCommonRateControlMode_PeakConstrainedVBR,
    eAVEncCommonRateControlMode_Quality,
};
use windows::Win32::System::Variant::VARIANT;

use super::Result;
use super::setup::{Property, Spec, apply_property};
use super::transform::Mft;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum RateMode {
    Quality(u32),
    Vbr { mean_bitrate: u32 },
}

impl RateMode {
    pub(super) fn label(self) -> String {
        match self {
            RateMode::Quality(quality) => format!("quality {quality}"),
            RateMode::Vbr { mean_bitrate } => {
                format!("vbr {:.1} Mbps", f64::from(mean_bitrate) / 1e6)
            }
        }
    }
}

pub(super) fn apply(codec: &dyn Mft, spec: &Spec) -> Result<RateMode> {
    if let Some(quality) = spec.quality {
        match try_quality(codec, quality) {
            Ok(()) => {
                apply_caps(codec, spec)?;
                return Ok(RateMode::Quality(quality));
            }
            Err(reason) => {
                tracing::warn!(%reason, "quality rate control unavailable, using peak-constrained VBR");
            }
        }
    }
    apply_vbr(codec, spec)?;
    Ok(RateMode::Vbr {
        mean_bitrate: spec.mean_bitrate,
    })
}

fn try_quality(codec: &dyn Mft, quality: u32) -> std::result::Result<(), String> {
    let mode = eAVEncCommonRateControlMode_Quality.0 as u32;
    let mode_property = Property {
        name: "AVEncCommonRateControlMode",
        id: CODECAPI_AVEncCommonRateControlMode,
        value: VARIANT::from(mode),
        required: false,
    };
    let applied = apply_property(codec, &mode_property).map_err(|e| e.to_string())?;
    if !applied.set {
        return Err("the encoder rejected Quality rate control mode".to_string());
    }
    if applied.read_back != Some(mode) {
        return Err(format!(
            "rate control mode read back as {:?} instead of {mode}",
            applied.read_back
        ));
    }
    let quality_property = Property {
        name: "AVEncCommonQuality",
        id: CODECAPI_AVEncCommonQuality,
        value: VARIANT::from(quality),
        required: false,
    };
    let applied = apply_property(codec, &quality_property).map_err(|e| e.to_string())?;
    if !applied.set {
        return Err("the encoder rejected the quality value".to_string());
    }
    if applied.read_back != Some(quality) {
        return Err(format!(
            "quality read back as {:?} instead of {quality}",
            applied.read_back
        ));
    }
    Ok(())
}

fn apply_caps(codec: &dyn Mft, spec: &Spec) -> Result<()> {
    for property in caps(spec) {
        apply_property(codec, &property)?;
    }
    Ok(())
}

fn apply_vbr(codec: &dyn Mft, spec: &Spec) -> Result<()> {
    let mode = eAVEncCommonRateControlMode_PeakConstrainedVBR.0 as u32;
    let leading = [
        Property {
            name: "AVEncCommonRateControlMode",
            id: CODECAPI_AVEncCommonRateControlMode,
            value: VARIANT::from(mode),
            required: true,
        },
        Property {
            name: "AVEncCommonMeanBitRate",
            id: CODECAPI_AVEncCommonMeanBitRate,
            value: VARIANT::from(spec.mean_bitrate),
            required: true,
        },
    ];
    for property in leading {
        apply_property(codec, &property)?;
    }
    apply_caps(codec, spec)
}

fn caps(spec: &Spec) -> [Property; 2] {
    [
        Property {
            name: "AVEncCommonMaxBitRate",
            id: CODECAPI_AVEncCommonMaxBitRate,
            value: VARIANT::from(spec.peak_bitrate),
            required: false,
        },
        Property {
            name: "AVEncCommonBufferSize",
            id: CODECAPI_AVEncCommonBufferSize,
            value: VARIANT::from(spec.peak_bitrate / 8),
            required: false,
        },
    ]
}
