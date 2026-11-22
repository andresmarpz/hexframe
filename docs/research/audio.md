# Audio capture and encoding (research)

Researched 2026-11-20. Nothing here was compiled or run: dev is macOS and the Windows PC was not touched. Claims are marked
**VERIFIED** (primary source or working source code read), **UNVERIFIED** (could not confirm, or only from memory), or
**TEST** (needs one experiment on the Windows PC).

## Summary

1. Use WASAPI **process loopback** (`ActivateAudioInterfaceAsync` + `VIRTUAL_AUDIO_DEVICE_PROCESS_LOOPBACK`, include-tree) on the `League of Legends.exe` PID. It captures only the game, never Discord or Spotify, and is not tied to an output device. Needs Windows 10 build 19041+ in practice, 20348+ officially. TEST on the target PC.
2. Capture with the `wasapi` crate (0.27.0, MIT, pins `windows ^0.62.2`). It already implements the activation handler, the `VT_BLOB` PROPVARIANT, the init flags and exposes `GetBuffer` QPC and flags. Hand-write only the Media Foundation side.
3. Loopback may deliver no packets while nothing plays. Treat audio as a sample-accurate timeline anchored to one shared QPC zero point (`t0`). Pad silence for gaps and on every wait timeout, and never use per-packet timestamps as PTS directly.
4. Request 48 kHz stereo float32 from WASAPI, convert to i16 in Rust, feed the sink writer PCM16 48 kHz with an AAC-LC 192 kbps output type. The sink writer inserts `CLSID_AACMFTEncoder` itself for PCM16 to AAC. Float input to AAC is not documented, so do not rely on it.
5. The Microsoft MP4 sink allows exactly **one** audio stream, so a separate mic track is impossible with the sink writer. In v1 the mic is off. When added it must be mixed into the single track.

## Comparison: process loopback vs system loopback

| Aspect | Process loopback (recommended) | System loopback (`AUDCLNT_STREAMFLAGS_LOOPBACK` on default render endpoint) |
|---|---|---|
| Captures | Only the target process (and its child processes with `INCLUDE_TARGET_PROCESS_TREE`). VERIFIED (MS sample page) | Mix of everything playing on that endpoint, all apps, system sounds. VERIFIED (MS loopback docs) |
| Minimum Windows | MS docs: struct page says Windows 10 build 20348, `ActivateAudioInterfaceAsync` page says build 20438 (the two pages disagree). OBS and the win-capture-audio author say it works from Windows 10 2004 (19041). VERIFIED that docs disagree, 19041 claim is secondhand, TEST on the PC | Windows 10 for event-driven loopback (1703+ per MS docs; older versions need a render-stream workaround). VERIFIED |
| Endpoint handling | Not tied to an endpoint, so one client survives default-device changes. VERIFIED (MS sample page) | Bound to one render endpoint. Default device change or hot-unplug gives `AUDCLNT_E_DEVICE_INVALIDATED`, must re-open. VERIFIED (GetBuffer docs for the error) |
| `GetMixFormat` / `GetDevicePeriod` | Not supported on the activated client, so you choose the format. VERIFIED (wcap source comment) | Supported, loopback data is in the device mix format unless `AUTOCONVERTPCM` is used |
| Silence behaviour | MS: "If the process has no render streams, the capturing process receives silence". Whether that is packets flagged SILENT or no packets is not stated. UNVERIFIED, TEST | No packets while nothing plays (hardware clock pauses). Fix used by wcap: run a render stream playing silence. Reported by third parties, same pattern in wcap. VERIFIED in code, not in MS docs |
| Voice chat, music apps | Excluded (good) | Included (bad for a game clip) |
| Complexity | Async activation + PROPVARIANT blob. Handled by the `wasapi` crate | Plain `IMMDevice::Activate`. cpal does this one (device.rs adds `AUDCLNT_STREAMFLAGS_LOOPBACK` for render devices) |
| Failure risk | Older Windows 10 builds, per-build bugs (one OBS forum report of 19042 not working, UNVERIFIED cause) | Works everywhere, but captures other apps |

Fallback: if process loopback activation fails, system loopback is the only alternative. Defer it to v1.1 unless the PC test shows the build matters.

## Which process for League

