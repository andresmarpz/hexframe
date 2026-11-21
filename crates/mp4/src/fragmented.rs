use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::sync::Arc;

use greyscreen_core::media::{AudioFormat, Packet, Track, VideoFormat};

use crate::bytes::BoxWriter;
use crate::moov::{
    AUDIO_TRACK_ID, Chunk, Chunking, NON_SYNC_SAMPLE_FLAGS, SYNC_SAMPLE_FLAGS, SampleRow,
    SampleTables, VIDEO_TRACK_ID, write_ftyp, write_init_moov,
};
use crate::{Mp4Error, Result};

pub use crate::finalize::FinishedInfo;
use crate::finalize::finalize;

pub(crate) const BRANDS: [[u8; 4]; 4] = [*b"isom", *b"iso6", *b"avc1", *b"mp41"];
const PLACEHOLDER_LEN: u64 = 16;
const SYNC_INTERVAL_SECS: u64 = 5;
const TFHD_DEFAULT_BASE_IS_MOOF: u32 = 0x02_0000;
const TFHD_DEFAULT_SAMPLE_FLAGS: u32 = 0x20;
const TRUN_VIDEO_FLAGS: u32 = 0x701;
const TRUN_AUDIO_FLAGS: u32 = 0x301;

#[derive(Debug)]
struct Queued {
    data: Arc<[u8]>,
    duration: u32,
    sync: bool,
}

#[derive(Debug, Default)]
struct TrackState {
    pending: Option<Packet>,
    queued: Vec<Queued>,
    rows: Vec<SampleRow>,
    chunks: Vec<Chunk>,
    flushed_ticks: u64,
    last_duration: u32,
}

impl TrackState {
    fn accept(&mut self, packet: &Packet) -> Result<()> {
        if let Some(prev) = &self.pending {
            let delta = packet.dts - prev.dts;
            if delta <= 0 {
                return Err(Mp4Error::Invalid(format!(
                    "dts {} does not follow {}",
                    packet.dts, prev.dts
                )));
            }
            let duration = u32::try_from(delta).unwrap_or(u32::MAX);
            self.queue_pending(duration);
        }
        Ok(())
    }

    fn queue_pending(&mut self, duration: u32) {
        if let Some(prev) = self.pending.take() {
            self.last_duration = duration;
            self.queued.push(Queued {
                data: prev.data,
                duration,
                sync: prev.key,
            });
        }
    }

    fn close(&mut self) {
        let fallback = self.pending.as_ref().map_or(0, |p| p.duration);
        let duration = if self.last_duration > 0 {
            self.last_duration
        } else {
            fallback
        };
        self.queue_pending(duration);
    }

    fn queued_bytes(&self) -> usize {
        self.queued.iter().map(|s| s.data.len()).sum()
    }
}

struct TrafInput<'a> {
    track_id: u32,
    base_time: u64,
    samples: &'a [Queued],
    data_offset: i32,
    video: bool,
}

#[derive(Debug)]
pub struct FragmentedWriter {
    file: File,
    video_format: VideoFormat,
    audio_format: Option<AudioFormat>,
    video: TrackState,
    audio: TrackState,
    placeholder_offset: u64,
    pos: u64,
    sequence: u32,
    synced_video_ticks: u64,
}

impl FragmentedWriter {
    pub fn create(path: &Path, video: &VideoFormat, audio: Option<&AudioFormat>) -> Result<Self> {
        let mut head = write_ftyp(b"isom", &BRANDS);
        let placeholder_offset = head.len() as u64;
        let mut placeholder = BoxWriter::new();
        placeholder.u32(PLACEHOLDER_LEN as u32);
        placeholder.bytes(b"free");
        placeholder.zeros(8);
        head.extend_from_slice(&placeholder.into_inner());
        head.extend_from_slice(&write_init_moov(video, audio));

        let mut file = File::create(path)?;
        file.write_all(&head)?;
        Ok(Self {
            file,
            video_format: video.clone(),
            audio_format: audio.cloned(),
            video: TrackState::default(),
            audio: TrackState::default(),
            placeholder_offset,
            pos: head.len() as u64,
            sequence: 0,
            synced_video_ticks: 0,
        })
    }

    pub fn push(&mut self, packet: &Packet) -> Result<()> {
        match packet.track {
            Track::Video => self.push_video(packet),
            Track::Audio => {
                if self.audio_format.is_none() {
                    return Err(Mp4Error::Invalid(
                        "audio packet without audio format".into(),
                    ));
                }
                self.audio.accept(packet)?;
                self.audio.pending = Some(packet.clone());
                Ok(())
            }
        }
    }

    fn push_video(&mut self, packet: &Packet) -> Result<()> {
        if self.video.pending.is_none() && self.video.rows.is_empty() && !packet.key {
            return Err(Mp4Error::Invalid(
                "first video packet is not a key frame".into(),
            ));
        }
        self.video.accept(packet)?;
        if packet.key && !self.video.queued.is_empty() {
            self.flush()?;
        }
        self.video.pending = Some(packet.clone());
        Ok(())
    }

