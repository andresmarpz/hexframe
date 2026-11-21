use greyscreen_core::media::{AudioFormat, VideoFormat};

use super::cursor::Cursor;
use crate::reader::{BoxRef, Boxes, boxes};
use crate::{Mp4Error, Result};

const AVC1_HEADER_LEN: usize = 78;
const MP4A_HEADER_LEN: usize = 28;

fn invalid(what: &str) -> Mp4Error {
    Mp4Error::Invalid(format!("init moov: {what}"))
}

fn child_path(data: &[u8], start: BoxRef, path: &[&[u8; 4]]) -> Option<BoxRef> {
    path.iter()
        .try_fold(start, |parent, fourcc| parent.find(data, fourcc))
}

pub(crate) fn read_formats(
    init: &[u8],
    first_video_duration: u32,
) -> Result<(VideoFormat, Option<AudioFormat>)> {
    let moov = boxes(init)
        .find(|b| &b.fourcc == b"moov")
        .ok_or_else(|| invalid("no moov"))?;
    let mut video = None;
    let mut audio = None;
    for trak in moov.children(init).filter(|b| &b.fourcc == b"trak") {
        let stbl = child_path(init, trak.clone(), &[b"mdia", b"minf", b"stbl"])
            .ok_or_else(|| invalid("no stbl"))?;
        let stsd = stbl.find(init, b"stsd").ok_or_else(|| invalid("no stsd"))?;
        let entries = Boxes::within(init, stsd.payload.start + 8..stsd.payload.end);
        for entry in entries {
            match &entry.fourcc {
                b"avc1" => video = Some(parse_avc1(init, &entry)?),
                b"mp4a" => {
                    let mdhd = child_path(init, trak.clone(), &[b"mdia", b"mdhd"])
                        .ok_or_else(|| invalid("no mdhd"))?;
                    audio = Some(parse_mp4a(init, &entry, &mdhd)?);
                }
                _ => {}
            }
        }
    }
    let mut video = video.ok_or_else(|| invalid("no video track"))?;
    video.fps = (VideoFormat::TIMESCALE + first_video_duration / 2)
        .checked_div(first_video_duration)
        .unwrap_or(0)
        .max(1);
    Ok((video, audio))
}

fn parse_avc1(data: &[u8], entry: &BoxRef) -> Result<VideoFormat> {
    let payload = entry.payload_bytes(data);
    let mut head = Cursor::new(payload);
    head.take(24).ok_or_else(|| invalid("short avc1"))?;
    let width = head.u16().ok_or_else(|| invalid("short avc1"))?;
    let height = head.u16().ok_or_else(|| invalid("short avc1"))?;
    let avcc = Boxes::within(
        data,
        entry.payload.start + AVC1_HEADER_LEN..entry.payload.end,
    )
    .find(|b| &b.fourcc == b"avcC")
    .ok_or_else(|| invalid("no avcC"))?;
    let mut cur = Cursor::new(avcc.payload_bytes(data));
    let short = || invalid("short avcC");
    cur.take(5).ok_or_else(short)?;
    let sps_count = cur.u8().ok_or_else(short)? & 0x1F;
    let sps = first_nal(&mut cur, sps_count).ok_or_else(short)?;
    let pps_count = cur.u8().ok_or_else(short)?;
    let pps = first_nal(&mut cur, pps_count).ok_or_else(short)?;
    Ok(VideoFormat {
        width: u32::from(width),
        height: u32::from(height),
        fps: 0,
        sps,
        pps,
    })
}

fn first_nal(cur: &mut Cursor, count: u8) -> Option<Vec<u8>> {
    let mut first = Vec::new();
    for index in 0..count {
        let len = usize::from(cur.u16()?);
        let nal = cur.take(len)?;
        if index == 0 {
            first = nal.to_vec();
        }
    }
    Some(first)
}

fn parse_mp4a(data: &[u8], entry: &BoxRef, mdhd: &BoxRef) -> Result<AudioFormat> {
    let short = || invalid("short mp4a");
    let mut head = Cursor::new(entry.payload_bytes(data));
    head.take(16).ok_or_else(short)?;
    let channels = head.u16().ok_or_else(short)?;

    let mut mdhd_cur = Cursor::new(mdhd.payload_bytes(data));
    let version = mdhd_cur.u8().ok_or_else(short)?;
    mdhd_cur.take(3).ok_or_else(short)?;
    mdhd_cur
        .take(if version == 1 { 16 } else { 8 })
        .ok_or_else(short)?;
    let sample_rate = mdhd_cur.u32().ok_or_else(short)?;

    let esds = Boxes::within(
        data,
        entry.payload.start + MP4A_HEADER_LEN..entry.payload.end,
    )
    .find(|b| &b.fourcc == b"esds")
    .ok_or_else(|| invalid("no esds"))?;
    let (bitrate, asc) = parse_esds(esds.payload_bytes(data)).ok_or_else(|| invalid("bad esds"))?;
    Ok(AudioFormat {
        sample_rate,
        channels,
        bitrate,
        asc,
    })
}

fn descriptor(bytes: &[u8]) -> Option<(u8, &[u8])> {
    let mut cur = Cursor::new(bytes);
    let tag = cur.u8()?;
    let mut len = 0usize;
    for _ in 0..4 {
        let byte = cur.u8()?;
        len = (len << 7) | usize::from(byte & 0x7F);
        if byte & 0x80 == 0 {
            break;
        }
    }
    Some((tag, cur.take(len)?))
}

fn parse_esds(payload: &[u8]) -> Option<(u32, Vec<u8>)> {
    let mut cur = Cursor::new(payload);
    cur.take(4)?;
    let (_, es) = descriptor(cur.rest())?;
    let (_, config) = descriptor(es.get(3..)?)?;
    let mut fields = Cursor::new(config);
    fields.take(5 + 4)?;
    let bitrate = fields.u32()?;
    let (tag, asc) = descriptor(fields.rest())?;
    (tag == 5).then(|| (bitrate, asc.to_vec()))
}