- Target **`League of Legends.exe`** (the game process, installed under `...\League of Legends\Game\`), with `INCLUDE_TARGET_PROCESS_TREE`.
- The OBS League guide tells users to capture the `League of Legends.exe` window or game for the match, and calls the lobby a separate source (`LeagueClientUx.exe`, Chromium-based). This supports "game audio is from the game process". It does not prove it. **UNVERIFIED, TEST**: capture the PID for 30 s in a real match, dump RMS per second, and confirm announcer, champion voice, spells and ambience are all present.
- I found no authoritative source for the parent process of `League of Legends.exe` (a sandbox report lists Riot Client services as parent). Do not rely on targeting `LeagueClient.exe` to get the game via the tree. UNVERIFIED.
- In-game voice chat: which process plays it is UNVERIFIED. TEST (if Riot voice is played by a different process it would be missing).
- Whether a stream created after activation is captured: implied by the MS "silence if none" wording, TEST (activate before the match audio session exists).
- Pick the PID by exact image name and verify the image path is under the Riot install (file.net warns malware masquerades under the same name). The `wasapi` example comment says that to capture a whole app tree you must target the **parent** PID; here the game process is the root of what we want.
- A new match means a new PID: create a new capture client per game process. The old client goes silent, not an error. UNVERIFIED.

## Recommendation for v1

- Capture: process loopback via the `wasapi` crate 0.27.0. Add it with the same `windows` version as the rest of the project (0.62.2, the latest on crates.io as of this research) to avoid two copies. Fallback if the crate ever blocks us: ~150 lines of hand-written code, the exact calls are in the list below.
- Format: request 48 kHz, 2 channels, 32-bit float via `WAVEFORMATEXTENSIBLE` (the `wasapi` example and wcap both do exactly this). Convert to i16 in Rust with clamping.
- Flags: `AUDCLNT_SHAREMODE_SHARED`, `LOOPBACK | EVENTCALLBACK | AUTOCONVERTPCM | SRC_DEFAULT_QUALITY`, buffer duration 0, periodicity 0.
- Timeline: one `t0` in QPC 100 ns units shared with video. Audio PTS is the running frame count, resynced only by gap fills and drops.
- Encode: sink writer, AAC LC 192 kbps out, PCM16 48 kHz stereo in. Single audio stream.
- Mic: off in v1. Later: mix into the same track (see Mic section).
- Threads: one capture thread, MTA COM, MMCSS `"Pro Audio"`, 50 ms event timeout used as the silence-fill tick.

### API calls in order

windows-rs features (crate `windows = "0.62.2"`). Each name below was checked against the 0.62.2 docs for where the item lives:

```toml
windows = { version = "0.62.2", features = [
  "Win32_Foundation",
  "Win32_Media_Audio",            # IAudioClient, IAudioCaptureClient, ActivateAudioInterfaceAsync, AUDIOCLIENT_*, AUDCLNT_*
  "Win32_Media_KernelStreaming",  # WAVE_FORMAT_EXTENSIBLE, SPEAKER_FRONT_LEFT/RIGHT
  "Win32_Media_Multimedia",       # KSDATAFORMAT_SUBTYPE_IEEE_FLOAT, WAVE_FORMAT_IEEE_FLOAT
  "Win32_Media_MediaFoundation",  # MFStartup, sink writer, MF_MT_*, MFAudioFormat_*, MFCreateSample
  "Win32_System_Com",
  "Win32_System_Com_StructuredStorage", # PROPVARIANT
  "Win32_System_Variant",         # VT_BLOB
  "Win32_System_Threading",       # CreateEventW, WaitForSingleObject, AvSetMmThreadCharacteristicsW
  "Win32_System_Performance",     # QueryPerformanceCounter / Frequency
] }
```

This list mirrors the `wasapi` 0.27.0 `Cargo.toml` plus the MF and QPC features. If you use the crate you only add the MF, Performance and Threading features yourself.

1. **Find the PID.** Enumerate processes (`CreateToolhelp32Snapshot` or `sysinfo`) for `League of Legends.exe`, check the path.
2. **Per capture thread: COM + MMCSS.**
   ```rust
   CoInitializeEx(None, COINIT_MULTITHREADED).ok()?;           // wasapi::initialize_mta()
   let mut task = 0u32;
   let mmcss = AvSetMmThreadCharacteristicsW(w!("Pro Audio"), &mut task)?;
   // end of thread: AvRevertMmThreadCharacteristics(mmcss)?; CoUninitialize();
   ```
   Do not call activation from a UI thread (the `wasapi` docs say so). The completion callback arrives on an MTA worker thread (MS docs). VERIFIED.
3. **Completion handler.** Must be agile. windows-rs `#[implement]` adds `IAgileObject` and `IMarshal` by default (`agile: true` in windows-implement source). VERIFIED.
   ```rust
   #[implement(IActivateAudioInterfaceCompletionHandler)]
   struct Handler(Arc<(Mutex<bool>, Condvar)>);
   impl IActivateAudioInterfaceCompletionHandler_Impl for Handler_Impl {
       fn ActivateCompleted(&self, _op: Ref<'_, IActivateAudioInterfaceAsyncOperation>) -> windows::core::Result<()> {
           let (lock, cv) = &*self.0; *lock.lock().unwrap() = true; cv.notify_one(); Ok(())
       }
   }
   ```
   Do not drop the handler before the callback ran (MS docs).
4. **Activation params in a VT_BLOB PROPVARIANT** (copied from `wasapi` `new_application_loopback_client`; the params struct must stay alive and pinned until activation completes):
   ```rust
   let mut params = AUDIOCLIENT_ACTIVATION_PARAMS {
       ActivationType: AUDIOCLIENT_ACTIVATION_TYPE_PROCESS_LOOPBACK,
       Anonymous: AUDIOCLIENT_ACTIVATION_PARAMS_0 {
           ProcessLoopbackParams: AUDIOCLIENT_PROCESS_LOOPBACK_PARAMS {
               TargetProcessId: pid,
               ProcessLoopbackMode: PROCESS_LOOPBACK_MODE_INCLUDE_TARGET_PROCESS_TREE,
           } } };
   let prop = PROPVARIANT { Anonymous: PROPVARIANT_0 { Anonymous: ManuallyDrop::new(PROPVARIANT_0_0 {
       vt: VT_BLOB, wReserved1: 0, wReserved2: 0, wReserved3: 0,
       Anonymous: PROPVARIANT_0_0_0 { blob: BLOB {
           cbSize: size_of::<AUDIOCLIENT_ACTIVATION_PARAMS>() as u32,
           pBlobData: (&mut params as *mut AUDIOCLIENT_ACTIVATION_PARAMS).cast() } } }) } };
   ```
5. **Activate and wait.**
   ```rust
   let cb: IActivateAudioInterfaceCompletionHandler = Handler(state.clone()).into();
   let op = ActivateAudioInterfaceAsync(VIRTUAL_AUDIO_DEVICE_PROCESS_LOOPBACK, &IAudioClient::IID, Some(&prop), &cb)?;
   // block on the Condvar until ActivateCompleted ran, then:
   let (mut hr, mut unk) = (HRESULT(0), None::<IUnknown>);
   op.GetActivateResult(&mut hr, &mut unk)?; hr.ok()?;
   let client: IAudioClient = unk.unwrap().cast()?;
   ```
6. **Format and Initialize.** `GetMixFormat` and `GetDevicePeriod` are not supported on this client, so do not call them (wcap source comment). Use extensible float 48 kHz stereo (block align 8, avg bytes 384000, `wValidBitsPerSample` 32, mask `SPEAKER_FRONT_LEFT | SPEAKER_FRONT_RIGHT`, subformat `KSDATAFORMAT_SUBTYPE_IEEE_FLOAT`).
   ```rust
   client.Initialize(AUDCLNT_SHAREMODE_SHARED,
       AUDCLNT_STREAMFLAGS_LOOPBACK | AUDCLNT_STREAMFLAGS_EVENTCALLBACK
         | AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM | AUDCLNT_STREAMFLAGS_SRC_DEFAULT_QUALITY,
       0, 0, &fmt.Format, None)?;
   ```
   Buffer duration 0 and periodicity 0 is what the MS sample, the `wasapi` example and the MS Initialize docs (shared + event mode must pass 0/0) use. VERIFIED. wcap passes 1 s and notes the process loopback device does not appear to buffer anyway, so drain the capture client promptly. `AUTOCONVERTPCM` makes the engine convert any device rate or format to ours. `SRC_DEFAULT_QUALITY` selects the better SRC (`AUTOCONVERTPCM` needs it for decent quality, wcap and the crate always pair them). If Initialize returns `AUDCLNT_E_UNSUPPORTED_FORMAT`, retry 48 kHz 16-bit PCM (the MS sample uses 16-bit 44.1 kHz).
7. **Event, service, start.**
   ```rust
   let ev = CreateEventW(None, false, false, None)?;
   client.SetEventHandle(ev)?;                          // before Start, MS docs require it
   let cap: IAudioCaptureClient = client.GetService()?;
   client.Start()?;
   ```
8. **Capture loop (drain, flags, timestamps).**
   ```rust
   loop {
       let w = WaitForSingleObject(ev, 50);              // 50 ms tick, timeout is NOT an error
       if w == WAIT_TIMEOUT { fill_silence_until(now_100ns() - SAFETY); continue; }
       while cap.GetNextPacketSize()? > 0 {
           let (mut p, mut n, mut flags, mut dev_pos, mut qpc) = (null_mut(), 0u32, 0u32, 0u64, 0u64);
           cap.GetBuffer(&mut p, &mut n, &mut flags, Some(&mut dev_pos), Some(&mut qpc))?;
           let silent = flags & AUDCLNT_BUFFERFLAGS_SILENT.0 as u32 != 0;
           let ts_bad = flags & AUDCLNT_BUFFERFLAGS_TIMESTAMP_ERROR.0 as u32 != 0;
           let discont = flags & AUDCLNT_BUFFERFLAGS_DATA_DISCONTINUITY.0 as u32 != 0;
           // copy n frames (or n frames of zeros if silent) into the timeline, see next section
           cap.ReleaseBuffer(n)?;                        // always, even for silent
       }
   }
   ```
   - `GetBuffer` returns the whole packet or none, and `ReleaseBuffer` must be called in the same thread. VERIFIED.
   - `SILENT` means ignore the data and treat it as silence. VERIFIED. The MS sample ignores all flags, do not copy that.
   - `pu64QPCPosition` is **not** raw QPC. It is `10_000_000 * t / f`, already in 100 ns units. VERIFIED (GetBuffer docs). If `GetBuffer` returns `AUDCLNT_S_BUFFER_EMPTY` the QPC and device position are not written, so do not read them.
   - Using the `wasapi` crate, steps 4 to 8 collapse to: `initialize_mta()`, `AudioClient::new_application_loopback_client(pid, true)`, `initialize_client(&WaveFormat::new(32, 32, SampleType::Float, 48000, 2, None), &Direction::Capture, &StreamMode::EventsShared{autoconvert:true, buffer_duration_hns:0}, None)`. The 0.27.0 source takes `direction: Direction` and `stream_mode: StreamMode` by value, check the exact signature against the version you pin. Then `set_get_eventhandle()`, `get_audiocaptureclient()`, `start_stream()`, and `read_from_device` returns `(frames, BufferInfo{flags{silent,data_discontinuity,timestamp_error}, index, timestamp})`. Its `Handle::wait_for_event(ms)` returns `Err` on timeout (the crate example treats that as fatal, we treat it as a tick). VERIFIED from source.
9. **Stop.** `client.Stop()`, `client.Reset()`, drop COM objects, close the event handle.

## Silence, timeline and A/V sync

### The problem

When nothing renders, a loopback client may get no packets and no events (system loopback is documented by wcap and third parties, process loopback is UNVERIFIED, see comparison). The MP4 audio track must still cover the whole clip, otherwise the AAC stream is shorter than video and the A/V offset grows wherever audio had a gap. Also the MS MP4 sink does **not support edit lists** (MS MPEG-4 sink docs), so you cannot fix a start offset in the container: the first audio sample must be at the same zero as the first video sample.

### The design (one rule: audio PTS is a gapless frame count)

- `t0`: one `QueryPerformanceCounter` read converted to 100 ns (`t * 10_000_000 / f`, use u128 math), taken when recording starts and shared with the video thread. Video PTS is `frame_qpc_100ns - t0`. The video vertical must confirm that its frame timestamps (WGC `SystemRelativeTime`) are in the same QPC domain. UNVERIFIED here.
- Audio state: `frames_out: u64` (frames already written to the sink writer, starting at 0 at `t0`).
- Audio PTS and duration: `pts = frames_out * 10_000_000 / 48_000`, `dur = (frames_out + n) * 10_000_000 / 48_000 - pts` (integer, computed from the running count, so rounding never accumulates).
- For each packet with timestamp `ts` (100 ns QPC) and `n` frames:
  1. `expected = t0 + frames_out * 10_000_000 / 48_000`.
  2. `gap = ts - expected`.
  3. If `gap > RESYNC` (suggest 30 ms): emit `gap * 48000 / 1e7` frames of zeros first (the pre-roll before the first packet is the same case, so the first packet naturally pads from `t0`).
  4. If `gap < -RESYNC`: drop `-gap * 48000 / 1e7` frames from the start of this packet (or all of it).
  5. Otherwise ignore small jitter and write the packet as is.
  6. If the packet is `SILENT`, write zeros for `n` frames. If `TIMESTAMP_ERROR` is set or the timestamp is out of range, skip steps 1 to 4 and just append. `DATA_DISCONTINUITY` should be logged, and treated as "resync check now".
- Timeout tick (no event within 50 ms): `target = now_100ns - SAFETY` (suggest 60 ms so late packets can still arrive in order). If `target > expected`, emit zeros up to `target`. This keeps AAC and the sink advancing during silence, and makes live clip export never wait on audio.
- Sanity check on the first packet (the pattern wcap uses for system loopback): if `ts` is more than 500 ms away from QPC-now taken just after `GetBuffer`, distrust device timestamps. Then use `now` at the first packet as the anchor and count frames only. wcap trusts device timestamps for process loopback without this check, so the check is cheap insurance, not a known need.
- Do not use `SendStreamTick` for audio. MS says call it at least once per second during a gap and set `MFSampleExtension_Discontinuity` on the first sample after, but real silence is simpler and keeps the AAC encoder state continuous. VERIFIED (docs).

### Clock drift

The audio engine clock and QPC drift apart by tens of ppm, typically. Because `expected` is the frame count and `ts` is QPC-derived, drift shows up as slowly growing `gap`. At 50 ppm that would be about 3 ms/min, so the 30 ms resync would fire roughly every 10 minutes and is inaudible when inserting or dropping at a packet boundary. Real drift is much smaller:

Measured on 10 game recordings of 16 to 23 minutes (3.3 hours in total, builds 0.1.15 to 0.1.20, game process loopback, one PC), the per-second audio-minus-video skew drifts by -3.3 ms per hour on average (range -9 to +4, about -1 ppm), which is about -10 ms over a 3-hour recording and -27 ms at the worst recording's rate, under a 40 ms bar. No recording shows a 30 ms step in the skew, and `gaps` is 0 in 6 recordings, 1 (31 to 42 ms) in 3 and 231 in one (longest 92 ms, no net drift), so corrections come about once an hour at most, against a trigger of one every 5 minutes. In `game.mp4` the audio and video track lengths differ by -6.7 to +14.0 ms on the three longest recordings, matching `drift_ms` in `game.json`, with both tracks starting at 0 and no edit lists. No recording is longer than 23 minutes and these builds do not log trims, so the 3-hour figure is an extrapolation and audibility was not tested. Slewing is not needed; re-check with an `av-180` run.

### Start offset and encoder delay

- The AAC encoder has a priming delay of about one 1024-sample frame (21.3 ms at 48 kHz). How the MS MP4 sink handles it without edit lists is UNVERIFIED. TEST with a clap test: record a clap on screen, compare audio and video positions in an editor. If audio is consistently late by 21 to 43 ms, shift audio PTS earlier by that amount (clamped at 0).

## Mic (v1 default off)

- The MS MP4 sink supports a maximum of **one audio stream and one video stream**, fixed at creation, no `AddStreamSink`. VERIFIED (`MFCreateMPEG4MediaSink` docs). One community report of a second AAC `AddStream` failing on a sink writer is consistent with this. So a separate mic track in MP4 through MF is **not possible**. The user's note that some players only play track one is moot here.
- Options when mic is enabled: (a) mix game and mic into the single track, (b) write a second file (`.m4a` or a second MP4) for the mic and mux later, (c) fragmented MP4 or a custom muxer (large scope, not researched).
- Recommend (a) for v1.1: mic is captured as normal shared-mode WASAPI capture on the default capture endpoint (`GetMixFormat` works there, use autoconvert to 48 kHz float stereo). It gets its own timeline using the same gap and drop logic, each source writes into a ring indexed by `(ts - t0) * 48000 / 1e7`. A mixer emits fixed 20 ms blocks up to `min(now) - SAFETY`, sums in float with per-source gain (default game 1.0, mic 1.0), hard clamps once, then converts to i16. Needs microphone permission for desktop apps on Windows 10/11: UNVERIFIED from memory. A mic on a Bluetooth headset can switch the headset to a lower-quality profile: UNVERIFIED from memory.
- With a mic, game audio echoes into the mic if the user wears no headphones. Out of scope for v1.

## AAC encoding with Media Foundation

All VERIFIED against the Microsoft AAC encoder page unless noted.

- `CLSID_AACMFTEncoder` (`wmcodecdsp.h`, DLL `mfaacenc.dll`), AAC-LC only (also HE-AAC v1/v2 profile levels via `MF_MT_AAC_AUDIO_PROFILE_LEVEL_INDICATION`). Minimum Windows 7.
- **Input**: `MFAudioFormat_PCM`, **16 bits only**, 44100 or 48000 Hz, 1, 2 or 6 channels (6 needs Windows 10). Float is not an accepted input subtype on the page.
- **Output**: `MFAudioFormat_AAC`, 16 bits per sample (yes, set it), same sample rate and channels as input, `MF_MT_AUDIO_AVG_BYTES_PER_SECOND` one of **12000, 16000, 20000, 24000** bytes/s, meaning **96, 128, 160, 192 kbps** for mono and stereo. Default 12000 (96 kbps). Payload type 0 (raw) for MP4. Profile level 0x29 (AAC L2) default.
- Every input sample **must** have time and duration. A missing time gives `MF_E_NO_SAMPLE_TIMESTAMP`, a missing duration gives `MF_E_NO_SAMPLE_DURATION`, and a duration of zero succeeds on input then divides by zero in `ProcessOutput`. Always write nonzero durations.
- Each output sample is one raw AAC frame of 1024 PCM samples (21.33 ms at 48 kHz).
- The MP4 sink builds the `mp4a` sample entry itself for `MFAudioFormat_AAC`, and assumes AAC-LC if `MF_MT_USER_DATA` is absent. Do not set it, the encoder fills in `MF_MT_USER_DATA` (AudioSpecificConfig) after the output type is set. VERIFIED (MP4 sink page, AAC encoder page).
- **Auto-insertion**: the sink writer attempts to load an encoder when the input type differs from the target type (`SetInputMediaType` docs, VERIFIED). wcap does exactly this for AAC: `AddStream` with the AAC type, `SetInputMediaType` with PCM 16-bit, no manual encoder (VERIFIED from wcap source). So use PCM16 input and let the sink writer insert `CLSID_AACMFTEncoder`.
- **Float input**: do not rely on the sink writer converting float to int16 before AAC. The docs do not say it does, wcap runs its own `CLSID_CResamplerMediaObject` float to PCM16 stage before the sink writer, and a Microsoft Q&A thread reports MF inserting a resampler for PCM input on Windows 11. Converting float to i16 ourselves is trivial and removes the question. UNVERIFIED whether auto-conversion works. Recommended: `i16 = (clamp(f, -1.0, 1.0) * 32767.0).round() as i16`.
- **Bitrate**: recommend 192 kbps (`24000`), about 1.4 MB/min, negligible next to video.
- Sink writer setup order: `MFCreateSinkWriterFromURL(path, None, attrs)` (video vertical owns attrs), `AddStream(video_out)`, `AddStream(audio_out)`, `SetInputMediaType(video_in)`, `SetInputMediaType(audio_in)`, then `BeginWriting()` once for both streams. VERIFIED order (sink writer tutorial, same pattern for audio as video).
  ```rust
  let out = MFCreateMediaType()?;
  out.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Audio)?;
  out.SetGUID(&MF_MT_SUBTYPE, &MFAudioFormat_AAC)?;
  out.SetUINT32(&MF_MT_AUDIO_BITS_PER_SAMPLE, 16)?;
  out.SetUINT32(&MF_MT_AUDIO_SAMPLES_PER_SECOND, 48_000)?;
  out.SetUINT32(&MF_MT_AUDIO_NUM_CHANNELS, 2)?;
  out.SetUINT32(&MF_MT_AUDIO_AVG_BYTES_PER_SECOND, 24_000)?; // 192 kbps
  let audio_stream = writer.AddStream(&out)?;

  let inp = MFCreateMediaType()?;
  inp.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Audio)?;
  inp.SetGUID(&MF_MT_SUBTYPE, &MFAudioFormat_PCM)?;
  inp.SetUINT32(&MF_MT_AUDIO_BITS_PER_SAMPLE, 16)?;
  inp.SetUINT32(&MF_MT_AUDIO_SAMPLES_PER_SECOND, 48_000)?;
  inp.SetUINT32(&MF_MT_AUDIO_NUM_CHANNELS, 2)?;
  inp.SetUINT32(&MF_MT_AUDIO_BLOCK_ALIGNMENT, 4)?;
  inp.SetUINT32(&MF_MT_AUDIO_AVG_BYTES_PER_SECOND, 192_000)?;
  writer.SetInputMediaType(audio_stream, &inp, None)?;
  ```
  Writing a block: `MFCreateMemoryBuffer(bytes)`, `Lock`, copy, `Unlock`, `SetCurrentLength(bytes)`, `MFCreateSample`, `AddBuffer`, `SetSampleTime(pts)`, `SetSampleDuration(dur)`, `writer.WriteSample(audio_stream, &sample)`. Matches the MS tutorial's video sample code.
- `MFStartup(MF_VERSION, MFSTARTUP_FULL)` once per process (video side owns it), `MFShutdown` at exit. Windows N editions without the Media Feature Pack may lack the AAC encoder: UNVERIFIED from memory, handle `MF_E_TOPO_CODEC_NOT_FOUND` from `SetInputMediaType` with a clear error.
- Concurrency: whether `WriteSample` on different streams may be called from different threads at once is UNVERIFIED. Safest: one writer thread fed by channels from video and audio, or one mutex around the writer.
- Always `Finalize()` on stop. A crashed process leaves an MP4 with no `moov`. Fragmented MP4 (`MFCreateFMPEG4MediaSink`) exists and is the usual answer, not researched here, owned by the muxer vertical.

## Crates

| Crate | Process loopback | Notes |
|---|---|---|
| `wasapi` 0.27.0 (2026-11-06, MIT, edition 2024, Rust 1.85, `windows ^0.62.2`) | **Yes**: `AudioClient::new_application_loopback_client(process_id, include_tree)`. Present at least since 0.17.0 (docs.rs example), verified in current source | Handler, VT_BLOB, flags, event wait, `BufferInfo` with QPC timestamp and SILENT flag. Has a `record_application` example, 48 kHz float stereo, `include_tree = true`, buffer 0. Also has a device test suite. |
| `cpal` 0.18.2 (2026-08-16) | **No**. Source search of `src/host/wasapi/*` shows only a general `ActivateAudioInterfaceAsync` helper for default devices and `AUDCLNT_STREAMFLAGS_LOOPBACK` on render devices. No `PROCESS_LOOPBACK` anywhere. VERIFIED by grep | System loopback only. Callback API hides per-packet QPC timestamps and the silent flag, which this design needs. |

Recommendation: use `wasapi` for capture. It is a thin, maintained wrapper over the exact calls verified above, shares our `windows` version, and exposes the QPC timestamp and flags we need. Hand-writing would be ~150 lines of unsafe code that duplicate it. Risk: a single maintainer. Mitigation: the hand-written steps above are the fallback and are already documented.

## Pitfalls

1. **No packets during silence** (system loopback verified in wcap, process loopback unknown). Never block forever on the event. Use a 50 ms timeout as a tick.
2. **`GetMixFormat` and `GetDevicePeriod` fail** on the process loopback client. Choose the format yourself.
3. **`pu64QPCPosition` is already 100 ns**, not raw QPC. Do not convert it again. Convert video timestamps into the same unit.
4. **AAC input must be 16-bit PCM**, 44.1 or 48 kHz. Convert float yourself.
5. **Every MF audio sample needs time and nonzero duration.**
6. **MS MP4 sink: one audio stream, no edit lists.** Start audio at `t0` (pad silence). No second track for mic.
7. **Docs disagree on the minimum build** (20348 vs 20438), real Windows 10 2004 (19041) support is claimed by OBS only. TEST on the target PC and log `RtlGetVersion` at startup.
8. **The completion handler must be agile** (the `#[implement]` default is fine) **and must outlive the callback.** Keep the PROPVARIANT and params alive until activation completes.
9. **Do not skip `ReleaseBuffer`** after a successful `GetBuffer`, including for `SILENT` packets. A delay of more than one period risks lost samples (GetBuffer docs).
10. **`SetEventHandle` before `Start`** (Initialize docs). Not doing so can cause multi-second delays on release.
11. **PID changes per match.** Re-activate when the game process restarts.
12. **Do not apply `SendStreamTick`/`Discontinuity` for audio gaps** when you can write real silence.
13. **Process tree semantics**: include-tree captures the target and its children. `wasapi`'s example notes you must target the parent to get a whole app tree. League needs only the game process (UNVERIFIED, TEST).
14. **DRM**: trusted drivers may refuse loopback of protected streams (MS loopback docs). Not relevant to League, listed for completeness.

## Open tests on the Windows PC (needed before calling this done)

1. Build number of the PC and whether activation succeeds on it.
2. Does process loopback of `League of Legends.exe` contain all in-game sound, and is voice chat included?
3. In a silent moment (muted in-game), do packets with `SILENT` arrive, or nothing? Log packet count per second.
4. First-packet timestamp versus QPC-now, and device-position-based versus QPC timestamps agreement.
5. 30-minute drift: final `gap` value in the resync logic and number of resyncs.
6. Clap test for A/V offset including AAC priming delay.
7. Does the sink writer accept float input for AAC (so we can drop our converter)? Only worth it if the answer is cheap.

## Sources

- https://learn.microsoft.com/en-us/windows/win32/coreaudio/loopback-recording
- https://learn.microsoft.com/en-us/samples/microsoft/windows-classic-samples/applicationloopbackaudio-sample/
- https://github.com/microsoft/windows-classic-samples/tree/main/Samples/ApplicationLoopback
- https://learn.microsoft.com/en-us/windows/win32/api/mmdeviceapi/nf-mmdeviceapi-activateaudiointerfaceasync
- https://learn.microsoft.com/en-us/windows/win32/api/audioclientactivationparams/ns-audioclientactivationparams-audioclient_process_loopback_params
- https://learn.microsoft.com/en-us/windows/win32/api/audioclient/nf-audioclient-iaudioclient-initialize
- https://learn.microsoft.com/en-us/windows/win32/api/audioclient/nf-audioclient-iaudiocaptureclient-getbuffer
- https://learn.microsoft.com/en-us/windows/win32/api/audioclient/ne-audioclient-_audclnt_bufferflags
- https://learn.microsoft.com/en-us/windows/win32/medfound/aac-encoder
- https://learn.microsoft.com/en-us/windows/win32/medfound/mpeg-4-file-sink
- https://learn.microsoft.com/en-us/windows/win32/api/mfidl/nf-mfidl-mfcreatempeg4mediasink
- https://learn.microsoft.com/en-us/windows/win32/api/mfreadwrite/nf-mfreadwrite-imfsinkwriter-setinputmediatype
- https://learn.microsoft.com/en-us/windows/win32/api/mfreadwrite/nf-mfreadwrite-imfsinkwriter-sendstreamtick
- https://learn.microsoft.com/en-us/windows/win32/medfound/tutorial--using-the-sink-writer-to-encode-video
- https://learn.microsoft.com/en-us/windows/win32/medfound/sink-writer
- https://github.com/HEnquist/wasapi-rs (src/api.rs, Cargo.toml, examples/record_application.rs, read at master, version 0.27.0)
- https://docs.rs/crate/wasapi/latest
- https://crates.io/crates/wasapi
- https://github.com/RustAudio/cpal (src/host/wasapi/device.rs, grep at master, cpal 0.18.2)
- https://github.com/mmozeiko/wcap (wcap_audio_capture.h, wcap_encoder.h: working process loopback + AAC via sink writer, silent render stream trick, MMCSS "Pro Audio")
- https://github.com/microsoft/windows-rs (crates/libs/implement: `agile: true` default)
- https://microsoft.github.io/windows-docs-rs/doc/windows/ (0.62.2 item locations and signatures)
- https://obsproject.com/kb/application-audio-capture-guide
- https://obsproject.com/kb/league-of-legends-capture-guide
- https://github.com/bozbez/win-capture-audio
- https://gitlab.freedesktop.org/gstreamer/gst-plugins-bad/-/merge_requests/1588 (loopback produces no data without a rendering stream)
- https://learn.microsoft.com/en-us/answers/a/1263165 and https://windows-hexerror.linestarve.com/q/so50032526-imfmediasink-fails-to-add-second-audio-output-stream-on-pcs-with-amd-gpus (secondhand reports, context only)