    pub fn finish(mut self) -> Result<FinishedInfo> {
        self.video.close();
        self.audio.close();
        self.flush()?;
        if self.video.rows.is_empty() {
            return Err(Mp4Error::Invalid("no video samples".into()));
        }
        let tables = SampleTables {
            video: std::mem::take(&mut self.video.rows),
            audio: std::mem::take(&mut self.audio.rows),
        };
        let chunking = Chunking {
            video: std::mem::take(&mut self.video.chunks),
            audio: std::mem::take(&mut self.audio.chunks),
        };
        finalize(
            &mut self.file,
            self.placeholder_offset,
            &self.video_format,
            self.audio_format.as_ref(),
            &tables,
            &chunking,
        )
    }

    fn flush(&mut self) -> Result<()> {
        if self.video.queued.is_empty() && self.audio.queued.is_empty() {
            return Ok(());
        }
        self.sequence += 1;
        let video_bytes = self.video.queued_bytes();
        let moof_len = self.moof(0, 0).len();
        let video_offset = moof_len + 8;
        let audio_offset = video_offset + video_bytes;
        let moof = self.moof(video_offset as i32, audio_offset as i32);

        let mdat_len = 8 + video_bytes + self.audio.queued_bytes();
        let mut buf = Vec::with_capacity(moof.len() + mdat_len);
        buf.extend_from_slice(&moof);
        buf.extend_from_slice(&u32::try_from(mdat_len).unwrap_or(u32::MAX).to_be_bytes());
        buf.extend_from_slice(b"mdat");
        for sample in self.video.queued.iter().chain(&self.audio.queued) {
            buf.extend_from_slice(&sample.data);
        }
        self.file.write_all(&buf)?;

        let base = self.pos;
        record(&mut self.video, base + video_offset as u64);
        record(&mut self.audio, base + audio_offset as u64);
        self.pos += buf.len() as u64;

        let video_ticks = self.video.flushed_ticks;
        if video_ticks - self.synced_video_ticks
            >= SYNC_INTERVAL_SECS * u64::from(VideoFormat::TIMESCALE)
        {
            self.file.sync_data()?;
            self.synced_video_ticks = video_ticks;
        }
        Ok(())
    }

    fn moof(&self, video_offset: i32, audio_offset: i32) -> Vec<u8> {
        let inputs = [
            (!self.video.queued.is_empty()).then_some(TrafInput {
                track_id: VIDEO_TRACK_ID,
                base_time: self.video.flushed_ticks,
                samples: &self.video.queued,
                data_offset: video_offset,
                video: true,
            }),
            (!self.audio.queued.is_empty()).then_some(TrafInput {
                track_id: AUDIO_TRACK_ID,
                base_time: self.audio.flushed_ticks,
                samples: &self.audio.queued,
                data_offset: audio_offset,
                video: false,
            }),
        ];
        let mut w = BoxWriter::new();
        w.boxed(b"moof", |w| {
            w.full_boxed(b"mfhd", 0, 0, |w| w.u32(self.sequence));
            for input in inputs.iter().flatten() {
                write_traf(w, input);
            }
        });
        w.into_inner()
    }
}

fn record(track: &mut TrackState, mut offset: u64) {
    let first_sample = track.rows.len() as u32;
    let chunk_offset = offset;
    let count = track.queued.len() as u32;
    for sample in track.queued.drain(..) {
        track.rows.push(SampleRow {
            offset,
            size: sample.data.len() as u32,
            duration: sample.duration,
            sync: sample.sync,
        });
        offset += sample.data.len() as u64;
        track.flushed_ticks += u64::from(sample.duration);
    }
    if count > 0 {
        track.chunks.push(Chunk {
            first_sample,
            count,
            offset: chunk_offset,
        });
    }
}

fn write_traf(w: &mut BoxWriter, input: &TrafInput) {
    w.boxed(b"traf", |w| {
        if input.video {
            w.full_boxed(b"tfhd", 0, TFHD_DEFAULT_BASE_IS_MOOF, |w| {
                w.u32(input.track_id)
            });
        } else {
            w.full_boxed(
                b"tfhd",
                0,
                TFHD_DEFAULT_BASE_IS_MOOF | TFHD_DEFAULT_SAMPLE_FLAGS,
                |w| {
                    w.u32(input.track_id);
                    w.u32(SYNC_SAMPLE_FLAGS);
                },
            );
        }
        w.full_boxed(b"tfdt", 1, 0, |w| w.u64(input.base_time));
        let flags = if input.video {
            TRUN_VIDEO_FLAGS
        } else {
            TRUN_AUDIO_FLAGS
        };
        w.full_boxed(b"trun", 0, flags, |w| {
            w.u32(input.samples.len() as u32);
            w.i32(input.data_offset);
            for sample in input.samples {
                w.u32(sample.duration);
                w.u32(sample.data.len() as u32);
                if input.video {
                    w.u32(if sample.sync {
                        SYNC_SAMPLE_FLAGS
                    } else {
                        NON_SYNC_SAMPLE_FLAGS
                    });
                }
            }
        });
    });
}
