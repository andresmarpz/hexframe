use crate::audio::{CHANNELS, RATE};

pub(crate) fn sine(frames: usize, first_frame: u64, hz: f32) -> Vec<f32> {
    let mut samples = Vec::with_capacity(frames * CHANNELS);
    for index in 0..frames {
        let t = (first_frame + index as u64) as f32 / RATE as f32;
        let value = (std::f32::consts::TAU * hz * t).sin() * 0.5;
        samples.extend(std::iter::repeat_n(value, CHANNELS));
    }
    samples
}

pub(crate) fn click(frames: usize, at: usize) -> Vec<f32> {
    let mut samples = vec![0.0; frames * CHANNELS];
    if at < frames {
        samples[at * CHANNELS..(at + 1) * CHANNELS].fill(1.0);
    }
    samples
}
