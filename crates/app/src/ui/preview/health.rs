use greyscreen_core::health::{AudioHealth, EncodeTiming, RecordingHealth};

pub(in crate::ui) fn healthy() -> RecordingHealth {
    RecordingHealth {
        target_fps: 60,
        width: 1920,
        height: 1080,
        duration_secs: 1_812.4,
        frames_expected: 108_744,
        frames_captured: 108_700,
        frames_encoded: 108_702,
        frames_dropped: 42,
        frames_repeated: 310,
        dropped_pct: 0.04,
        repeated_pct: 0.29,
        source_fps: 60.0,
        longest_video_gap_ms: 50.0,
        encoder: "NVIDIA H.264 Encoder MFT (quality 70)".into(),
        encode_ms: Some(EncodeTiming {
            avg: 3.1,
            p95: 6.5,
            max: 11.0,
        }),
        audio: Some(AudioHealth {
            packets: 84_900,
            start_offset_ms: -6.2,
            drift_ms: 4.0,
            ..AudioHealth::default()
        }),
        file_bytes: 2_050_000_000,
        avg_bitrate_kbps: 9_050,
        ..RecordingHealth::default()
    }
}

pub(in crate::ui) fn struggling() -> RecordingHealth {
    RecordingHealth {
        frames_encoded: 106_590,
        frames_dropped: 2_154,
        frames_expected: 108_744,
        dropped_pct: 1.98,
        repeated_pct: 6.4,
        source_fps: 57.0,
        longest_video_gap_ms: 650.0,
        encode_ms: Some(EncodeTiming {
            avg: 9.0,
            p95: 21.0,
            max: 140.0,
        }),
        audio: Some(AudioHealth {
            packets: 84_900,
            gaps: 3,
            longest_gap_ms: 120.0,
            drift_ms: -310.0,
            ..AudioHealth::default()
        }),
        capture_soft_errors: 12,
        ..healthy()
    }
}
