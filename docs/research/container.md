# Container, muxing, crash safety, clip cutting

Research date: 2026-11-20. Scope: how greyscreen stores H.264/AAC from Media Foundation, survives a crash, and cuts clips without re-encoding.

Conventions: "UNVERIFIED" marks a claim I could not confirm from a primary source or from code I read. "Secondary" marks a claim from a third-party project or forum, not from Microsoft or a spec. Crate facts come from the crate sources I downloaded from crates.io and read (not only the READMEs).

## Summary (5 lines)

1. Do not let `IMFSinkWriter` own the encoder or the file. Drive the H.264 and AAC MFTs directly, then fan the encoded packets out to (1) a full-game file writer and (2) an in-memory clip ring. Only this design lets one encoder feed both.
2. Plain `MFTranscodeContainerType_MPEG4` writes the `moov` at the end (default, documented). A kill leaves no `moov`, so the file is not playable. Reject it for the full-game file.
3. Record the full game as fragmented MP4 (fMP4), one fragment per 1-second GOP, with a 16-byte `free` placeholder after `ftyp`. On clean stop, "soft remux" to a regular MP4 (append a full `moov`, overwrite the placeholder with an `mdat` header). This is the OBS "Hybrid MP4" design. On crash, the file is still valid fMP4 and a startup recovery pass finalizes it.
4. Clips: keep the last ~40 s of encoded packets in RAM (Arc'd, from an IDR), and write each clip as a regular faststart MP4 from that ring with no re-encode. Cutting later from the finished game file at keyframes is a v1.1 add-on that reuses the same code.
5. Mux in Rust with `shiguredo_mp4 =2026.5.0` (fMP4 segment muxer, regular MP4 muxer with faststart, Annex B to AVCC helpers, avcC/esds builders, zero dependencies). Metadata goes in a sidecar JSON; chapters (`chpl`) are an optional later nicety.

## Comparison table

| | (a) SinkWriter plain MP4 | (b) SinkWriter fMP4 (`MFCreateFMPEG4MediaSink` / `FMPEG4` container) | (c) Own muxing in Rust (recommended) | (d) MKV then remux |
|---|---|---|---|---|
| Exists / how | `MFTranscodeContainerType_MPEG4`. `moov` after `mdat` by default. | Exists. Win 8+ (`MFCreateFMPEG4MediaSink`), `MFTranscodeContainerType_FMPEG4` (GUID `9ba876f1-...`). Fragment length via `MF_MPEG4SINK_MIN_FRAGMENT_DURATION` (UINT64, hns, Win10 1709+). | Encoder MFT output goes to our code. | No MF MKV sink. Writer is our own EBML code (no usable writer crate). |
| Crash or kill | `moov` is written at finalize. Kill leaves no `moov`: unplayable (documented behaviour; I did not kill-test it). | Playable up to the last complete fragment (design property of fMP4; MF-specific kill test not done). | Same as (b), and we control fragment size and flush. | Playable (MKV clusters are self-contained). |
| Plays in WMP / Films & TV | Yes. | `moof` is supported by the MF MPEG-4 source since Win 8, `mfra` is not. A Win11 Media Player test on fMP4 with zero durations showed no duration and no seeking (secondary). UNVERIFIED for our files. | Final hybrid file is a regular MP4: yes. Un-finalized crash file has the (b) risk. | No (needs remux). |
| VLC / Chrome | VLC yes, Chrome yes. | VLC plays fMP4 (secondary). Chrome plays it, may read a lot before start (secondary). | Final regular MP4: yes. | VLC yes. Chrome no. |
| Discord / Premiere | Fine (standard MP4, after `moov`). Discord faststart need UNVERIFIED. | Discord: UNVERIFIED. Premiere: OBS KB rates fMP4 "good", Adobe forum reports are mixed on OBS files (secondary). | Regular MP4: the safe type. | Premiere: poor, needs remux. |
| One encoder to file and clips | No. Sink writer owns the encoder. | No (can only re-parse the bytes it writes, ~300 ms to ~2 s late, fragment cuts happen on keyframes). | Yes. | No. |
| Known defects | B-frames need DTS set or frames are dropped (documented). | Gap at each fragment end is lost (sink uses nominal last-sample duration; openscreen #1164). Mediabunny #578: MF fMP4 had no `tfdt` and `tfra` pointing mid-fragment (conflicts with openscreen, which says `tfdt` is present). H.264 only (MuRecorder, secondary). | We own the bugs. About 600-900 lines. | Full read+write remux of GBs at stop. |
| Verdict | Reject. | Fallback only. | **Use.** | Reject (no gain over fMP4 hybrid, costs a full copy). |

### Crates evaluated for (c)

| Crate | Version (date) | Writes fMP4? | Verdict |
|---|---|---|---|
| `shiguredo_mp4` | 2026.5.0 (2026-10-04), Apache-2.0, MSRV 1.93, zero deps, no_std | Yes: `Fmp4SegmentMuxer` (init segment, `moof`+`mdat` header, optional `sidx`, `mfra_bytes()`, `trun` composition offsets). Also `Mp4FileMuxer` (regular MP4, optional faststart). `bitstream::h264` (Annex B to length-prefixed, SPS parse, `build_avc1_box`), `bitstream::aac` (`build_mp4a_box`). | **Primary.** Risks: CalVer versions (pin with `=`), Japanese-only issue channel, Sans-I/O API. No `chpl`/`udta` type (use `unknown_boxes`). |
| `mp4-atom` | 0.17.1 (2026-11-04), MIT/Apache-2.0, MSRV 1.88 | Box codec only (no muxing logic). Has `Moof`, `Traf`, `Tfhd`, `Tfdt`, `Trun`, `Mfra`, `Tfra`, `Sidx`, `Ctts`, `Avcc`, `Esds`, `Chpl`, `Udta`. | **Fallback** for boxes `shiguredo_mp4` lacks (e.g. `chpl`), and for the crash-recovery scanner if needed. Actively maintained. |
| `mp4` (alfg/mp4-rust) | 0.14.0 (2023-08-01) | No. `Mp4Writer` has `write_start/add_track/write_sample/write_end` only, no `moof` writing (I grepped the source). Fragment box types exist for reading. Fragmented-write PRs #114, #137 not merged as far as the issue list shows. | Reject (stale, no fMP4 writer). |
| `muxide` | 0.5.0 (2026-10-19), MIT/Apache-2.0, single maintainer | Yes, `FragmentedMuxer`. But `flush_segment` sets the next base time to `last dts + average duration` (not the last sample's real duration), which drifts with variable frame rate. It copies every sample into a `Vec`, wants Annex-B/ADTS-style inputs in places, and ships an audit file listing earlier bugs. | Reject (too young, timing flaw for VFR). |
| `re_mp4` | 0.5.1 (2026-07-08) | No. Parser only (fork of `mp4`). | Reject for muxing. Could parse for recovery, but `shiguredo_mp4` demux does too. |
| `mp4e` | 1.0.5 (2025-10-14), MIT, ~2.1k lines, 3.6k downloads | Yes (`Mp4e::new_with_fragment`). Not read in depth. | Reject (small, no `mfra`/`ctts` control checked). UNVERIFIED depth. |
| `minimp4` 0.1.2 | 2024-06-19 | C binding of minimp4 (needs a C build under cargo-xwin). | Reject. |
| MKV: `matroska` 0.30.1 | 2026-04-15 | Parse only. | No writer. |
| MKV: `mkv` 0.0.8 | 2021-08-31, "incomplete yet" | Parser/generator, abandoned. | Reject. |
| MKV: `webm` 2.2.1 | 2026-08-25 | libwebm C++ wrapper (VP8/VP9/AV1 oriented). H.264 support not verified. | Reject. UNVERIFIED for H.264. |
| Hand-written | n/a | n/a | About 900-1100 lines with no crate, about 500-700 with `shiguredo_mp4`. Not needed. |

## Why option (c) wins the clip question

Goal: one encoded stream feeds a full-game file and clips.

- `IMFSinkWriter` is a closed pipeline: it loads the encoder and hands encoded samples only to its media sink. There is no tap. Everything encoded goes into the one sink.
- Three ways to get at the encoded packets:
  1. **Drive the MFTs directly (recommended).** `MFTEnumEx`/`CoCreateInstance` for the H.264 encoder and `CLSID_AACMFTEncoder`. The encoder-vertical owner handles async hardware MFT events (`MF_TRANSFORM_ASYNC_UNLOCK`, `METransformNeedInput`, `METransformHaveOutput`). We get `IMFSample` out and fan it out in plain Rust. This vertical assumes it. If the encoder vertical picks something else, tell me.
  2. Sink writer plus a custom `IMFMediaSink` (MuRecorder does this, secondary). Works, but a custom sink needs a long list of COM quirks (stream count 0 at start, one COM object per stream, events with `pvarContextValue`, etc.). More code than option 1 for no gain.
  3. Sink writer plus `MFCreateFMPEG4MediaSink` onto our own byte stream, then re-parse what MF wrote. Delays packets by a fragment, inherits the MF fragment timing defects, and parses our own output. Fallback only.
- Cost of option 1: you own timestamps, keyframe flags, SPS/PPS and AAC config (all listed below). That is the muxer work anyway.

### Clip Option A (RAM ring) versus Option B (cut from the file)

| | A: ring buffer of encoded packets | B: cut from the full-game file |
|---|---|---|
| Latency | Clip ready `post` seconds after the event | Needs the data on disk (and a readable index) |
| Works with game recording off | Yes | No |
| Start precision | Nearest IDR at or before `T-pre` (GOP = 1 s, so up to 1 s extra) | Same |
| Disk | One extra clip write | Read back from the file being written (page cache), or wait for finalize |
| Cost | RAM: about 20 Mbps x 41 s = about 100 MB (shared `Arc`, no copies) | None |
| Code | Reuses the clip writer | Needs a demuxer and the same clip writer |

Recommendation: A is the v1 clip source. B (re-cut the same clip with different bounds from the finished game file plus the sidecar events) is v1.1 and uses `Mp4FileDemuxer` plus the same clip writer. A also makes it unnecessary to read a file that is still being appended.

## What the MFTs output (verified from Microsoft Learn unless marked)

### H.264 video

- Subtype `MFVideoFormat_H264`: Annex B, 3- or 4-byte start codes (H.264 Video Types page, and MPEG-4 sink requirements: "must conform to the H.264 Annex B format").
- After you set the output type, the Microsoft encoder adds `MF_MT_MPEG_SEQUENCE_HEADER` to it: SPS and PPS NAL units concatenated with start codes (H.264 Video Encoder page, `MF_MT_MPEG_SEQUENCE_HEADER` page). Read it with `GetOutputCurrentType` after `SetOutputType`.
- UNVERIFIED for NVENC/QSV/AMF MFTs: whether they also fill `MF_MT_MPEG_SEQUENCE_HEADER`, and whether every IDR sample carries inline SPS/PPS (MuRecorder says MF emits Annex B with parameter sets inline on the GPU path, secondary). Handle both: take SPS/PPS from the type if present, else from the first IDR sample, else fail the start of recording with a clear error.
- The Microsoft encoder sets `MFSampleExtension_DecodeTimestamp` (hns) from Win 8. Hardware: UNVERIFIED.
- `MFSampleExtension_CleanPoint` (UINT32, default FALSE) marks a random access point. Hardware encoder behaviour: UNVERIFIED. Rule: sample is sync only if it contains NAL type 5 (IDR). Log if `CleanPoint` disagrees.
- B-frames: `CODECAPI_AVEncMPVDefaultBPictureCount` (0 = none, the default; set before `SetOutputType`). Keyframe control: `CODECAPI_AVEncMPVGOPSize` (set to fps, so GOP = 1 s) and `CODECAPI_AVEncVideoForceKeyFrame`.
- **v1: set B-frames to 0.** With B-frames off, DTS = PTS, there is no `ctts`, and the clip start has no negative-offset problem. Keep the `composition_time_offset` path in the muxer wrapper but do not enable it.

### Converting to AVCC (`avcC` and samples)

1. Parse `MF_MT_MPEG_SEQUENCE_HEADER`: split on start codes `00 00 01` / `00 00 00 01`, drop trailing zero bytes of each NAL (`trailing_zero_8bits`). Keep emulation-prevention bytes as they are.
2. `avcC` (via `bitstream::h264::build_avc1_box`): `configurationVersion=1`, `AVCProfileIndication=sps[1]`, `profile_compatibility=sps[2]`, `AVCLevelIndication=sps[3]`, `lengthSizeMinusOne=3`, one SPS, one PPS. For High profile (100/110/122/144) the extension fields (chroma_format, bit depths) are required. `shiguredo_mp4` fills them and rejects a missing value.
3. Each sample: Annex B to 4-byte big-endian length prefixes (`annexb_to_length_prefixed`). Drop NAL types 7 (SPS), 8 (PPS) and 9 (AUD) from the sample: with `avc1` sample entries the parameter sets live in `avcC`. Keep types 1, 5 and 6 (SEI).
4. If a later IDR carries an SPS/PPS that differs from `avcC` (encoder re-init, resolution change), end the file and start a new one. v1 does not support mid-file changes.

### AAC audio (`CLSID_AACMFTEncoder`)

- Input: 16-bit PCM, 44.1 or 48 kHz, 1/2/6 channels. Every input sample needs a time and a non-zero duration (a zero duration makes `ProcessOutput` divide by zero).
- Output subtype `MFAudioFormat_AAC`, `MF_MT_AAC_PAYLOAD_TYPE` = 0 (default) means raw `raw_data_block()`, one frame per sample, 1024 PCM samples per frame (21.33 ms at 48 kHz). No ADTS header. (Payload type 1 = ADTS exists from Win 8, do not use it.)
- Supported bitrates (bytes/s): 12000, 16000, 20000, 24000 (96 to 192 kbps for stereo). AAC-LC only.
- After `SetOutputType` the encoder adds `MF_MT_USER_DATA` = the part of `HEAACWAVEINFO` after the `WAVEFORMATEX`, then the AudioSpecificConfig. The documented example is 14 bytes: 12 bytes of header, then `12 10` (AAC-LC, 44.1 kHz, stereo). So **ASC = `user_data[12..]`**. For 48 kHz stereo LC the ASC is `11 90` (AOT 2, freq index 3, channels 2; computed by hand, matches the structure of the documented `12 10`).
- `esds`: ES_Descriptor (tag 0x03) containing DecoderConfigDescriptor (0x04, objectTypeIndication `0x40`, streamType `0x15`) containing DecoderSpecificInfo (0x05) = the ASC, plus SLConfigDescriptor (0x06, predefined 2). `bitstream::aac::build_mp4a_box` writes this. The MS MPEG-4 sink uses the same `mp4a` + OTI `0x40` form.
- UNVERIFIED: the MF AAC encoder's priming/delay (typically ~1024 samples, ~21 ms). OBS reports encoder delay pitfalls with other AAC encoders. Measure with a clap test on the PC, and see Pitfall 12.

### Timestamps and timescales

- MF uses 100 ns units (hns). Movie timescale 1000. Video track timescale 90000. Audio track timescale 48000 (equals the sample rate, so one AAC frame is exactly 1024 ticks).
- Video: `ticks = round((pts_hns - t0_hns) * 9 / 1000)`. Compute each sample's `duration` as the tick difference to the next sample's DTS (this keeps variable frame rate exact). The last sample copies the previous duration.
- Audio: do not use wall-clock timestamps. Frame `n` starts at `n * 1024` ticks. Fill gaps (WASAPI loopback delivers nothing during silence) with silent PCM before encoding.
- MP4 stores no absolute timestamps, only cumulative durations (the `shiguredo_mp4` docs say the same). Any gap in video must be absorbed into the previous sample's duration. Any gap in audio must be silence.

## Faststart: convert fMP4 to a regular MP4 at the end, or keep fMP4?

Convert (soft remux). Evidence:

- The MF MPEG-4 source supports `moof` (Win 8+) but not `mfra`; Windows Explorer/OBS notes: no duration shown for fMP4, some players do not seek (OBS blog, secondary).
- Win11 Media Player on fMP4 with zero durations: no duration, no seeking, VLC fine (Addpipe, secondary).
- OBS made "Hybrid MP4" its default on Windows for exactly this reason, and it is rated "excellent" for editors versus "good" for plain fMP4 (OBS KB, secondary).
- A full re-copy to faststart (move `moov` to the front) rewrites the whole file (several GB). Soft remux touches only a few MB at the end plus a 16-byte header. MS also notes `MF_MPEG4SINK_MOOV_BEFORE_MDAT` "involves an additional file copying/remuxing" (same idea).

Decision details:

- Full-game file ends as a regular MP4 with `moov` at the end (not "web faststart"). That is fine for local playback and editors. A web or Discord-style progressive upload may wait for the whole file. If the user wants a faststart game file, that is a later optional "Export" action (one file copy), not part of stop.
- **Clips are written faststart directly** (`moov` before `mdat`), since they are small and built from memory.
- `mfra`: not needed. The final file is a regular MP4. For a crash file we do not write `mfra` (we cannot, we crashed); the recovery pass fixes the file. MF cannot read `mfra` anyway.
- `sidx`: not needed for local files.
- Keep a regular `moov` with `stss` so Windows players get a seek table.

## Event metadata: sidecar JSON, chapters, or `udta`

| Option | What players show | Verdict |
|---|---|---|
| Sidecar JSON `<name>.greyscreen.json` | Nothing (our app reads it). Survives crashes if written incrementally. | **v1 source of truth.** |
| Nero `chpl` in `moov/udta` | OBS writes chapters this way at finalize and DaVinci Resolve reads them (OBS KB/release notes). VLC reads `chpl`, but a June 2026 VLC commit prefers an Apple chapter track and falls back to `chpl` (not confirmed shipped). Apple players ignore `chpl`. `chpl` has a one-byte count: **max 255 chapters**. WMP, Films & TV, Premiere, Discord: UNVERIFIED (assume not shown). | Optional v1.1 nicety, written only into the final `moov`. |
| QuickTime chapter track (`tref/chap` + text track) | Apple players, VLC. Needs an extra track. | Skip. |
| `udta`/`meta` with JSON | Ignored by players. | Skip. Sidecar is simpler. |

Sidecar layout: `game.mp4`, `game.greyscreen.json` (final, atomic write via temp+rename), and while recording `game.greyscreen.events.jsonl` (append one JSON line per event, `flush` each). Each event holds `{t_ms, type, ...}` where `t_ms` is milliseconds from the recording `t0`, the same clock as the video timeline. Each clip gets its own `clip-<id>.greyscreen.json` with `source_game`, `event`, `window_start_ms`, `window_end_ms`.

## Recommendation for v1

### Architecture

```
capture -> H.264 MFT -+                           +-> GameWriter thread -> game.mp4.part (fMP4) -> soft remux -> game.mp4
                      +-> Packetizer (AVCC) -> Arc<Packet> --+
PCM -> AAC MFT -------+                           +-> ClipRing (RAM, Mutex<VecDeque<Arc<Packet>>>)
                                                           \-> ClipJob thread -> clip-<id>.mp4 (regular, faststart)
```

`Packet { track, dts_ticks, pts_ticks, duration_ticks, sync: bool, data: Bytes (AVCC or raw AAC) }`.

### Settings

- H.264: B-frames 0, GOP = fps (1 s) with forced IDR, no mid-stream parameter-set changes.
- Fragment boundary: every video IDR (one fragment per ~1 s). Audio frames with DTS before the next IDR time go in the same fragment. Each track's `tfdt` = running sum of that track's durations (continuous, so no gaps or overlaps).
- Write each fragment with a single `write_all`. A process crash keeps data (OS cache), so at most the current fragment is lost. Call `sync_data` about every 5 s for power loss (general OS behaviour, not tested here).
- Name the in-progress file `game-<ts>.mp4.part`. Rename to `.mp4` after soft remux succeeds.

### Box layout, recording phase (valid fMP4 at all times)

```
ftyp   major 'isom', minor 0x200, compatible [isom, iso6, avc1, mp41]
free   size = 16 (8 byte header + 8 zero bytes)           <- placeholder, later overwritten
moov
  mvhd   v0, timescale 1000, duration 0, rate 1.0, volume 1.0, identity matrix, next_track_ID 3
  trak (video, track_ID 1)
    tkhd  v0, flags 3, duration 0, width/height 16.16 fixed
    mdia
      mdhd  v0, timescale 90000, duration 0, language 'und'
      hdlr  'vide'
      minf
        vmhd  flags 1
        dinf/dref/url  (flags 1, self-contained)
        stbl
          stsd  avc1 { avcC, optional colr(nclx 1/1/1, limited range), optional pasp 1:1 }
          stts, stsc, stsz, stco  all with entry_count 0
  trak (audio, track_ID 2)
    tkhd  v0, flags 3, volume 1.0, duration 0
    mdia
      mdhd  v0, timescale 48000, language 'und'
      hdlr  'soun'
      minf
        smhd
        dinf/dref/url
        stbl
          stsd  mp4a { channelcount 2, samplesize 16, samplerate 48000<<16, esds }
          stts, stsc, stsz, stco  all empty
  mvex
    trex  track 1, default_sample_description_index 1, durations/sizes 0, flags 0x01010000
    trex  track 2, same, flags 0x02000000
```

Per fragment (one `moof` and one `mdat`):

```
moof
  mfhd   sequence_number (starts at 1, +1 per fragment)
  traf   (video)
    tfhd  flags 0x020000 (default-base-is-moof), track_ID 1
    tfdt  v1, baseMediaDecodeTime (u64)
    trun  flags 0x701 (data_offset | duration | size | sample_flags); version 1 and +0x800 only if B-frames are ever enabled
  traf   (audio)
    tfhd  flags 0x020020 (default-base-is-moof | default-sample-flags), track_ID 2, default_sample_flags 0x02000000
    tfdt  v1
    trun  flags 0x301 (data_offset | duration | size)
mdat     video samples first, then audio samples
```

- `trun.data_offset` is relative to the start of the `moof` (because of default-base-is-moof): `moof_size + 8 + offset_of_track_data_in_mdat_payload`.
- Sample flags (from FFmpeg's `isom.h`, `movenc.c`): sync = `0x02000000` (depends_on = 2); non-sync = `0x01010000` (depends_on = 1 plus is_non_sync `0x00010000`).
- Brands: FFmpeg writes `iso6` when it uses `tfdt` and signed CTS, `iso5` for default-base-is-moof. We use `tfdt` and default-base-is-moof, so `iso6` is correct to list.
- The W3C ISOBMFF byte-stream spec requires `mvex` in the init segment and a `tfdt` in every `traf`. Our file meets both.
- `moov` duration fields are 0 (unknown) in the init segment. Crash files therefore show no duration in some players. Recovery fixes that.

### Soft remux at stop (the "hybrid" step)

1. Keep an in-memory sample table per track while recording: `(file_offset u64, size u32, duration u32, sync bool, cts i32)`. `file_offset` is the absolute offset of the sample bytes in the file.
2. Flush the last fragment. `sync_data`.
3. Append a full `moov` at EOF: `mvhd` with real duration; per track `tkhd`/`mdhd` real durations; `stsd` copied from the init `moov`; `stts` run-length; `stss` for video sync samples; `ctts` only if present; `stsc` one chunk per (fragment, track) as run-length entries; `stsz`; `co64` (always 64-bit). **No `mvex`.** `sync_data`.
4. Overwrite the 16-byte placeholder:
   - If `moov_offset - placeholder_offset - 8` fits in 32 bits (file under 4 GiB): bytes 0..8 become a `free` box of size 8, bytes 8..16 become an `mdat` header with 32-bit size and type `mdat`. The `mdat` ends where the new `moov` starts.
   - Otherwise: one 16-byte `mdat` header: size = 1, type `mdat`, 64-bit largesize.
   - Everything between (the init `moov` and all `moof`/`mdat` fragment boxes) is now just bytes inside the big `mdat`; readers skip it. `sync_data`.
5. Rename `.part` to `.mp4`.

This mirrors the OBS Hybrid MP4 soft-remux (OBS blog: placeholder `free` overwritten by an `mdat` header, `moov` appended) and MuRecorder (which uses `co64` and a 64-bit `mdat` header). It costs about 150-250 lines.

Whether `shiguredo_mp4::Mp4FileMuxer` can be fed these absolute offsets (its `Sample.data_offset` is an explicit field, and `FinalizedBoxes::offset_and_bytes_pairs()` returns what to write) to produce step 3 is UNVERIFIED. Spike it first. If it does not fit, build `StblBox`/`MoovBox` directly from `shiguredo_mp4::boxes`.

### Crash recovery (runs at app start)

For each `*.mp4.part`: parse `ftyp`, `free`, `moov` (init), then `moof`+`mdat` pairs. Stop at the first box whose declared size passes EOF (truncate the file there). Rebuild the sample table from each `trun` (using `tfdt` as base time), then run the soft remux steps 3-5. If recovery fails, rename to `*.mp4.recovered-fmp4` (still valid fMP4, plays in VLC and with `ffmpeg -c copy`). Never delete a `.part`.

### Clip writer

- Trigger: event at media time `T` (ms since `t0`), `pre` (default 15 s), `post` (default 10 s).
- At request: clone the `Arc`s from the IDR at or before `T - pre` to the ring tail. These `Arc`s pin the data against ring pruning. As the ring grows, append packets until the video DTS passes `T + post`.
- Video range: start at the IDR with the largest `dts <= T - pre` (if none is in the ring, the oldest IDR). End at the last sample with `dts <= T + post` (any frame, since B-frames are off).
- Audio range: first AAC frame whose start is at or before the video start, up to the frame covering `T + post`. Zero the clip timeline at that audio frame's start. Video then starts 0 to 21.3 ms after zero, which means up to 21.3 ms of audio lead at clip start. Accept it. (No `elst`: the MS MPEG-4 sink does not support edit lists, so this stays players-agnostic.)
- Rebase DTS so both tracks start at 0, then write `ftyp`, `moov`, `mdat` (regular MP4, faststart) with `Mp4FileMuxer` and `reserved_moov_box_size = estimate_maximum_moov_box_size(&[video_count, audio_count])`. Write to `clip-<id>.mp4.tmp`, rename when done.
- Ring pruning: drop packets older than the newest IDR that is at least `max_pre + GOP` seconds old, except those pinned by a pending clip.
- RAM bound: bitrate x (max_pre 30 s + post 10 s + GOP 1 s). At 20 Mbps that is about 100 MB, plus audio (192 kbps, about 1 MB).

### Crates and features

```toml
shiguredo_mp4 = "=2026.5.0"          # fMP4 + regular MP4 mux, H.264/AAC helpers (Apache-2.0)
mp4-atom      = "=0.17.1"            # fallback: chpl, recovery scan, any missing box (MIT OR Apache-2.0)
windows       = { version = "0.62.2", features = ["Win32_Media_MediaFoundation", "Win32_Media", "Win32_System_Com", "Win32_Foundation"] }
```

The `windows` crate (0.62.2, the latest on crates.io when checked) exports `MFTranscodeContainerType_FMPEG4`, `MFCreateFMPEG4MediaSink`, `MFCreateSinkWriterFromMediaSink`, `MF_MPEG4SINK_MIN_FRAGMENT_DURATION`, `MF_MPEG4SINK_MOOV_BEFORE_MDAT`, `MF_MT_MPEG_SEQUENCE_HEADER`, `MF_MT_USER_DATA`, `MF_MT_AAC_PAYLOAD_TYPE`, `MFSampleExtension_CleanPoint`, `MFSampleExtension_DecodeTimestamp`, `CODECAPI_AVEncMPVGOPSize`, `CODECAPI_AVEncMPVDefaultBPictureCount`, `CODECAPI_AVEncVideoForceKeyFrame` (I grepped the 0.62.2 source). Both muxer crates are pure Rust, so `cargo-xwin` cross-compiles them with no C toolchain.

Use of `Fmp4SegmentMuxer` (note: it learns each track's sample entry from the samples you pass it, so build the init segment after the first fragment):
1. Build the first fragment's `Sample` list (each with `sample_entry: Some(...)` on the first sample per track).
2. Call `create_media_segment_metadata(&samples)` for the `moof` and `mdat` header. Keep it.
3. Call `init_segment_bytes()` and write it (after the `ftyp`/`free` you wrote, or use the `ftyp` it returns, then the `free` placeholder, then the `moov`; check the byte order against the layout above).
4. Write the saved `moof`+`mdat` header, then the sample payloads in the order given by `data_offset`.
5. Repeat step 2 and write for later fragments. Do not call `init_segment_bytes()` again once written.

### Test plan (before shipping)

1. Kill test: `taskkill /F` the app mid-game. Open the `.part` in VLC, then run recovery and open the result in Films & TV, Media Player, VLC, Chrome, Premiere. Check duration, seek and A/V sync.
2. Clean stop: same players, plus a 30-minute file past 4 GiB-worth of bitrate (or simulate with a forced 64-bit path).
3. Clip: 15 s pre, 10 s post. Check the first frame is an IDR, no gray frames, audio in sync, plays in Chrome and a Discord upload.
4. Clap test: measure A/V offset (AAC priming).
5. `ffprobe -show_packets` to compare DTS, PTS, flags with what we wrote.

## Pitfalls

1. MF clock units are hns (100 ns), MP4 wants ticks. Convert once at the packetizer, not per box.
2. Annex B has both 3- and 4-byte start codes in the same stream, and trailing zero bytes. Strip both before measuring NAL lengths.
3. Do not leave SPS/PPS/AUD in `avc1` samples. Do keep them in `avcC`.
4. `avcC` for High profile needs the extension bytes (`0xFC|chroma`, `0xF8|depth` x2, `numSPSExt`), or strict parsers fail.
5. Use IDR (NAL 5), not just `CleanPoint`, for sync samples. Hardware MFT `CleanPoint` behaviour is UNVERIFIED.
6. Hardware encoders may enable B-frames. Set `AVEncMPVDefaultBPictureCount` to 0 before `SetOutputType`, and assert `DTS == PTS` at runtime. If it is not, fail loudly rather than write wrong `ctts`.
7. Variable frame rate (desktop capture): duration of sample *i* = DTS(*i+1*) - DTS(*i*). Never write nominal 1/fps durations. This is the exact bug in MF's fragmented sink (openscreen #1164: about 1.2 ms/s lost).
8. Audio gaps: build the audio timeline from the sample counter. Insert silence for loopback gaps. A drifting audio track cannot be fixed by the MP4 layer.
9. `tfdt` must be a 64-bit (version 1) value and continuous per track across fragments. Use `default-base-is-moof` plus `data_offset`, not `base_data_offset`.
10. `mdhd` version 1 (64-bit duration) is only needed past 2^32 ticks. At 90 kHz that is 13.2 hours, so version 0 is fine for games, but 64-bit `co64` and `mdat` largesize are needed past 4 GiB.
11. The 16-byte placeholder must exist from the first byte written. You cannot add it later without rewriting the file (MuRecorder removed an earlier 1.7 MB reservation because it grew a 5 s clip by 46%, which is why 16 bytes, not a `moov` reservation).
12. AAC priming (about 1024 samples) has no `elst` compensation in v1. Expect up to about 21 ms audio skew (UNVERIFIED for the MF encoder). Measure, then shift audio start by whole AAC frames if needed.
13. `chpl` holds at most 255 chapters (one-byte count). Cap and keep the rest in the sidecar.
14. Process crash is not power loss. `WriteFile` data survives a process kill. Power loss needs `sync_data`.
15. MS MPEG-4 sink limits (if you ever fall back to option (b)): single video track, no `edts`/`elst`, B-frames need DTS or frames are dropped, `mfra` unsupported by the source.
16. Fallback fragment control if ever on MF's sink: set both `MF_TRANSCODE_CONTAINERTYPE = MFTranscodeContainerType_FMPEG4` (the switch) and `MF_MPEG4SINK_MIN_FRAGMENT_DURATION` (hns, UINT64). The duration alone does nothing (MuRecorder, secondary). The sink cuts only at keyframes, so fragments can run longer than the minimum.
17. The `shiguredo_mp4` repo says issues and PRs are not answered unless discussed on a Japanese-only Discord. Pin `=2026.5.0` and vendor it if a fix is needed.
18. Hardware encoder re-init (device lost, GPU reset) changes the SPS/PPS and timestamps. Close the file and start a new part, never splice.
19. Large `moov` from one-chunk-per-sample tables is about 16 bytes per sample (about 2.3 MB for 40 min at 60 fps video, plus audio). Chunk per fragment keeps it smaller.

## Open items (UNVERIFIED, to settle on the Windows PC)

- Real behaviour of Films & TV, Media Player and Premiere on a crash `.part` (fMP4 with no duration).
- Discord upload and embed with a regular faststart H.264/AAC MP4, and Discord's current size limits (not researched).
- Hardware MFT specifics: inline SPS/PPS on every IDR, `MF_MT_MPEG_SEQUENCE_HEADER` presence, `CleanPoint`, `DecodeTimestamp`, B-frame defaults per vendor (NVENC, QSV, AMF).
- MF AAC encoder priming and first-sample timestamp.
- Whether `Mp4FileMuxer` can be reused for the soft-remux `moov`.
- Whether MF's own fMP4 sink writes `tfdt` (openscreen says yes, Mediabunny says no). Only matters for the fallback.
- Films & TV and Media Player behaviour for `mdat` with 64-bit largesize over 4 GiB.

## Sources

Microsoft Learn
- https://learn.microsoft.com/en-us/windows/win32/medfound/mpeg-4-file-sink
- https://learn.microsoft.com/en-us/windows/win32/medfound/mpeg-4-file-source
- https://learn.microsoft.com/en-us/windows/win32/api/mfidl/nf-mfidl-mfcreatefmpeg4mediasink
- https://learn.microsoft.com/en-us/windows/win32/medfound/mf-transcode-containertype
- https://learn.microsoft.com/en-us/windows/win32/medfound/sink-writer-attributes
- https://learn.microsoft.com/en-us/windows/win32/medfound/mf-mpeg4sink-min-fragment-duration
- https://learn.microsoft.com/en-us/windows/win32/medfound/mf-mpeg4sink-moov-before-mdat
- https://learn.microsoft.com/en-us/windows/win32/medfound/mf-mt-mpeg-sequence-header-attribute
- https://learn.microsoft.com/en-us/windows/win32/medfound/h-264-video-encoder
- https://learn.microsoft.com/en-us/windows/win32/directshow/h-264-video-types
- https://learn.microsoft.com/en-us/windows/win32/medfound/aac-encoder
- https://learn.microsoft.com/en-us/windows/win32/medfound/mfsampleextension-cleanpoint-attribute
- https://learn.microsoft.com/en-us/windows/win32/medfound/mfsampleextension-decodetimestamp
- https://support.microsoft.com/en-gb/servicing/os/windows/2017/01/file-duration-is-incorrect-in-a-file-that-is-created-by-media-foundation-mpeg-4-file-sink-in-windows

Specs and format references
- https://www.w3.org/TR/mse-byte-stream-format-isobmff/
- https://raw.githubusercontent.com/FFmpeg/FFmpeg/master/libavformat/isom.h (trun/tfhd/sample flags)
- https://raw.githubusercontent.com/FFmpeg/FFmpeg/master/libavformat/movenc.c (brands, `mfra` trailer)
- https://raw.githubusercontent.com/mingw-w64/mingw-w64/master/mingw-w64-headers/include/mfidl.h (`MFTranscodeContainerType_FMPEG4`)
- ISO/IEC 14496-12 (ISOBMFF), 14496-14 (MP4), 14496-15 (AVC in ISOBMFF): not freely available, not read directly; I relied on the Microsoft pages, the W3C spec and FFmpeg's implementation above.

Crates
- https://crates.io/crates/shiguredo_mp4 and https://github.com/shiguredo/mp4-rs (source 2026.5.0 read: `src/mux_fmp4_segment.rs`, `src/mux_mp4_file.rs`, `src/bitstream/h264.rs`, `src/bitstream/aac.rs`)
- https://crates.io/crates/mp4-atom and https://github.com/kixelated/mp4-atom (0.17.1 source and CHANGELOG read)
- https://crates.io/crates/mp4 and https://github.com/alfg/mp4-rust (0.14.0 source read; PRs listed at https://github.com/alfg/mp4-rust/issues?q=fragmented)
- https://crates.io/crates/muxide and https://github.com/Michael-A-Kuykendall/muxide (0.5.0 source read)
- https://crates.io/crates/re_mp4, https://crates.io/crates/mp4e, https://crates.io/crates/minimp4
- https://crates.io/crates/matroska, https://crates.io/crates/mkv, https://crates.io/crates/webm
- https://crates.io/crates/windows (0.62.2 source grepped for the Media Foundation symbols)

Projects and field reports (secondary)
- https://obsproject.com/kb/hybrid-mp4
- https://obsproject.com/kb/audio-video-formats-guide
- https://obsproject.com/blog/obs-studio-hybrid-mp4
- https://obsproject.com/blog/obs-studio-30-2-release-notes
- https://github.com/katomegumi1999/MuRecorder (Rust MF recorder: fMP4 sink, custom sink, slicing while recording)
- https://github.com/getopenscreen/openscreen/issues/1164 (MF fMP4 sink drops the gap at each fragment end)
- https://github.com/Vanilagy/mediabunny/issues/578 (MF fMP4: ~300 ms fragments, `tfra` mid-fragment)
- https://alax.info/blog/1733 (MF MPEG-4 source timestamp regression on fMP4, Win10 14393 era; fix reported in a comment for 15063, UNVERIFIED)
- https://blog.addpipe.com/duration-in-mp4-files-produced-by-chrome-safari/ (Win11 Media Player: fMP4 with zero durations, no seek)
- https://mailman.videolan.org/pipermail/vlc-commits/2026-June/073356.html (VLC prefers Apple chapter track over `chpl`)
- https://lists.ffmpeg.org/pipermail/ffmpeg-trac/2019-July/049664.html (Nero vs QuickTime chapters)
