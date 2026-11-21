# Hardware H.264 encoding for greyscreen

Research date: 2026-11-20. Verified items cite a source in the list at the end. Anything unverified is marked **UNVERIFIED**.

## Summary

1. Use the hardware H.264 encoder MFT directly (option b), not the sink writer. It gives raw Annex-B packets, which fits a replay buffer with keyframe cuts, and it avoids hidden colour converters. No DLL ships.
2. Do the BGRA to NV12 conversion ourselves on the GPU with `ID3D11VideoProcessor` (BT.709 limited range). The documented input types of the Microsoft H.264 MFT are NV12, I420, IYUV, YUY2 and YV12 only. No source I found says vendor hardware MFTs accept BGRA, so assume they do not.
3. Settings: High profile, PeakConstrainedVBR (20 Mbps mean and 30 Mbps peak at 1080p60, 30 and 45 Mbps at 1440p60), GOP 120 frames (2 s), 0 B-frames, low-latency mode on, CABAC on. These come from the properties Microsoft requires of every certified hardware encoder, so they are safe on NVIDIA, AMD and Intel.
4. Risk: Chromium's authors state that current MFTs write an UNSPECIFIED colour space into the stream. We must write the BT.709 colour tag ourselves (MP4 `colr` box, or an SPS VUI patch). Also no verified AMD-specific MFT bug was found, so AMD needs real hardware testing.
5. No software fallback in v1. (Superseded for the public beta: Windows' software MFT is now the last fallback, capped at 1080p 30 fps; see `docs/architecture.md`, Encoder selection.) If `MFTEnumEx(HARDWARE)` finds nothing, fail with a clear error. Put everything behind an `Encoder` trait so a direct NVENC (and later AMF) backend slots in. HEVC and AV1 are v2 and fit the same trait.

## Comparison

| | (a) Sink writer, hardware transforms | (b) Hardware MFT direct (recommended) | (c) NVENC / AMF / QSV direct | (d) FFmpeg via `ffmpeg-next`, d3d11va |
|---|---|---|---|---|
| Quality controls | Only what the sink writer exposes. Encoder settings go through the attribute store passed to `SetInputMediaType` ("use the attribute store to configure the encoder") or via the encoder's `ICodecAPI`. Which MFT it picks is not under our control. | Full `ICodecAPI` set plus media type attributes. Limited to what the vendor MFT implements. | Full vendor feature set: CQP, CBR, VBR, target quality, multi-pass, B-frame as reference (NVENC); presets, QVBR, pre-analysis, 0-3 B-frames (AMF); VUI colour fields (AMF). | Everything the three FFmpeg encoders expose (`h264_nvenc`, `h264_amf`, `h264_qsv`). |
| Vendor coverage | All three, if each vendor ships a certified MFT. UNVERIFIED which MFT it chooses on hybrid GPUs. | Same as (a), but we choose the MFT (`MFTEnum2` with adapter LUID). | One backend per vendor. NVENC: Kepler and later. | All three. |
| Complexity | Lowest. Sink writer hides the async event loop, muxes MP4. Cannot give an in-memory packet ring. | Medium. Async event loop, own MP4 muxer (or the `muxide` / `mp4` crates), parse SPS/PPS for `avcC`. | Highest. Three SDKs, three failure modes. Only NVENC has a maintained Rust crate. | Medium API, high build cost. Cross-building FFmpeg for MSVC from macOS is non-trivial (UNVERIFIED). |
| Known bugs | Sink writer inserts a converter for RGB input (see below). Throttling and GPU selection are opaque. | NVIDIA MFT: crash report when rates are changed during encoding (driver 516.94). NVIDIA wants a context flush before the texture is passed in. Intel MFT: prefix NAL units in Quality mode. Intel: `SetInputType` after resolution change fails. | NVENC: none relevant. AMF: B-frames not on VCE 1.0, LTR and B-frames are exclusive. | FFmpeg ticket 11292: AMD AMF broken in 7.1+ (peak VBR fallback without max rate). Not an MFT issue. |
| Binary size and deps | None. | None. | NVENC: `nvEncodeAPI64.dll` is installed with the driver (UNVERIFIED for the 64-bit file name; NVIDIA docs say `nvEncodeAPI.dll`). AMF: runtime from driver. QSV: oneVPL dispatcher plus Intel runtime (`vpl-sys` loads it dynamically). | FFmpeg DLLs. Size UNVERIFIED, tens of MB is typical. |

Rust bindings for (c):

- NVENC: `nvidia-video-codec-sdk` 0.4.0 (Sep 2025, built against SDK 12.1.14, links `nvEncodeAPI.lib` at build time, D3D11 input not mentioned in the README) and `nvenc` 0.1.0 (Jan 2026). Both are small projects. Writing our own thin runtime-loaded binding is likely better.
- AMF: `shiguredo_amf` 2026.1.0 (README says Linux/Vulkan target, D3D11 not mentioned), `amffi`, `hwcodec`. None confirmed for D3D11 on Windows. UNVERIFIED maturity.
- QSV: `vpl-sys` 0.2.3 (Nov 2026, 424 downloads, dynamic loading). Very young.
- (d): `ffmpeg-next` 9.1.0 and `ffmpeg-sys-next` 9.1.1 (11 Nov 2026), `ffmpeg-the-third` 6.0.0.

FFmpeg facts verified in source: `h264_nvenc` takes `AV_PIX_FMT_D3D11` frames and maps `sw_format` NV12, BGR0 (ABGR) and 0RGB (ARGB) to NVENC buffer formats; `amfenc.c` wraps D3D11 textures with `CreateSurfaceFromDX11Native` and does not copy. QSV D3D11 input: UNVERIFIED.

## Zero-copy input and colour conversion

- NV12 D3D11 textures can be passed to a hardware MFT with no copy. Wrap with `MFCreateDXGISurfaceBuffer(IID_ID3D11Texture2D, tex, subresource, false)`, add to an `IMFSample`, call `ProcessInput`. This is exactly what a public working sample (KeloCube gist) and the FFmpeg `mfenc` D3D11 patch do. FFmpeg's patch accepts only NV12 for D3D11 frames.
- The MFT must be told about the D3D device first: `ProcessMessage(MFT_MESSAGE_SET_D3D_MANAGER, manager_ptr)`. The manager comes from `MFCreateDXGIDeviceManager` and `ResetDevice(device, token)`.
- BGRA directly into a vendor MFT: not documented anywhere I found. The Microsoft H.264 MFT lists NV12, I420, IYUV, YUY2 and YV12. NVIDIA's NVENC SDK itself accepts ARGB/ABGR (and does the conversion with CUDA internally), but that is the SDK, not the MFT. **Treat BGRA to MFT as unsupported. Enumerate `GetInputAvailableType` on the test PC to confirm.**
- Sink writer with an RGB32 input type: the docs say the sink writer "can perform some format conversions on uncompressed audio and video streams" unless `MF_READWRITE_DISABLE_CONVERTERS` is set, and `SetInputMediaType` "attempts to load an encoder that can encode from the input format". Microsoft's own tutorial feeds RGB32 to an encoder this way. The docs do not name the converter. It is almost certainly the Video Processor MFT (`CLSID_VideoProcessorMFT`), which accepts RGB32/ARGB32 and outputs NV12 and is D3D11 aware. Whether it runs on the GPU or CPU in this path is **UNVERIFIED**. Do not rely on it. Hardware MFT guidance from Microsoft also says format conversion belongs in a separate video processor MFT and that Media Foundation supplies a software one when hardware has none, which is a CPU path.
- Our conversion: `ID3D11VideoProcessor::VideoProcessorBlt` from the WGC BGRA texture to an NV12 texture. It is GPU-only. It also scales (fixed encode size, letterbox) and sets colour spaces. OBS instead converts with its own shaders and gives NVENC NV12/P010 textures with `D3D11_BIND_RENDER_TARGET` only. If a vendor's video processor rejects BGRA to NV12 (probe with `CheckVideoProcessorFormatConversion`), fall back to an HLSL pass writing the Y plane (R8_UNORM view) and UV plane (R8G8_UNORM view) of an NV12 render target. Write that shader fallback only when a test machine needs it.

## Settings by `ICodecAPI` and media type

Source of truth: Microsoft H.264 encoder page. Required for certified hardware encoders (HCK): `AVEncCommonRateControlMode`, `AVEncCommonQuality`, `AVEncCommonQualityVsSpeed`, `AVEncCommonMeanBitRate`, `AVEncCommonMaxBitRate`, `AVEncCommonBufferSize`, `AVEncMPVGOPSize`, `AVEncVideoEncodeQP`, `AVEncVideoForceKeyFrame`. Optional and tested if supported: `AVLowLatencyMode`, `AVEncVideoMaxNumRefFrame`, `AVEncH264CABACEnable`, `AVEncVideoMinQP/MaxQP`, LTR, slices. Optional and **not tested in HCK**: `AVEncMPVDefaultBPictureCount`. A certified hardware encoder passing HCK is generally used instead of the inbox one.

| Setting | Value | Notes |
|---|---|---|
| `MF_MT_SUBTYPE` out | `MFVideoFormat_H264` | |
| `MF_MT_MPEG2_PROFILE` out | `eAVEncH264VProfile_High` (100) | Docs: default is Base; High needs Windows 8+. |
| `MF_MT_MPEG2_LEVEL` | do not set | Docs recommend letting the encoder choose. |
| `MF_MT_AVG_BITRATE` out | mean bitrate | Required, must be above 0. Overridden by `AVEncCommonMeanBitRate` if both set. |
| `MF_MT_FRAME_RATE`, `MF_MT_FRAME_SIZE` | 60/1, fixed WxH | Both types. Size must be even. |
| `MF_MT_INTERLACE_MODE` | `MFVideoInterlace_Progressive` (2) | |
| `AVEncCommonRateControlMode` | `PeakConstrainedVBR` (1) | Default of the inbox MFT is UnconstrainedVBR. Set before `SetOutputType`. CBR and PeakConstrainedVBR need Windows 8+. |
| `AVEncCommonMeanBitRate` | 20 000 000 (1080p60) | |
| `AVEncCommonMaxBitRate` | 30 000 000 (1080p60) | Only applies in PeakConstrainedVBR. |
| `AVEncCommonBufferSize` | `max_bitrate / 8` (about 1 s of peak) | Docs say "size, in bytes, for CBR". Use `max_bitrate / 8`. |
| `AVEncMPVGOPSize` | 120 | 0 means encoder chooses. |
| `AVEncMPVDefaultBPictureCount` | 0 | Optional and untested property. Not needed in v1. |
| `AVLowLatencyMode` | `VARIANT_TRUE` | In the inbox MFT it selects slice vs multi-frame threading; hardware MFT meaning is vendor defined. Chromium sets it. Also set the MFT attribute `MF_LOW_LATENCY` (same GUID). |
| `AVEncH264CABACEnable` | `VARIANT_TRUE` | Optional property. Inbox default is false. Ignore failure. |
| `AVEncCommonQualityVsSpeed` | 100 | Required property, 67-100 is high complexity. Whether each vendor maps it is **UNVERIFIED**. Ignore failure. |
| `AVEncCommonQuality` and mode `Quality` | not used by default | Only applies in Quality mode. Intel's MFT is reported to emit prefix NALs in Quality mode (Chromium comment). Offer as an advanced option after testing on hardware. |
| Colour (input and output types) | `MF_MT_VIDEO_PRIMARIES`=`MFVideoPrimaries_BT709` (2), `MF_MT_TRANSFER_FUNCTION`=`MFVideoTransFunc_709` (5), `MF_MT_YUV_MATRIX`=`MFVideoTransferMatrix_BT709` (1), `MF_MT_VIDEO_NOMINAL_RANGE`=`MFNominalRange_16_235` (2) | Set them. But see pitfall 1: Chromium states MFTs write UNSPECIFIED anyway. |

Which vendor MFTs honour which property: **no primary source exists**. Verified facts only: the properties above are required for certification; Chromium sets rate control mode, temporal layers, max/mean bitrate, adaptive mode, GOP size and low latency before the media types, and treats some failures as non-fatal; Chromium forces software bitrate control on AMD/ARM H.264 only behind feature flags. So: probe each property with `IsSupported`, set it, log success, and read it back with `GetValue` on the test PC. Build a one-off "probe" binary for this (task list, step 14).

Ordering (from Chromium and Microsoft docs): codec properties first, then `SetOutputType`, then `SetInputType`. The output type must be set before the input type or `SetInputType` returns `MF_E_TRANSFORM_TYPE_NOT_SET`.

## Recommendation for v1

Option (b), direct hardware H.264 MFT, fed with NV12 textures converted on the GPU, with a greyscreen-owned MP4 writer and packet ring.

Why not the sink writer: it writes files, not packets (no in-memory replay ring), it hides which MFT is used, and its RGB path is undocumented.
Why not FFmpeg: it adds DLLs, a hard cross-build, and a second abstraction for what three MFT calls do.
Why not NVENC/AMF/QSV now: three SDKs, and we have not yet seen what the MFTs on the test PC do. The trait below keeps the door open. If the probe shows the AMD or Intel MFT misbehaves, write that vendor's direct backend first.

### Call sequence (init)

Cargo features for `windows` 0.62.2 (latest on crates.io as of this research, released 2025-10-06):

```toml
windows = { version = "0.62", features = [
  "Win32_Foundation",
  "Win32_Graphics_Direct3D",
  "Win32_Graphics_Direct3D11",
  "Win32_Graphics_Dxgi",
  "Win32_Graphics_Dxgi_Common",
  "Win32_Media_MediaFoundation",
  "Win32_System_Com",
  "Win32_System_Com_StructuredStorage",
  "Win32_System_Ole",
  "Win32_System_Variant",
  "Win32_System_Threading",
] }
```

`ICodecAPI` lives in `Win32_Media_MediaFoundation`, but `SetValue`/`GetValue` are gated on `Win32_System_Com` + `Win32_System_Ole` + `Win32_System_Variant` (checked in the crate source). `ID3D11Multithread`, `D3D11_BIND_VIDEO_ENCODER` (1024), `D3D11_CREATE_DEVICE_VIDEO_SUPPORT` and the video processor interfaces are in `Win32_Graphics_Direct3D11`. `MFSetAttributeSize`/`MFSetAttributeRatio` do not exist in windows-rs (they are C inline helpers): pack `(hi as u64) << 32 | lo as u64` and use `SetUINT64`. There is no `MFStartup` feature gate beyond `Win32_Media_MediaFoundation`. Confirm exact gating at compile time; it is cheap.

1. **COM and MF on an MTA thread.**
   ```rust
   CoInitializeEx(None, COINIT_MULTITHREADED).ok()?;
   MFStartup(MF_VERSION, MFSTARTUP_FULL)?; // MF_VERSION = 131184 in 0.62.2
   ```
   Pair with `MFShutdown()` and `CoUninitialize()` at the end. Microsoft's sink writer tutorial uses an STA; async MFT callbacks arrive on MF work-queue threads, so MTA is the safe choice. Run all MFT calls from one dedicated encoder thread.

2. **D3D11 device (shared with capture).** Flags `D3D11_CREATE_DEVICE_BGRA_SUPPORT | D3D11_CREATE_DEVICE_VIDEO_SUPPORT`. Use the adapter that drives the game display (hybrid laptops: the same adapter as WGC). Then:
   ```rust
   let mt: ID3D11Multithread = device.cast()?;
   let _ = mt.SetMultithreadProtected(true);
   ```
   Needed because WGC callback thread, the video processor and the MFT's internal threads all touch the immediate context. Microsoft's D3D11 decode doc requires this for DXVA with a device manager; for encoders it is the same practice (**UNVERIFIED** as a documented encoder requirement). The KeloCube sample works without it, so it is protection, not a known hard requirement.

3. **Device manager.**
   ```rust
   let mut token = 0u32; let mut mgr = None;
   MFCreateDXGIDeviceManager(&mut token, &mut mgr)?;
   let mgr = mgr.unwrap();
   mgr.ResetDevice(&device, token)?;
   ```

4. **Enumerate hardware encoders for our adapter.** `MFTEnum2(MFT_CATEGORY_VIDEO_ENCODER, MFT_ENUM_FLAG_HARDWARE | MFT_ENUM_FLAG_SORTANDFILTER, None, Some(&out_type), attrs, ..)` where `out_type = {MFMediaType_Video, MFVideoFormat_H264}` and `attrs` carries `MFT_ENUM_ADAPTER_LUID` with the device's LUID (`IDXGIAdapter::GetDesc().AdapterLuid`, 8-byte blob). `MFTEnumEx` with the same flags works if `MFTEnum2` is unavailable (the KeloCube sample uses `MFTEnumEx`). `MFTEnum2` adapter filtering is documented by name only; exact attribute type is **UNVERIFIED**. Free the array with `CoTaskMemFree`. Zero results: return `EncoderError::NoHardwareEncoder`.
   Log `MFT_FRIENDLY_NAME_Attribute` and `MFT_ENUM_HARDWARE_VENDOR_ID_Attribute` of the chosen activate object.

5. **Activate and unlock.**
   ```rust
   let mft: IMFTransform = activate.ActivateObject()?;
   let attrs = mft.GetAttributes()?;
   // MF_TRANSFORM_ASYNC should read 1 for a hardware MFT
   attrs.SetUINT32(&MF_TRANSFORM_ASYNC_UNLOCK, 1)?;
   attrs.SetUINT32(&MF_LOW_LATENCY, 1)?;
   ```
   Until unlocked, all `IMFTransform` methods return `MF_E_TRANSFORM_ASYNC_LOCKED` except a short list (`GetAttributes`, `GetInputAvailableType`, `GetStreamCount`, `GetStreamIDs`, `SetOutputType`, `GetOutputCurrentType` for encoders).

6. **Stream ids.** `GetStreamIDs` may return `E_NOTIMPL`; then use 0 for input and output.

7. **Give the MFT the device.**
   ```rust
   mft.ProcessMessage(MFT_MESSAGE_SET_D3D_MANAGER, mgr.as_raw() as usize)?;
   ```
   Check `MF_SA_D3D11_AWARE` on the MFT attributes first. If this call fails, the encoder cannot take textures: treat as "no zero-copy encoder" and fail (no CPU path in v1). Optionally read `MF_SA_D3D11_BINDFLAGS` from `GetInputStreamAttributes` and OR those bits into the NV12 texture bind flags.

8. **Codec properties.** `let api: ICodecAPI = mft.cast()?;` Build `VARIANT`s with `windows::core::VARIANT::from(u32)` (VT_UI4) and `VARIANT::from(true)` for the bool (confirm `From` impls at compile time). Set in this order: rate control mode, mean bitrate, max bitrate, buffer size, GOP size, low latency, CABAC, quality-vs-speed. Log each `IsSupported` and `SetValue` result. Fail only on rate control mode, mean bitrate and GOP size.
   ```rust
   let v = VARIANT::from(eAVEncCommonRateControlMode_PeakConstrainedVBR.0 as u32);
   api.SetValue(&CODECAPI_AVEncCommonRateControlMode, &v)?;
   ```

9. **Output type, then input type.** Output: major Video, subtype H264, `MF_MT_AVG_BITRATE`, frame size, frame rate 60/1, progressive, `MF_MT_MPEG2_PROFILE` = High, colour attributes. `mft.SetOutputType(out_id, &out_type, 0)?`. Then read `GetOutputCurrentType` and its `MF_MT_MPEG_SEQUENCE_HEADER` blob (Microsoft: "after the output type is set, the video encoder updates the type by adding MF_MT_MPEG_SEQUENCE_HEADER"). Input: pick via `GetInputAvailableType`, set subtype `MFVideoFormat_NV12`, frame size, frame rate, progressive, same colour attributes, `SetInputType`. If `SetInputType` fails, log the available types and fail.

10. **NV12 texture pool and video processor.**
    - 6 NV12 textures (`DXGI_FORMAT_NV12`, `D3D11_USAGE_DEFAULT`, `BindFlags = RENDER_TARGET`, plus `VIDEO_ENCODER` if the MFT's `MF_SA_D3D11_BINDFLAGS` or a failing `ProcessInput` asks for it; the Chromium code uses `VIDEO_ENCODER` for NV12 staging textures, OBS uses `RENDER_TARGET` alone for NVENC). Track reuse with `MFCreateTrackedSample` + `IMFTrackedSample::SetAllocator`, or a simple "in flight" counter released when the MFT is done (the sample is released after output arrives; **UNVERIFIED** that this is exactly when the MFT is done reading, so keep a pool of at least 6 and never write a texture that has not been returned).
    - `ID3D11VideoDevice` and `ID3D11VideoContext` from the device. `CreateVideoProcessorEnumerator` (content desc: progressive, input and output size, 60 fps), `CheckVideoProcessorFormatConversion(B8G8R8A8_UNORM, RGB_FULL_G22_NONE_P709, NV12, YCBCR_STUDIO_G22_LEFT_P709)` must return true. `CreateVideoProcessor`.
    - `VideoProcessorSetStreamColorSpace1(DXGI_COLOR_SPACE_RGB_FULL_G22_NONE_P709)` and `VideoProcessorSetOutputColorSpace1(DXGI_COLOR_SPACE_YCBCR_STUDIO_G22_LEFT_P709)`. Set source and destination rectangles for letterbox scaling (`VideoProcessorSetStreamSourceRect`, `VideoProcessorSetStreamDestRect`).
    - One `ID3D11VideoProcessorOutputView` per pool texture. For each capture frame, a `ID3D11VideoProcessorInputView` over the WGC texture (cache by texture pointer; WGC's frame pool recycles a small set).

11. **Event pump.** Implement `IMFAsyncCallback` (via `windows::core::implement`) and call `mft.cast::<IMFMediaEventGenerator>()?.BeginGetEvent(&callback, None)`. In `Invoke`, call `EndGetEvent`, push the event type into a channel read by the encoder thread, and call `BeginGetEvent` again. A blocking `GetEvent(0)` loop (as in the public sample) works too, but cannot wait on frames and events together. The sample's own comment says it assumes NeedInput and HaveOutput strictly alternate, which is not guaranteed.
    - On `METransformNeedInput`: increment `need_input`.
    - On `METransformHaveOutput`: call `ProcessOutput` with an `MFT_OUTPUT_DATA_BUFFER` whose `pSample` is null if the MFT provides samples (check `GetOutputStreamInfo().dwFlags & MFT_OUTPUT_STREAM_PROVIDES_SAMPLES`; allocate one with `MFCreateMemoryBuffer` otherwise). Handle `MF_E_TRANSFORM_STREAM_CHANGE` by re-reading the output type.
    - Copy the bytes out of the output buffer (`Lock`, copy into a `Vec<u8>`, `Unlock`). This is a copy of the compressed bitstream (about 3 MB/s) not of frames, which is within the "no CPU readback of frames" rule.

12. **Start streaming.**
    ```rust
    mft.ProcessMessage(MFT_MESSAGE_COMMAND_FLUSH, 0)?;
    mft.ProcessMessage(MFT_MESSAGE_NOTIFY_BEGIN_STREAMING, 0)?;
    mft.ProcessMessage(MFT_MESSAGE_NOTIFY_START_OF_STREAM, 0)?;
    ```
    Microsoft: the MFT sends no NeedInput event until START_OF_STREAM.

13. **Per frame.**
    1. Take the WGC texture, run `VideoProcessorBlt` into the next free NV12 texture.
    2. `context.Flush()` (Chromium flushes for NVIDIA before passing a texture to the MFT; the comment is truncated in my fetch, so the exact failure is **UNVERIFIED**; the flush is cheap).
    3. Wait until `need_input > 0`; if frames queue up (encoder behind), drop the oldest frame and count it. Never block the capture callback.
    4. Build the sample:
       ```rust
       let buf = MFCreateDXGISurfaceBuffer(&ID3D11Texture2D::IID, &nv12_tex, 0, false)?;
       // for a texture, the buffer length is not set by default: some MFTs need it
       buf.SetCurrentLength(w * h * 3 / 2)?; // Chromium: "Some encoder MFTs depend on the sample buffer having a valid current length."
       let s = MFCreateSample()?; s.AddBuffer(&buf)?;
       s.SetSampleTime(pts_100ns)?; s.SetSampleDuration(frame_dur_100ns)?;
       mft.ProcessInput(in_id, &s, 0)?; need_input -= 1;
       ```
    5. Force a keyframe on demand (e.g. clip boundaries): `api.SetValue(&CODECAPI_AVEncVideoForceKeyFrame, &VARIANT::from(1u32))` before the next `ProcessInput` (Chromium does exactly this).

14. **Verification probe (do this first on the Windows PC, before building the rest).** A 150-line binary that does steps 1-9, prints the chosen MFT name and vendor id, every `GetInputAvailableType` subtype, and for each codec property in the table: `IsSupported`, `SetValue`, `GetValue`. Run on whatever GPU the PC has. This replaces all the **UNVERIFIED** vendor-behaviour claims in this document with facts.

15. **Shutdown.** `MFT_MESSAGE_COMMAND_DRAIN`, keep servicing `HaveOutput` until `METransformDrainComplete`, then `NOTIFY_END_OF_STREAM`, `NOTIFY_END_STREAMING`, `COMMAND_FLUSH`, then `IMFShutdown::Shutdown` (async MFTs must implement it; it stops the event queue so pending `BeginGetEvent` calls end). Release in order: MFT, device manager, textures, device, `MFShutdown`.

### Packets, keyframes and muxing

- Output is Annex-B H.264 (**UNVERIFIED** for every vendor; the inbox MFT is; parse start codes to check). Detect keyframes by NAL type 5 (IDR) in the packet, with `MFSampleExtension_CleanPoint` as a hint (reliability across vendors **UNVERIFIED**). Drop NAL type 14 (prefix) units.
- The SPS/PPS come from `MF_MT_MPEG_SEQUENCE_HEADER` and are also repeated in-stream on IDR for most encoders (UNVERIFIED). Cache them and prepend to every clip start if missing.
- Packet ring buffer: store `(pts, dts, is_key, bytes)`. A clip request snaps the start back to the previous keyframe. With 2 s GOP the clip starts up to 2 s early (trim with an MP4 edit list or just accept it).
- Muxer: use a pure-Rust MP4 muxer (`muxide` 0.5.0, Oct 2026, "minimal-dependency pure-Rust MP4 muxer for recording"; `mp4` 0.14.0 is older, 2023). Both UNVERIFIED for H.264 edit lists and the `colr` box. If neither supports `colr`, write the box in a small own muxer (fragmented MP4 is about 300 lines).
- B-frames are off, so `dts == pts`. When B-frames are enabled later, the encoder sets `MFSampleExtension_DecodeTimestamp` on output samples (Microsoft doc, Windows 8+); the muxer must write composition offsets.

## Timestamps

- MF time unit is 100 ns (10 000 000 per second). Microsoft's tutorial calls `SetSampleTime(rtStart)` and `SetSampleDuration(10_000_000 / fps)`.
- From QPC: `t100 = (qpc_ticks - qpc_start) as u128 * 10_000_000 / qpc_freq as u128` (use `u128`; `qpc_ticks * 10_000_000` overflows `i64` after about 10 hours at 10 MHz). Anchor `qpc_start` to the first captured frame.
- WGC frames carry `SystemRelativeTime` (100 ns `TimeSpan`); `windows-capture` uses it relative to the first frame in 100 ns ticks for the same purpose. Prefer it over calling QPC in the callback. It shares a clock with audio only if audio uses QPC too; that is a muxing concern for the audio vertical.
- WGC only delivers a frame when the window content changes. For fixed 60 fps encode, run a 60 Hz pacer: at each tick, if no new frame arrived, re-blit the last capture texture. `pts = frame_index * 166_667` with periodic re-anchoring to QPC (if `round((now - t0) * 60)` jumps by more than one, you dropped; if equal, skip). `duration = 166_667` (rounded; carry the remainder so the average is exact: 10_000_000/60 = 166_666.67).
- Variable frame rate: the sink writer takes arbitrary timestamps; for gaps Microsoft says to call `SendStreamTick` once for every missing video frame. The hardware MFT takes timestamps as given; rate control uses `MF_MT_FRAME_RATE`, not timestamps (UNVERIFIED per vendor). A constant pacer avoids the question. With our own muxer, VFR would need per-sample durations in `stts`.

## Defaults

| Setting | 1080p60 | 1440p60 | Source |
|---|---|---|---|
| Profile | High | High | OBS recommends High for all encoders. |
| Rate control | PeakConstrainedVBR | PeakConstrainedVBR | Our choice. CQP is OBS's choice for recording but CQP is not in the MFT required set (only `Quality` mode and `EncodeQP` are, and their vendor behaviour is unverified). |
| Mean bitrate | 20 Mbps | 30 Mbps | Medal "Recommended Clip Quality Settings": H.264 1080p 15-20M, 1440p 20-30M. Top of range, because League has fast camera pans and particle effects and local disk is not a constraint. |
| Max bitrate | 30 Mbps | 45 Mbps | 1.5x mean. |
| VBV buffer | 1 s of max | 1 s of max | |
| GOP | 120 (2 s) | 120 (2 s) | OBS recommends keyframe interval 2 s for all encoders. 60 (1 s) if clip start precision matters more than about 3-5% bitrate (**UNVERIFIED** figure). |
| B-frames | 0 | 0 | OBS uses 2 (NVENC), 0 (AMF), 3 (QSV). Property is optional and untested in HCK for MFTs, and 0 keeps `dts == pts`. Revisit after the probe. |
| Reference frames | encoder default | encoder default | |
| Entropy | CABAC | CABAC | |
| Low latency | on | on | Avoids multi-frame lookahead delay. |

What others use (verified): OBS Advanced Recording guide: Constant QP/CQP/ICQ with QP 16-23, keyframe interval 2, High profile, B-frames 2/0/3, presets P5/Quality/TU4. Medal: bitrate-based, 3M-100M selectable, rate control mode **UNVERIFIED**. Outplayed: not found, **UNVERIFIED**.

## Pitfalls

1. **Colour tagging is probably lost.** Chromium: "We don't actually tell the MFT about the color space since all current MFT implementations just write UNSPECIFIED." Set the four colour attributes on both types anyway, then **verify with `ffprobe -show_streams` / `mediainfo`** on the test PC. Write `colr` (`nclx`, primaries 1, transfer 1, matrix 1, full_range 0) in the MP4. If the bitstream VUI is missing, most players guess BT.709 for HD (UNVERIFIED), but do not rely on it.
2. **Sink writer RGB path is opaque.** Not used. See above.
3. **NVIDIA:** flush the device context before `ProcessInput` (Chromium). A forum report describes a crash in `nvEncMFTH264x` when `SetRates` re-initialises the encoder during encoding (driver 516.94, no resolution). Set bitrate once; to change it, stop and restart the encoder at the next keyframe.
4. **Intel:** the Quality rate control mode emits prefix NAL units (Chromium comment). Changing the input type after a resolution change fails with `MF_E_TRANSFORM_CANNOT_CHANGE_MEDIATYPE_WHILE_PROCESSING` (forum report). Fix the encode size at start and scale in the video processor. If the capture window size changes, scale; never renegotiate.
5. **AMD:** no verified MFT-specific bug found. The public sample's comments say the AMD H.264 encoder accepts `MFT_MESSAGE_SET_D3D_MANAGER`. AMF notes: B-frames unsupported on VCE 1.0. AMD needs hardware testing before launch. **UNVERIFIED** whether AMD's MFT honours `PeakConstrainedVBR` max bitrate (FFmpeg's AMF encoder had such a bug, which is a different code path).
6. **Property order matters.** Set `AVEncCommonRateControlMode`, `AVEncMPVDefaultBPictureCount` and quality before `SetOutputType`; Microsoft says the encoder ignores later changes on Windows 7, and on Windows 8+ some can change live (quality, mean bitrate).
7. **Output type before input type**, or `MF_E_TRANSFORM_TYPE_NOT_SET`.
8. **Async unlock first**, or everything returns `MF_E_TRANSFORM_ASYNC_LOCKED`. Do not call `ProcessInput` without a matching `NeedInput` event: the MFT returns `MF_E_NOTACCEPTING`. `ProcessOutput` without `HaveOutput` returns `E_UNEXPECTED`.
9. **Buffer size units:** Microsoft says `AVEncCommonBufferSize` is in bytes for CBR. Other docs and encoders use bits. Set `max_bitrate / 8` and check the resulting stream with a VBV analyser (**UNVERIFIED** per vendor).
10. **Odd sizes:** Chromium notes the MF video encoder accelerator does not handle odd resolutions well. Encode size must be a multiple of 2, preferably of 16. 1080 is not a multiple of 16: encoders pad internally. Do not set a crop rectangle yourself.
11. **Hybrid GPUs:** the MFT must be on the same adapter as the D3D device. Use the LUID filter in step 4; otherwise a cross-adapter copy hits system memory.
12. **COM and MF:** MTA, `MFSTARTUP_FULL`, one dedicated encoder thread. Release every COM object before `MFShutdown`. A leaked `IMFSample` that holds a pool texture will deadlock the pool.
13. **Texture reuse:** the MFT reads the NV12 texture asynchronously. Never overwrite a texture before its sample has been released.
14. **Software MFT fallback: do not.** (Superseded for the public beta: it is the last fallback, capped at 1080p 30 fps with the speed preset; see `docs/architecture.md`, Encoder selection.) The inbox "H.264 Video Encoder" (`Mfh264enc.dll`) takes only system-memory NV12/I420/IYUV/YUY2/YV12, so using it means a CPU readback of every frame. This breaks the stated rule and would eat CPU while a game runs. Return `EncoderError::NoHardwareEncoder` and show the user a message. Revisit only if testers have machines without a hardware encoder.
15. **Detection:** hardware encoder exists if `MFTEnum2/MFTEnumEx(HARDWARE | SORTANDFILTER)` returns at least one activate object AND the MFT has `MF_SA_D3D11_AWARE` AND `SET_D3D_MANAGER` succeeds AND `SetInputType(NV12)` succeeds. Check all four at startup and cache the result.
16. **`MFSetAttributeSize` is missing in windows-rs.** Pack manually.

## Rust trait shape

```rust
pub struct EncoderConfig {
    pub width: u32,            // even, fixed for the session
    pub height: u32,
    pub fps_num: u32,
    pub fps_den: u32,
    pub codec: Codec,          // H264 now; Hevc, Av1 later
    pub rate: RateControl,
    pub gop_frames: u32,
    pub b_frames: u32,
    pub colour: ColourSpace,   // BT709 limited by default
}

pub enum RateControl {
    Cbr { bitrate: u32 },
    Vbr { mean: u32, peak: u32 },
    Quality { value: u8 },     // maps to QP / CQ / ICQ per backend
}

pub struct EncodedPacket {
    pub data: Vec<u8>,         // Annex-B
    pub pts_100ns: i64,
    pub dts_100ns: i64,
    pub duration_100ns: i64,
    pub is_key: bool,
}

pub struct Capabilities {
    pub vendor: Vendor,        // Nvidia | Amd | Intel | Other
    pub backend: &'static str, // "mft", "nvenc", "amf"
    pub codecs: Vec<Codec>,
    pub accepts_bgra: bool,    // NVENC/AMF true, MFT false
    pub max_bframes: u32,
}

pub trait Encoder: Send {
    fn capabilities(&self) -> &Capabilities;
    /// Submit one GPU frame. The texture is BGRA on the encoder's device.
    /// A backend that needs NV12 converts internally; callers never convert.
    fn submit(&mut self, frame: &ID3D11Texture2D, pts_100ns: i64) -> Result<(), EncoderError>;
    /// Non-blocking. Returns packets that are ready.
    fn poll(&mut self, out: &mut Vec<EncodedPacket>) -> Result<(), EncoderError>;
    fn request_keyframe(&mut self) -> Result<(), EncoderError>;
    fn parameter_sets(&self) -> &ParameterSets; // SPS/PPS for the muxer
    /// Drain remaining packets and release GPU resources.
    fn finish(self: Box<Self>, out: &mut Vec<EncodedPacket>) -> Result<(), EncoderError>;
}

pub fn probe(device: &ID3D11Device) -> Vec<Box<dyn EncoderFactory>>; // best first
pub trait EncoderFactory {
    fn name(&self) -> &'static str;
    fn create(&self, device: &ID3D11Device, cfg: &EncoderConfig)
        -> Result<Box<dyn Encoder>, EncoderError>;
}
```

Why this shape:

- `submit` takes BGRA. The MFT backend owns the video processor and NV12 pool. A future NVENC backend registers the BGRA texture directly (`NV_ENC_BUFFER_FORMAT_ARGB`/`ABGR` verified supported by the SDK and by FFmpeg's mapping) or does its own conversion. Callers do not change.
- Packets are Annex-B with explicit pts/dts so the ring buffer and muxer do not know the backend.
- `probe` is a list so the app can try NVENC direct, then MFT, per vendor, without the app knowing.
- Error type has `NoHardwareEncoder`, `UnsupportedConfig`, `DeviceLost` (call `probe` again and recreate).

## HEVC and AV1 later

Recommendation: yes, but v2, behind the same trait, as a `Codec` enum value and a `probe` filter.

- HEVC: Microsoft documents an inbox HEVC encoder MFT (software, Main profile); a certified hardware encoder takes precedence. Whether the hardware HEVC MFT also needs the Store "HEVC Video Extensions" package: **UNVERIFIED**.
- AV1: hardware AV1 encode in Windows is through D3D12 video encode on Windows 11 24H2 (WDDM 3.2) per Microsoft. An AV1 encoder MFT: **UNVERIFIED**. NVENC (Ada and later for 10-bit AV1 per SDK) and AMF have their own AV1 paths, so a direct backend is the likely route.
- Medal offers H.264, H.265 and AV1 (early access) with lower recommended bitrates (1080p: H.265 10-15M, AV1 7-10M vs H.264 15-20M). That is a ~25-50% bitrate saving for the same target quality, which matters for upload, not for local capture.
- Constraints for later: HEVC/AV1 do not play in every browser or Discord inline, so keep H.264 as default and as the export format.

## Sources

Microsoft Learn:
- https://learn.microsoft.com/en-us/windows/win32/medfound/h-264-video-encoder
- https://learn.microsoft.com/en-us/windows/win32/medfound/asynchronous-mfts
- https://learn.microsoft.com/en-us/windows/win32/medfound/hardware-mfts
- https://learn.microsoft.com/en-us/windows/win32/medfound/mf-readwrite-enable-hardware-transforms
- https://learn.microsoft.com/en-us/windows/win32/medfound/mf-readwrite-disable-converters
- https://learn.microsoft.com/en-us/windows/win32/medfound/sink-writer-attributes
- https://learn.microsoft.com/en-us/windows/win32/medfound/mf-sink-writer-d3d-manager
- https://learn.microsoft.com/en-us/windows/win32/medfound/sink-writer
- https://learn.microsoft.com/en-us/windows/win32/medfound/tutorial--using-the-sink-writer-to-encode-video
- https://learn.microsoft.com/en-us/windows/win32/api/mfreadwrite/nf-mfreadwrite-imfsinkwriter-setinputmediatype
- https://learn.microsoft.com/en-us/windows/win32/api/mfreadwrite/nf-mfreadwrite-imfsinkwriter-sendstreamtick
- https://learn.microsoft.com/en-us/windows/win32/medfound/video-processor-mft
- https://learn.microsoft.com/en-us/windows/win32/api/d3d11/nf-d3d11-id3d11videocontext-videoprocessorblt
- https://learn.microsoft.com/en-us/windows/win32/api/d3d11/ne-d3d11-d3d11_video_processor_format_support
- https://learn.microsoft.com/en-us/windows/win32/medfound/h-265---hevc-video-encoder
- https://learn.microsoft.com/en-us/windows-hardware/drivers/display/video-encoding-d3d12

Vendor and project sources:
- NVENC programming guide 13.0: https://docs.nvidia.com/video-technologies/video-codec-sdk/13.0/nvenc-video-encoder-api-prog-guide/index.html
- AMF video encode API: https://github.com/GPUOpen-LibrariesAndSDKs/AMF/blob/master/amf/doc/AMF_Video_Encode_API.md
- Chromium MF encoder: https://chromium.googlesource.com/chromium/src/+/main/media/gpu/windows/media_foundation_video_encode_accelerator_win.cc
- Working hardware MFT sample (NV12 texture, async loop): https://gist.github.com/KeloCube/0e56ba7f2c5729223483147eb35d9cc7
- FFmpeg mfenc D3D11 patch: https://ffmpeg.org/pipermail/ffmpeg-devel/2025-July/346427.html
- FFmpeg nvenc: https://github.com/FFmpeg/FFmpeg/blob/master/libavcodec/nvenc.c
- FFmpeg amfenc: https://github.com/FFmpeg/FFmpeg/blob/master/libavcodec/amfenc.c
- FFmpeg ticket 11292 (AMD AMF): https://trac.ffmpeg.org/ticket/11292
- OBS NVENC D3D11: https://github.com/obsproject/obs-studio/blob/master/plugins/obs-nvenc/nvenc-d3d11.c
- OBS Advanced Recording Settings: https://obsproject.com/kb/advanced-recording-settings-guide
- Medal recommended clip quality: https://support.medal.tv/support/solutions/articles/48001159800
- NVIDIA forum, MFT NVENC crash: https://forums.developer.nvidia.com/t/mft-nvenc-h264-hevc-encoder-crash/227798
- Intel forum, SetInputType after resolution change: https://community.intel.com/t5/Media-Intel-oneAPI-Video/How-to-dynamic-update-input-type-by-intel-H264-MFT/td-p/1181273
- windows-capture encoder (MediaTranscoder, 100 ns timestamps): https://docs.rs/windows-capture/2.0.0/src/windows_capture/encoder.rs.html
- windows crate 0.62.2 (source inspected locally for feature gates and constants): https://crates.io/crates/windows
- Crates: https://crates.io/crates/nvidia-video-codec-sdk, https://crates.io/crates/nvenc, https://crates.io/crates/shiguredo_amf, https://crates.io/crates/vpl-sys, https://crates.io/crates/ffmpeg-next, https://crates.io/crates/ffmpeg-the-third, https://crates.io/crates/muxide, https://crates.io/crates/mp4
