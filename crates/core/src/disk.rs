use crate::media::CaptureConfig;

pub const BYTES_PER_MB: u64 = 1_000_000;
pub const BYTES_PER_GB: u64 = 1_000 * BYTES_PER_MB;
pub const START_FLOOR_BYTES: u64 = 2 * BYTES_PER_GB;
pub const START_MINUTES: u64 = 10;
pub const STOP_BYTES: u64 = 500 * BYTES_PER_MB;
pub const CHECK_EVERY_SECS: u64 = 10;

const _: () = assert!(STOP_BYTES < START_FLOOR_BYTES);

pub fn bytes_per_sec(config: &CaptureConfig) -> u64 {
    let audio = if config.audio {
        u64::from(config.audio_bitrate)
    } else {
        0
    };
    (u64::from(config.video_bitrate) + audio) / 8
}

pub fn start_threshold(bytes_per_sec: u64) -> u64 {
    START_FLOOR_BYTES.max(bytes_per_sec.saturating_mul(START_MINUTES * 60))
}

pub fn can_start(free_bytes: u64, bytes_per_sec: u64) -> bool {
    free_bytes >= start_threshold(bytes_per_sec)
}

pub fn must_stop(free_bytes: u64) -> bool {
    free_bytes < STOP_BYTES
}

pub fn gb_text(bytes: u64) -> String {
    format!("{:.1} GB", bytes as f64 / BYTES_PER_GB as f64)
}

#[cfg(test)]
mod tests;
