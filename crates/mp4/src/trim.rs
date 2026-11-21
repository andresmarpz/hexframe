use std::path::Path;
use std::sync::Arc;

use greyscreen_core::media::{AudioFormat, Packet, Track, VideoFormat};

use crate::clip::write_clip;
use crate::demux::{Movie, SampleRef};
use crate::{Mp4Error, Result};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Trimmed {
    pub start_secs: f64,
    pub end_secs: f64,
}

impl Trimmed {
    pub fn duration_secs(&self) -> f64 {
        self.end_secs - self.start_secs
    }
}

pub fn trim(src: &Path, dst: &Path, start_secs: f64, end_secs: f64) -> Result<Trimmed> {
    let movie = Movie::open(src)?;
    let (first, last) = video_range(&movie, start_secs, end_secs)?;
    let video = &movie.video_samples[first..last];
    let start_ticks = video[0].dts;
    let end_ticks = video[video.len() - 1].end();

    let audio = movie.audio.as_ref();
    let audio_samples = audio
        .map(|format| audio_range(&movie.audio_samples, format, start_ticks, end_ticks))
        .unwrap_or_default();

    let mut reader = movie.reader()?;
    let mut packets: Vec<Arc<Packet>> = Vec::with_capacity(video.len() + audio_samples.len());
    let mut audio_iter = audio_samples.iter().peekable();
    let rate = audio.map_or(1, |a| a.sample_rate);
    for sample in video {
        while let Some(next) = audio_iter.peek() {
            if audio_first(next, rate, sample.dts) {
                let next = audio_iter.next();
                if let Some(next) = next {
                    packets.push(packet(&mut reader, Track::Audio, next)?);
                }
            } else {
                break;
            }
        }
        packets.push(packet(&mut reader, Track::Video, sample)?);
    }
    for rest in audio_iter {
        packets.push(packet(&mut reader, Track::Audio, rest)?);
    }

    write_clip(dst, &movie.video, movie.audio.as_ref(), &packets)?;
    Ok(Trimmed {
        start_secs: start_ticks as f64 / f64::from(VideoFormat::TIMESCALE),
        end_secs: end_ticks as f64 / f64::from(VideoFormat::TIMESCALE),
    })
}

fn video_range(movie: &Movie, start_secs: f64, end_secs: f64) -> Result<(usize, usize)> {
    let samples = &movie.video_samples;
    if samples.is_empty() {
        return Err(Mp4Error::Invalid("source has no video".into()));
    }
    let ticks = |secs: f64| (secs.max(0.0) * f64::from(VideoFormat::TIMESCALE)).round() as i64;
    let (start, end) = (ticks(start_secs), ticks(end_secs));
    let at_or_before = samples.partition_point(|s| s.dts <= start);
    let first = samples[..at_or_before.max(1)]
        .iter()
        .rposition(|s| s.key)
        .ok_or_else(|| Mp4Error::Invalid("source has no key frame before the start".into()))?;
    let last = samples.partition_point(|s| s.dts < end);
    if last <= first {
        return Err(Mp4Error::Invalid("the range holds no video".into()));
    }
    Ok((first, last))
}

fn audio_range(
    samples: &[SampleRef],
    format: &AudioFormat,
    start_ticks: i64,
    end_ticks: i64,
) -> Vec<SampleRef> {
    let rate = i128::from(format.sample_rate);
    let scale = i128::from(VideoFormat::TIMESCALE);
    samples
        .iter()
        .filter(|s| {
            let ends_after_start = i128::from(s.end()) * scale > i128::from(start_ticks) * rate;
            let starts_before_end = i128::from(s.dts) * scale < i128::from(end_ticks) * rate;
            ends_after_start && starts_before_end
        })
        .copied()
        .collect()
}

fn audio_first(audio: &SampleRef, rate: u32, video_dts: i64) -> bool {
    i128::from(audio.dts) * i128::from(VideoFormat::TIMESCALE)
        <= i128::from(video_dts) * i128::from(rate)
}

fn packet(
    reader: &mut crate::demux::SampleReader,
    track: Track,
    sample: &SampleRef,
) -> Result<Arc<Packet>> {
    let data = reader.read(sample)?;
    Ok(Arc::new(Packet {
        track,
        dts: sample.dts,
        pts: sample.dts,
        duration: sample.duration,
        key: sample.key,
        data: Arc::from(data),
    }))
}
