use greyscreen_core::media::{AudioFormat, VideoFormat};

use crate::aac::write_esds;
use crate::avc::write_avcc_box;
use crate::bytes::BoxWriter;

const MOVIE_TIMESCALE: u32 = 1000;
pub(crate) const VIDEO_TRACK_ID: u32 = 1;
pub(crate) const AUDIO_TRACK_ID: u32 = 2;
const MINOR_VERSION: u32 = 0x200;
pub(crate) const SYNC_SAMPLE_FLAGS: u32 = 0x0200_0000;
pub(crate) const NON_SYNC_SAMPLE_FLAGS: u32 = 0x0101_0000;
const IDENTITY_MATRIX: [u32; 9] = [0x0001_0000, 0, 0, 0, 0x0001_0000, 0, 0, 0, 0x4000_0000];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SampleRow {
    pub offset: u64,
    pub size: u32,
    pub duration: u32,
    pub sync: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SampleTables {
    pub video: Vec<SampleRow>,
    pub audio: Vec<SampleRow>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Chunk {
    pub first_sample: u32,
    pub count: u32,
    pub offset: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Chunking {
    pub video: Vec<Chunk>,
    pub audio: Vec<Chunk>,
}

pub fn chunk_per_sample(rows: &[SampleRow]) -> Vec<Chunk> {
    rows.iter()
        .enumerate()
        .map(|(i, row)| Chunk {
            first_sample: i as u32,
            count: 1,
            offset: row.offset,
        })
        .collect()
}

pub fn write_ftyp(major: &[u8; 4], compatible: &[[u8; 4]]) -> Vec<u8> {
    let mut w = BoxWriter::new();
    w.boxed(b"ftyp", |w| {
        w.bytes(major);
        w.u32(MINOR_VERSION);
        for brand in compatible {
            w.bytes(brand);
        }
    });
    w.into_inner()
}

pub fn write_init_moov(video: &VideoFormat, audio: Option<&AudioFormat>) -> Vec<u8> {
    build_moov(video, audio, None, 0)
}

pub fn write_full_moov(
    video: &VideoFormat,
    audio: Option<&AudioFormat>,
    tables: &SampleTables,
    chunking: &Chunking,
) -> Vec<u8> {
    write_full_moov_delayed(video, audio, tables, chunking, 0)
}

pub fn write_full_moov_delayed(
    video: &VideoFormat,
    audio: Option<&AudioFormat>,
    tables: &SampleTables,
    chunking: &Chunking,
    audio_delay_ms: u64,
) -> Vec<u8> {
    build_moov(video, audio, Some((tables, chunking)), audio_delay_ms)
}

enum Media<'a> {
    Video(&'a VideoFormat),
    Audio(&'a AudioFormat),
}

struct Track<'a> {
    id: u32,
    timescale: u32,
    media: Media<'a>,
    samples: &'a [SampleRow],
    chunks: &'a [Chunk],
    delay_ms: u64,
}

impl Track<'_> {
    fn duration_ticks(&self) -> u64 {
        self.samples.iter().map(|s| u64::from(s.duration)).sum()
    }

    fn media_movie_duration(&self) -> u64 {
        self.duration_ticks() * u64::from(MOVIE_TIMESCALE) / u64::from(self.timescale)
    }

    fn movie_duration(&self) -> u64 {
        self.delay_ms + self.media_movie_duration()
    }
}

fn build_moov(
    video: &VideoFormat,
    audio: Option<&AudioFormat>,
    full: Option<(&SampleTables, &Chunking)>,
    audio_delay_ms: u64,
) -> Vec<u8> {
    let empty = (&SampleTables::default(), &Chunking::default());
    let (tables, chunking) = full.unwrap_or(empty);
    let mut tracks = vec![Track {
        id: VIDEO_TRACK_ID,
        timescale: VideoFormat::TIMESCALE,
        media: Media::Video(video),
        samples: &tables.video,
        chunks: &chunking.video,
        delay_ms: 0,
    }];
    if let Some(audio) = audio {
        tracks.push(Track {
            id: AUDIO_TRACK_ID,
            timescale: audio.sample_rate,
            media: Media::Audio(audio),
            samples: &tables.audio,
            chunks: &chunking.audio,
            delay_ms: audio_delay_ms,
        });
    }
    let movie_duration = tracks.iter().map(Track::movie_duration).max().unwrap_or(0);

    let mut w = BoxWriter::new();
    w.boxed(b"moov", |w| {
        write_mvhd(w, movie_duration);
        for track in &tracks {
            write_trak(w, track, full.is_some());
        }
        if full.is_none() {
            write_mvex(w, audio.is_some());
        }
    });
    w.into_inner()
}

fn clamp32(v: u64) -> u32 {
    u32::try_from(v).unwrap_or(u32::MAX)
}

fn write_mvhd(w: &mut BoxWriter, duration: u64) {
    w.full_boxed(b"mvhd", 0, 0, |w| {
        w.u32(0);
        w.u32(0);
        w.u32(MOVIE_TIMESCALE);
        w.u32(clamp32(duration));
        w.fixed16_16(1.0);
        w.fixed8_8(1.0);
        w.zeros(10);
        for v in IDENTITY_MATRIX {
            w.u32(v);
        }
        w.zeros(24);
        w.u32(AUDIO_TRACK_ID + 1);
    });
}

fn write_mvex(w: &mut BoxWriter, with_audio: bool) {
    w.boxed(b"mvex", |w| {
        let mut trex = |id: u32, flags: u32| {
            w.full_boxed(b"trex", 0, 0, |w| {
                w.u32(id);
                w.u32(1);
                w.u32(0);
                w.u32(0);
                w.u32(flags);
            });
        };
        trex(VIDEO_TRACK_ID, NON_SYNC_SAMPLE_FLAGS);
        if with_audio {
            trex(AUDIO_TRACK_ID, SYNC_SAMPLE_FLAGS);
        }
    });
}

fn write_trak(w: &mut BoxWriter, track: &Track, full: bool) {
    w.boxed(b"trak", |w| {
        write_tkhd(w, track, track.movie_duration());
        if track.delay_ms > 0 {
            write_delay_edits(w, track);
        }
        w.boxed(b"mdia", |w| {
            w.full_boxed(b"mdhd", 0, 0, |w| {
                w.u32(0);
                w.u32(0);
                w.u32(track.timescale);
                w.u32(clamp32(track.duration_ticks()));
                w.u16(0x55C4);
                w.u16(0);
            });
            write_hdlr(w, track);
            w.boxed(b"minf", |w| {
                match track.media {
                    Media::Video(_) => w.full_boxed(b"vmhd", 0, 1, |w| w.zeros(8)),
                    Media::Audio(_) => w.full_boxed(b"smhd", 0, 0, |w| w.zeros(4)),
                }
                w.boxed(b"dinf", |w| {
                    w.full_boxed(b"dref", 0, 0, |w| {
                        w.u32(1);
                        w.full_boxed(b"url ", 0, 1, |_| {});
                    });
                });
                write_stbl(w, track, full);
            });
        });
    });
}

fn write_delay_edits(w: &mut BoxWriter, track: &Track) {
    w.boxed(b"edts", |w| {
        w.full_boxed(b"elst", 0, 0, |w| {
            w.u32(2);
            w.u32(clamp32(track.delay_ms));
            w.i32(-1);
            w.u16(1);
            w.u16(0);
            w.u32(clamp32(track.media_movie_duration()));
            w.i32(0);
            w.u16(1);
            w.u16(0);
        });
    });
}

fn write_tkhd(w: &mut BoxWriter, track: &Track, duration: u64) {
    w.full_boxed(b"tkhd", 0, 3, |w| {
        w.u32(0);
        w.u32(0);
        w.u32(track.id);
        w.u32(0);
        w.u32(clamp32(duration));
        w.zeros(8);
        w.u16(0);
        w.u16(0);
        match track.media {
            Media::Video(_) => w.fixed8_8(0.0),
            Media::Audio(_) => w.fixed8_8(1.0),
        }
        w.u16(0);
        for v in IDENTITY_MATRIX {
            w.u32(v);
        }
        let (width, height) = match track.media {
            Media::Video(v) => (f64::from(v.width), f64::from(v.height)),
            Media::Audio(_) => (0.0, 0.0),
        };
        w.fixed16_16(width);
        w.fixed16_16(height);
    });
}

fn write_hdlr(w: &mut BoxWriter, track: &Track) {
    let (kind, name): (&[u8; 4], &[u8]) = match track.media {
        Media::Video(_) => (b"vide", b"VideoHandler\0"),
        Media::Audio(_) => (b"soun", b"SoundHandler\0"),
    };
    w.full_boxed(b"hdlr", 0, 0, |w| {
        w.u32(0);
        w.bytes(kind);
        w.zeros(12);
        w.bytes(name);
    });
}

fn write_stbl(w: &mut BoxWriter, track: &Track, full: bool) {
    w.boxed(b"stbl", |w| {
        w.full_boxed(b"stsd", 0, 0, |w| {
            w.u32(1);
            match track.media {
                Media::Video(v) => write_avc1(w, v),
                Media::Audio(a) => write_mp4a(w, a),
            }
        });
        write_stts(w, track.samples);
        if full && matches!(track.media, Media::Video(_)) {
            write_stss(w, track.samples);
        }
        write_stsc(w, track.chunks);
        write_stsz(w, track.samples);
        if full {
            w.full_boxed(b"co64", 0, 0, |w| {
                w.u32(track.chunks.len() as u32);
                for chunk in track.chunks {
                    w.u64(chunk.offset);
                }
            });
        } else {
            w.full_boxed(b"stco", 0, 0, |w| w.u32(0));
        }
    });
}

fn write_avc1(w: &mut BoxWriter, video: &VideoFormat) {
    w.boxed(b"avc1", |w| {
        w.zeros(6);
        w.u16(1);
        w.zeros(16);
        w.u16(video.width as u16);
        w.u16(video.height as u16);
        w.fixed16_16(72.0);
        w.fixed16_16(72.0);
        w.u32(0);
        w.u16(1);
        w.zeros(32);
        w.u16(0x0018);
        w.u16(0xFFFF);
        write_avcc_box(w, &video.sps, &video.pps);
        w.boxed(b"colr", |w| {
            w.bytes(b"nclx");
            w.u16(1);
            w.u16(1);
            w.u16(1);
            w.u8(0);
        });
        w.boxed(b"pasp", |w| {
            w.u32(1);
            w.u32(1);
        });
    });
}

fn write_mp4a(w: &mut BoxWriter, audio: &AudioFormat) {
    w.boxed(b"mp4a", |w| {
        w.zeros(6);
        w.u16(1);
        w.zeros(8);
        w.u16(audio.channels);
        w.u16(16);
        w.u32(0);
        let rate = u16::try_from(audio.sample_rate).unwrap_or(0);
        w.u32(u32::from(rate) << 16);
        write_esds(w, &audio.asc, audio.bitrate);
    });
}

fn write_stts(w: &mut BoxWriter, samples: &[SampleRow]) {
    let mut runs: Vec<(u32, u32)> = Vec::new();
    for sample in samples {
        match runs.last_mut() {
            Some((count, duration)) if *duration == sample.duration => *count += 1,
            _ => runs.push((1, sample.duration)),
        }
    }
    w.full_boxed(b"stts", 0, 0, |w| {
        w.u32(runs.len() as u32);
        for (count, duration) in runs {
            w.u32(count);
            w.u32(duration);
        }
    });
}

fn write_stss(w: &mut BoxWriter, samples: &[SampleRow]) {
    let sync: Vec<u32> = samples
        .iter()
        .enumerate()
        .filter(|(_, s)| s.sync)
        .map(|(i, _)| i as u32 + 1)
        .collect();
    w.full_boxed(b"stss", 0, 0, |w| {
        w.u32(sync.len() as u32);
        for index in sync {
            w.u32(index);
        }
    });
}

fn write_stsc(w: &mut BoxWriter, chunks: &[Chunk]) {
    let mut entries: Vec<(u32, u32)> = Vec::new();
    for (i, chunk) in chunks.iter().enumerate() {
        if entries
            .last()
            .is_none_or(|&(_, count)| count != chunk.count)
        {
            entries.push((i as u32 + 1, chunk.count));
        }
    }
    w.full_boxed(b"stsc", 0, 0, |w| {
        w.u32(entries.len() as u32);
        for (first_chunk, count) in entries {
            w.u32(first_chunk);
            w.u32(count);
            w.u32(1);
        }
    });
}

fn write_stsz(w: &mut BoxWriter, samples: &[SampleRow]) {
    w.full_boxed(b"stsz", 0, 0, |w| {
        w.u32(0);
        w.u32(samples.len() as u32);
        for sample in samples {
            w.u32(sample.size);
        }
    });
}

#[cfg(test)]
mod tests;
