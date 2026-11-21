# greyscreen research: video frame capture

Researched 2026-11-20. Facts are marked **UNVERIFIED** when no primary source or code confirmed them. Nothing here was run on Windows. Code snippets were compile-checked only (`cargo check --target x86_64-pc-windows-msvc`, rustc 1.99.1, `windows` 0.62.2).

## Summary

- Use Windows.Graphics.Capture (WGC) on the League game window (`RiotWindowClass` / `League of Legends (TM) Client`). No hooks and no injection, so there is nothing for Vanguard to object to.
- Write the WGC glue by hand with `windows` 0.62.2 (about 300 lines). `windows-capture` 2.0.1 (MIT) is reference only. It is CPU-frame oriented and `scap` is stale.
- Pipeline: `CreateFreeThreaded` pool (2 buffers, BGRA8), `FrameArrived` callback, `CopyResource` into your own texture, drop the frame, then `ID3D11VideoProcessor` BGRA to NV12 (BT.709 limited) with scaling, then the encoder. All on one `ID3D11Device`.
- Pace in software: WGC only delivers on change, and `MinUpdateInterval` needs Windows 11 24H2. Skip frames before copying, and run a 60 Hz clock that re-submits the latest frame, so the output is constant frame rate.
- Main risks: the yellow border on Windows 10 (cannot be removed), exclusive fullscreen (WGC cannot see it, UNVERIFIED for League), hybrid-GPU adapter choice, and Riot's October 2026 memory-access change (it should not affect pure capture, but Riot has no stated exception for capture tools).

## Comparison

| | WGC (window or monitor item) | DXGI Desktop Duplication | Game hook (OBS Game Capture style) |
|---|---|---|---|
| Vanguard safety | No code in the game process. Medal says it "records your screen through standard Windows graphics capture" (vendor claim). | No code in the game process. Same risk class as WGC. | Injects a DLL, which Vanguard treats as hostile. Ruled out. OBS Game Capture went black for Valorant and League on 2026-07-21; OBS blamed Vanguard, and a Riot update then reportedly fixed it (win.gg report, not an official statement). OBS advised Window Capture in the meantime. |
| Granularity | Per window (`CreateForWindow`) or per monitor | Per output (monitor) only. Cannot isolate the game window, so overlays and notifications leak in. | Per process |
| Min OS | Win10 1803 (window and monitor items via interop; `CreateFreeThreaded` needs 1809) | Win8 | n/a |
| Frame delivery | Event on change, via the DWM compositor path. Always B8G8R8A8 for SDR. | `AcquireNextFrame` with timeout. Always B8G8R8A8 for SDR, plus dirty and move rects. | Copy of the game's back buffer, lowest latency |
| Latency | About one compositor frame (UNVERIFIED, no Microsoft figure found) | About one frame. Microsoft recommends `DuplicateOutput1` for improved performance. | Lowest |
| GPU cost | One `CopyResource` per kept frame, then VP blt | Same | Same |
| Exclusive fullscreen | Not captured (UNVERIFIED for League: the OBS forum reports exclusive-fullscreen apps defeat display and window capture). Borderless and windowed work. | Same limit. Desktop duplication needs DWM composition, and `DuplicateOutput` returns `DXGI_ERROR_UNSUPPORTED` for non-DWM modes. | Works (hooks Present) |
| Multi-GPU laptop | Device must be sensible for the game's GPU. OBS reports WGC display capture works across GPUs on Win10 1903+. For window capture, OBS says to run on the dGPU. UNVERIFIED for the exact failure mode. | `pDevice` must be created on the adapter that owns the output (documented, `E_INVALIDARG` otherwise). On a hybrid laptop the output usually hangs off the iGPU. | Hook runs in the game, so it uses the game's GPU |
| HDR | Pool format `R16G16B16A16_FLOAT` needed to avoid clipping (Microsoft docs). You then need tone mapping. | `DuplicateOutput1` supports FP16 | Depends on the game |
| Cursor and border | `IsCursorCaptureEnabled` (Win10 2004). `IsBorderRequired` (see below). | Cursor is separate: draw it yourself | n/a |
| Limits | Closed event when the window dies. Border on Win10. | Max 4 duplications per session, one per output per process, `ACCESS_LOST` on mode change or desktop switch | Anti-cheat |
| Verdict | Use this | Fallback only (monitor, borderless) | Rejected |

## Window versus monitor capture for League

- **Identity.** The in-game window is title `League of Legends (TM) Client`, class `RiotWindowClass`, process `League of Legends.exe`. Verified against code in several open-source projects:
  - `FFFFFFFXXXXXXX/league_record` (`src-tauri/src/recorder/window.rs`) uses exactly these three constants with `FindWindowA`.
  - `KebsCS/KBotExt` (`GameTab.h`) calls `FindWindowA("RiotWindowClass", "League of Legends (TM) Client")`.
  - `RePlays` whitelists class `riotwindowclass`.
  - Not read in Riot documentation. Riot does not publish window names.
  - The lobby client is a different window (`League of Legends`, process `LeagueClient`, UNVERIFIED class). One scoring project ranks the "(TM) Client" title above the launcher window.
- **Finding the HWND.**
  1. `FindWindowW("RiotWindowClass", "League of Legends (TM) Client")`.
  2. Check ownership with `GetWindowThreadProcessId` plus `QueryFullProcessImageNameW` ending in `League of Legends.exe`. This guards against other windows using the same class.
  3. Poll about once per second until it exists.
  4. Wait for `GetClientRect` to be larger than 1x1. `league_record` documents that a freshly created in-game window reports 1x1 for a short time under per-monitor DPI awareness.
- **Window versus monitor.** Prefer the window item. It excludes Discord toasts, Windows notifications and other monitors. It follows the window across moves and resizes. Costs: the border story, the need to handle `ContentSize` changes, and nothing for exclusive fullscreen. Fall back to a monitor item (`CreateForMonitor(MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST))`) only if window capture yields black frames. Start with the window item.
- **League window modes.** The official wiki lists Window Mode as Full Screen, Borderless or Windowed. Whether League's "Full Screen" is true exclusive or a flip-model borderless window is UNVERIFIED. Tell users to choose Borderless until it is tested.
- **Loading screen.** UNVERIFIED. I found no source on whether the loading screen is drawn by the same HWND. `league_record` polls for the window by class and title and does not mention a separate loading window. The expected design is to attach whenever the HWND exists, handle the first-frame size change, and re-attach on `Closed`.
- **Behaviour across events** (no primary source found for any of these except where noted):
  - Resize: `ContentSize` of the frame changes while the pool stays at the old size. Call `pool.Recreate(..)` with the new size, as OBS does (Microsoft docs: "Recreate is the recommended approach"). Frames with undefined padding are possible, so always copy only the `ContentSize` rect.
  - Minimize: WebRTC's WGC code notes a minimized window becomes momentarily non-capturable and frames stop. Handle by timer: keep the clock running and duplicate the last frame, or pause output.
  - Alt-tab and occlusion: UNVERIFIED. Expectation, based on how DWM composes windows: an occluded window keeps producing frames. Test on the PC.
  - Game exit: `GraphicsCaptureItem.Closed` fires. Stop the session and go back to polling.

## Exact WGC API surface (windows-rs)

- **Crate version.** `windows` 0.62.2 is the newest (`cargo search windows` on 2026-11-20 returns `windows = "0.62.2"`; crates.io lists 0.62.2 published 2025-10-06). `windows-future` 0.3.2 comes with it. `windows-core` has a 0.100.0 available that needs Rust 1.95, but `windows` 0.62.2 still pins 0.62.x. Pin `windows = "0.62.2"`.
- **Feature flags** (the set compiled in the probe; this is the whole capture plus video-processor path):

```toml
[dependencies.windows]
version = "0.62.2"
features = [
  "Foundation",
  "Graphics_Capture",
  "Graphics_DirectX",
  "Graphics_DirectX_Direct3D11",
  "Security_Authorization_AppCapabilityAccess",   # only for GraphicsCaptureAccess::RequestAccessAsync
  "Win32_Foundation",
  "Win32_Graphics_Direct3D",
  "Win32_Graphics_Direct3D11",
  "Win32_Graphics_Dxgi",
  "Win32_Graphics_Dxgi_Common",
  "Win32_Graphics_Gdi",                           # HMONITOR, CreateForMonitor
  "Win32_System_Performance",                     # QueryPerformanceCounter
  "Win32_System_WinRT_Direct3D11",                # CreateDirect3D11DeviceFromDXGIDevice, IDirect3DDxgiInterfaceAccess
  "Win32_System_WinRT_Graphics_Capture",          # IGraphicsCaptureItemInterop
  "Win32_UI_HiDpi",
  "Win32_UI_WindowsAndMessaging",
]
```

- **Session properties and minimum Windows build.** Versions are from the Microsoft Learn pages, read 2026-11-20.

| Property | Minimum | Notes |
|---|---|---|
| `GraphicsCaptureSession.IsSupported()` | 1803 (17134) | Call first |
| `IsCursorCaptureEnabled` | 2004 (19041), contract v10 | Default true. Set false and draw nothing; the game cursor is irrelevant. |
| `IsBorderRequired` | Learn says "Windows 10, version 2104 (10.0.20348)", contract v12. That build is Windows Server 2022, so on client think Win11 22000+. Win10 client support UNVERIFIED. | Disabling needs consent via `GraphicsCaptureAccess.RequestAccessAsync(Borderless)`, which needs the `graphicsCaptureWithoutBorder` manifest capability. If the user denies, the setter succeeds but is ignored. If another app requires the border for the same window, it is shown. |
| `MinUpdateInterval` | Learn lists it only under 26100 (Win11 24H2). No requirements table. | Treat as 24H2+. |
| `DirtyRegionMode` | Contract v19 (enum value 1245184 = 0x130000), 26100 | Values: `ReportOnly` 0, `ReportAndRender` 1. Not useful for v1. |
| `IncludeSecondaryWindows` | Win11 24H2 (10.0.26100), contract v19 | Default false. Secondary windows are `WS_POPUP` or `WS_EX_TOOLWINDOW` windows intersecting the main window. Not needed for League. |
| `Direct3D11CaptureFramePool.CreateFreeThreaded` | 1809 (17763), contract v7 | `FrameArrived` runs on the pool's internal worker thread, no `DispatcherQueue` needed. |

  - Always gate the newer setters at runtime. In windows-rs the setters do a `QueryInterface` to `IGraphicsCaptureSession{2..6}` and return an error on older Windows. Ignore the error (`let _ =`) or check `ApiInformation::IsPropertyPresent` first, as `windows-capture` does.
  - `RequestAccessAsync` for an unpackaged exe: OBS (unpackaged) calls `RequestAccessAsync(Borderless).get()` then `IsBorderRequired(false)`, from `libobs-winrt/winrt-capture.cpp`. Whether it prompts, succeeds silently or needs an MSIX manifest in an unpackaged app is UNVERIFIED. Treat the border as best-effort.
- **Item from an HWND.** `windows::core::factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>()?.CreateForWindow::<GraphicsCaptureItem>(hwnd)`. `CreateForMonitor` needs `Win32_Graphics_Gdi`. Both compile in the probe.
- **WinRT device from D3D11.** `CreateDirect3D11DeviceFromDXGIDevice(&dev.cast::<IDXGIDevice>()?)?.cast::<IDirect3DDevice>()?`.
- **Texture from a frame.** `frame.Surface()?.cast::<IDirect3DDxgiInterfaceAccess>()?.GetInterface::<ID3D11Texture2D>()?`.
- **`SystemRelativeTime`.** Returns a `TimeSpan`. Microsoft: "QPC time that can be used to synchronize other media elements". Units are 100 ns ticks (the `TimeSpan` unit), derived from `QueryPerformanceCounter`. Convert with `ticks * 100 ns`. Audio capture on the same clock (WASAPI QPC positions) lines up directly. Note: OBS does not use it. That the zero point equals raw QPC counts divided by frequency is UNVERIFIED beyond the Microsoft sentence; test by comparing with `QueryPerformanceCounter` in the callback.
- **Frame lifetime.** A frame is "checked out" by `TryGetNextFrame` and returned when the `Direct3D11CaptureFrame` object is dropped (windows-rs `Drop`, or call `frame.Close()`). Microsoft: do not keep references to frames or their surfaces after check-in. Microsoft does not say whether GPU work that is only queued on your device still protects the buffer after check-in. Greyscreen waits for its copy (an event query) before closing the frame. See `architecture.md`, "Capture frame lifetime".

## Frame pacing and constant frame rate

- **Behaviour.** WGC raises `FrameArrived` when the window content changes. A static loading screen can yield seconds with no frame. At display refresh (for example 144 Hz) frames can arrive faster than 60 per second.
- **Cap at 60 fps.**
  1. Primary: software skip. In the callback compare `SystemRelativeTime` with `next_deadline` (previous kept frame + 16.667 ms, less a small slack of about 2 ms). If early, drop the frame with no copy. Cost: one `TryGetNextFrame` plus a drop.
  2. Optional on Win11 24H2+: `session.SetMinUpdateInterval(TimeSpan { Duration: 166_666 })` (100 ns units, about 16.7 ms) so DWM does the skipping. Keep the software skip too, because it is the only path on 1809 to 23H2.
- **Constant frame rate output (recommended).** Run a 60 Hz output clock in its own thread (waitable timer, high-resolution). On each tick take the newest copied BGRA texture, run VP to NV12, and submit with `pts = n * 1/60 s`. If no new frame arrived, resubmit the previous BGRA texture, which duplicates the frame. The duplicate costs one VP blt on the GPU, which is cheap. Reasons to prefer CFR over VFR: simple muxing, simple audio sync, and players seek well. VFR is possible (use the `SystemRelativeTime` deltas as pts), but it needs encoder support for variable timestamps and complicates audio sync. UNVERIFIED which the encoder vertical prefers; decide with that team.
- **Slack.** Align the clock to the first frame's timestamp. Use the QPC from `SystemRelativeTime` as the shared timeline with audio.
- **Buffer count.** Use 2 (OBS uses 2 and `windows-capture` uses 1). The pool hands out a buffer until you drop the frame. Held frames are not available for new content, and when all buffers are held WGC drops new frames. Microsoft says the count "can be any integer". Do not hold a frame across a VP blt or encode. Copy and release. Cost of holding: 1 buffer per held frame. Cost of a larger pool: more GPU memory (1080p BGRA is about 8 MB per buffer) and more latency if you queue. Use 3 only if frame-arrived drops show up in tests.
- **Copy then release.** `CopyResource` of 1080p BGRA is a GPU-only copy that costs microseconds to well under a millisecond (UNVERIFIED figure, not benchmarked). It decouples WGC buffer lifetime from encoder timing, and avoids creating VP input views on WGC-owned textures, whose bind flags I could not verify. Use your own ring of 3 BGRA textures with `BIND_SHADER_RESOURCE | BIND_RENDER_TARGET`.

## Prior art and recommendation

| Project | Version / licence | Facts | Use |
|---|---|---|---|
| `windows-capture` (NiiightmareXD) | 2.0.1 (2026-08-08), MIT, depends on `windows` 0.62.2, `windows-future` 0.3.2, `rayon`, `parking_lot`, `thiserror`. No cargo features. | WGC plus a DXGI duplication API and a Media Foundation `VideoEncoder`. Uses `Create` (not `CreateFreeThreaded`) with 1 buffer and a message-loop thread, `Frame` hands out a texture and CPU buffer helpers. Settings cover cursor, border, secondary windows, min interval, dirty region (all gated by `ApiInformation`). Recreates the pool on `ContentSize` change. `Closed` posts `WM_QUIT`. | Reference only. Its device is created with BGRA support only (no video support), it pulls `rayon` and a built-in encoder you will not use, and `Frame` is CPU-oriented. |
| OBS `libobs-winrt/winrt-capture.cpp` | GPL-2.0 | `Create` with 2 buffers. `Recreate` on content size change. Rebuilds everything on device loss. Closed handler sets inactive. Switches to `R16G16B16A16_FLOAT` on HDR monitors. `CopyResource` or `CopySubresourceRegion` inside the callback. | Best design reference. Do not copy code. |
| `scap` (CapSoftware) | 0.1.0-beta.1, MIT, last push 2025-08-05 | Windows backend wraps `windows-capture` 1.5.0 with `windows` 0.58. | Reject: stale and double-wrapped. |
| `win_desktop_duplication` (rhinostream) | 0.10.11, MIT OR Apache-2.0 (crate), last push 2026-08-19 | DXGI Desktop Duplication wrapper with adapter selection and a texture reader. | Fallback reference if WGC fails. Not needed in v1. |
| `league_record` and `libobs-recorder` | open source | Records League by window class, title and process through OBS Game Capture, which hooks. | Useful for window lookup only. |

**Recommendation: hand-written `windows` code.** The surface is small (compiled probe is under 120 lines for capture plus VP), you need the raw `ID3D11Texture2D` on your own device, and you want none of the CPU readback or encoder machinery. Keep `windows-capture` open as a reference for `ApiInformation` gating and resize handling.

## Colour conversion and scaling: video processor versus compute shader

- **Choose `ID3D11VideoProcessor` for v1.** It does BGRA to NV12, scaling and matrix selection in one blt, with no shader code, and is the path hardware encoders expect (NV12 texture). A compute shader gives control over scaling filters and exact rounding but means writing and validating RGB to YUV math, plane writes to NV12 (needs `R8` and `R8G8` UAV views, which need a `TYPELESS`-compatible NV12 texture and are driver-sensitive, UNVERIFIED), and a second code path. Switch only if VP scaling quality or colour accuracy is shown to be poor.
- **Caveat.** VP scaling filter quality is driver-defined (UNVERIFIED) and the default is not guaranteed to be bicubic or Lanczos. Prefer capturing at the target resolution where possible (League can render at the output size).
- **Steps** (all compile-checked; see `probe` snippet below):
  1. Device flags: `D3D11_CREATE_DEVICE_BGRA_SUPPORT | D3D11_CREATE_DEVICE_VIDEO_SUPPORT`.
  2. `dev.cast::<ID3D11VideoDevice>()` and `ctx.cast::<ID3D11VideoContext1>()`. Use `VideoContext1` for the DXGI colour-space setters (Windows 8.1+).
  3. `CreateVideoProcessorEnumerator(&D3D11_VIDEO_PROCESSOR_CONTENT_DESC { InputFrameFormat: PROGRESSIVE, InputWidth/Height = capture size, OutputWidth/Height = target size, Usage: PLAYBACK_NORMAL, .. })`.
  4. `CreateVideoProcessor(&enumerator, 0)`.
  5. Optional gate: `enumerator.cast::<ID3D11VideoProcessorEnumerator1>()?.CheckVideoProcessorFormatConversion(B8G8R8A8_UNORM, RGB_FULL_G22_NONE_P709, NV12, YCBCR_STUDIO_G22_LEFT_P709)`.
  6. Colour space (BT.709, limited range output):
     - `VideoProcessorSetStreamColorSpace1(&vp, 0, DXGI_COLOR_SPACE_RGB_FULL_G22_NONE_P709)` (enum value 0).
     - `VideoProcessorSetOutputColorSpace1(&vp, DXGI_COLOR_SPACE_YCBCR_STUDIO_G22_LEFT_P709)` (enum value 8 in windows-rs 0.62.2).
     - Legacy equivalent through `ID3D11VideoContext::VideoProcessorSetOutputColorSpace`, a bitfield struct: `Usage` bit 0, `RGB_Range` bit 1, `YCbCr_Matrix` bit 2 (1 = BT.709), `YCbCr_xvYCC` bit 3, `Nominal_Range` bits 4-5 (1 = 16-235, 2 = 0-255). Output limited BT.709 would be `(1 << 2) | (1 << 4)` = `0x14`. Do not use the legacy path unless the `Color_Space1` path fails; the default matrix is BT.601, a common silent error.
  7. `VideoProcessorSetStreamFrameFormat(.., PROGRESSIVE)`, `SetStreamSourceRect(.., true, &crop)` (use the frame `ContentSize`), `SetStreamDestRect(.., true, &dst)`, `SetOutputTargetRect(true, &dst)`.
  8. Allocate the NV12 output texture: `Format: DXGI_FORMAT_NV12`, `BindFlags: RENDER_TARGET | SHADER_RESOURCE`, `Usage: DEFAULT`. Exact bind flags the chosen encoder needs are UNVERIFIED (check with the encoder vertical).
  9. `CreateVideoProcessorInputView(&bgra_tex, &enumerator, &D3D11_VIDEO_PROCESSOR_INPUT_VIEW_DESC { ViewDimension: D3D11_VPIV_DIMENSION_TEXTURE2D, .. }, Some(&mut iv))`, `CreateVideoProcessorOutputView(&nv12_tex, &enumerator, &D3D11_VIDEO_PROCESSOR_OUTPUT_VIEW_DESC { ViewDimension: D3D11_VPOV_DIMENSION_TEXTURE2D, .. }, Some(&mut ov))`. Create views once per ring texture and cache them.
  10. `VideoProcessorBlt(&vp, &ov, 0, &[D3D11_VIDEO_PROCESSOR_STREAM { Enable: true.into(), pInputSurface: ManuallyDrop::new(iv), ..Default::default() }])`. `pInputSurface` is a `ManuallyDrop` field, so call `ManuallyDrop::drop` on it after the blt or you leak a reference per frame.
- **HDR.** SDR only in v1. If the monitor is in HDR mode, set `Win+Alt+B` guidance in the UI, or request `R16G16B16A16_FLOAT` and tone-map later (Microsoft: pixel format may not be B8G8R8A8 with HD Color).

## Recommendation for v1

Capture League with WGC on the game window, hand-written against `windows` 0.62.2. Exact calls, in order (compile-checked in a throwaway probe; the code below is the contract).

1. Process start: `SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2)`. Check `GraphicsCaptureSession::IsSupported()`.
2. Pick the adapter. Use `IDXGIFactory6::EnumAdapterByGpuPreference(0, DXGI_GPU_PREFERENCE_HIGH_PERFORMANCE)` (needs Win10 1803; this call and its reliability on hybrid laptops are UNVERIFIED here, I did not compile it). Allow a settings override. Let the encoder choose the same adapter.
3. Create the device once, on that adapter, with `D3D_DRIVER_TYPE_UNKNOWN`:

```rust
D3D11CreateDevice(&adapter, D3D_DRIVER_TYPE_UNKNOWN, HMODULE::default(),
    D3D11_CREATE_DEVICE_BGRA_SUPPORT | D3D11_CREATE_DEVICE_VIDEO_SUPPORT,
    Some(&[D3D_FEATURE_LEVEL_11_1]), D3D11_SDK_VERSION,
    Some(&mut dev), None, Some(&mut ctx))?;
ctx.cast::<ID3D11Multithread>()?.SetMultithreadProtected(true);   // callbacks run on a worker thread
```

4. Wrap it: `let winrt_dev: IDirect3DDevice = CreateDirect3D11DeviceFromDXGIDevice(&dev.cast::<IDXGIDevice>()?)?.cast()?;`
5. Find the window (loop, 1 Hz): `FindWindowW(w!("RiotWindowClass"), w!("League of Legends (TM) Client"))`, verify the process image name, wait for client rect above 1x1.
6. Item: `factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>()?.CreateForWindow::<GraphicsCaptureItem>(hwnd)?`. Register `item.Closed(...)` first.
7. Pool: `Direct3D11CaptureFramePool::CreateFreeThreaded(&winrt_dev, DirectXPixelFormat::B8G8R8A8UIntNormalized, 2, item.Size()?)?`.
8. `pool.FrameArrived(&TypedEventHandler::new(...))`. In the callback:
   1. `let frame = pool.TryGetNextFrame()?`.
   2. Compare `frame.ContentSize()` to `last_size`. If changed, `pool.Recreate(&winrt_dev, format, 2, new_size)`, drop the frame and return.
   3. Skip if `SystemRelativeTime` is earlier than `next_deadline` (drop with no copy).
   4. `let tex: ID3D11Texture2D = frame.Surface()?.cast::<IDirect3DDxgiInterfaceAccess>()?.GetInterface()?`.
   5. `ctx.CopySubresourceRegion(&ring[i], 0, 0, 0, 0, &tex, 0, Some(&box_of_content_size))` into the next ring texture, then drop `frame`.
   6. Store `(i, SystemRelativeTime)` as the latest frame (atomic or mutex).
9. Session: `let session = pool.CreateCaptureSession(&item)?;` then, ignoring errors: `SetIsCursorCaptureEnabled(false)`; on 24H2+ `SetMinUpdateInterval(TimeSpan { Duration: 166_666 })`; `SetIsBorderRequired(false)` after `GraphicsCaptureAccess::RequestAccessAsync(GraphicsCaptureAccessKind::Borderless)?.join()?` returns `AppCapabilityAccessStatus::Allowed`. Then `session.StartCapture()?`.
10. 60 Hz output clock thread: each tick, take the latest ring slot (or the previous one), run the VP steps above to an NV12 output texture, pass it to the encoder with `pts = tick_index / 60 s`. Record the QPC base for audio alignment.
11. Teardown (in order): remove the `FrameArrived` and `Closed` tokens, `session.Close()`, `pool.Close()`, drop views and textures. On `Closed`, go to step 5.
12. Device loss: on `DXGI_ERROR_DEVICE_REMOVED` or `DEVICE_RESET` from any call (check `dev.GetDeviceRemovedReason()`), tear everything down and rebuild from step 2 (OBS does this: release session, pool, item, then rebuild all), and tell the encoder to rebuild too.

Reference snippets (compile-checked):

```rust
let interop = factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>()?;
let item: GraphicsCaptureItem = unsafe { interop.CreateForWindow(hwnd)? };
let pool = Direct3D11CaptureFramePool::CreateFreeThreaded(
    &winrt_dev, DirectXPixelFormat::B8G8R8A8UIntNormalized, 2, item.Size()?)?;
pool.FrameArrived(&TypedEventHandler::new(move |p: windows::core::Ref<Direct3D11CaptureFramePool>, _| {
    let frame = p.ok()?.TryGetNextFrame()?;
    let _t = frame.SystemRelativeTime()?;                 // TimeSpan, 100 ns ticks, QPC based
    let access: IDirect3DDxgiInterfaceAccess = frame.Surface()?.cast()?;
    let _tex: ID3D11Texture2D = unsafe { access.GetInterface()? };
    Ok(())
}))?;
let session = pool.CreateCaptureSession(&item)?;
session.StartCapture()?;
```

## Windowed mode cropping

Not run on Windows yet (UNVERIFIED). The Mac only compile-checks it.

- WGC on a windowed game captures the whole window, including the title bar and borders. Borderless and fullscreen have no chrome.
- Each frame, `client_area` reads the window frame bounds (`DwmGetWindowAttribute` with `DWMWA_EXTENDED_FRAME_BOUNDS`), the client origin (`ClientToScreen(hwnd, 0,0)`) and the client size (`GetClientRect`). `core::crop::client_crop` turns them into a crop inside the frame.
- Offset and size are scaled from screen pixels to frame pixels by frame `ContentSize` divided by the frame-bounds size, which covers DPI scaling.
- The crop is the video processor source rect. The existing letterbox then fits the client area into the fixed output size.
- A zero client size (minimized), an empty frame bounds, or a client rect outside the frame (more than 1 px) falls back to the full frame. Width and height round down to even.
- The result is cached. It refreshes when the frame size changes and at most every 500 ms otherwise. A window move does not change the crop. A resize does. The crop is logged at start and on every change.
- The output size at session start comes from the first frame's client area, then `encode_size`.
- A mid-game resize keeps the output size fixed. The new client area is letterboxed into it, so the recording does not change resolution.

## Pitfalls

- **Yellow border on Windows 10.** The system draws it around every captured item, and Microsoft's own page confirms it for each active session. `IsBorderRequired` is Win11-era and needs consent; Learn lists 20348. On Win10 expect the border. OBS users report the border can return when an exclusive-fullscreen game starts. Document it for users, do not fight it.
- **Exclusive fullscreen.** WGC cannot see true exclusive fullscreen (UNVERIFIED for League). Detect it: if no frame arrives within about 2 s of `StartCapture` while the window is foreground, show "switch League to Borderless".
- **Hybrid GPUs.** Create the device on the adapter that renders the game. OBS says on hybrid laptops Window and Game Capture go black if the capturing app runs on the other adapter, and offers a "multi-adapter compatibility" CPU-copy mode. If frames come back black, offer the Windows Graphics Settings "High performance" binding for `greyscreen.exe`. The failure mode for a mismatched WGC device is UNVERIFIED.
- **Item size versus frame size.** The pool surface is always the size you gave it. Content larger than the surface is clipped, smaller leaves undefined data. Always copy only `ContentSize`.
- **Resize storms.** Resizing fires many size changes. Recreate once per distinct size and drop the stale frame (OBS and `windows-capture` both do).
- **Frame holding.** Never hold a `Direct3D11CaptureFrame` past the callback. With 2 buffers, one held frame halves capacity.
- **DPI scaling.** Without per-monitor DPI awareness, Windows reports virtualised window sizes, and `GetClientRect` can be wrong. Set per-monitor v2 at process start. `league_record` notes that League's window reports 1x1 initially with this mode.
- **Capture item closed on game exit.** `Closed` is raised on a WinRT thread. Do not tear down inside the handler: set an atomic flag and let the supervisor thread do it.
- **Device-lost.** `Recreate` discards all frames, so process pending frames first (Microsoft docs). A full rebuild is needed after device removal.
- **Pixel format on HDR.** With Windows HD Color on, SDR `B8G8R8A8` may look washed out (Microsoft). OBS switches to FP16 for HDR monitors.
- **VP input view on WGC textures.** Bind flags unknown, hence the copy to your own texture.
- **Colour matrix default.** Video processor output defaults to BT.601 in the legacy struct. Always set the colour spaces explicitly.
- **Vanguard.**
  - Riot's Vanguard FAQ and its third-party-application policy do not mention recording or capture tools (both pages read, no matches).
  - Riot announced (support article published 2026-09-29) that from 2026-10-06 it will block game memory access by default for unknown third-party apps, with an April 2027 grace period for known tools. The article names no recording exception and says "many third party tools do not pilfer game memory". WGC does not read game memory, so it should be unaffected. Treat as UNVERIFIED until tested on the PC with Vanguard running.
  - Outplayed (Overwolf) and Medal both list League of Legends as supported. Those are vendor claims. Medal says it uses "standard Windows graphics capture". I found no Riot statement approving either tool, and nothing specific on Insights (not searched in a way that returned usable results).
  - Hooking is out. The 2026-07-21 OBS Game Capture breakage on Valorant and League shows hooks are fragile against Vanguard even for a well-known tool.
- **Test on the PC first (cheap, decisive).** Run Medal-style window capture on League in Borderless: does it produce frames at the loading screen, in game, on minimize and restore, on alt-tab, and with a second window covering it? This settles all the UNVERIFIED behaviour items above.

## Sources

- https://learn.microsoft.com/en-us/windows/apps/develop/media-authoring-processing/screen-capture
- https://learn.microsoft.com/en-us/uwp/api/windows.graphics.capture.graphicscapturesession
- https://learn.microsoft.com/en-us/uwp/api/windows.graphics.capture.graphicscapturesession.isborderrequired
- https://learn.microsoft.com/en-us/uwp/api/windows.graphics.capture.graphicscapturesession.iscursorcaptureenabled
- https://learn.microsoft.com/en-us/uwp/api/windows.graphics.capture.graphicscapturesession.minupdateinterval
- https://learn.microsoft.com/en-us/uwp/api/windows.graphics.capture.graphicscapturesession.dirtyregionmode
- https://learn.microsoft.com/en-us/uwp/api/windows.graphics.capture.graphicscapturesession.includesecondarywindows
- https://learn.microsoft.com/en-us/uwp/api/windows.graphics.capture.graphicscapturedirtyregionmode
- https://learn.microsoft.com/en-us/uwp/api/windows.graphics.capture.graphicscaptureaccess.requestaccessasync
- https://learn.microsoft.com/en-us/uwp/api/windows.graphics.capture.direct3d11captureframepool.createfreethreaded
- https://learn.microsoft.com/en-us/uwp/api/windows.graphics.capture.direct3d11captureframepool.create
- https://learn.microsoft.com/en-us/windows/win32/direct3ddxgi/desktop-dup-api
- https://learn.microsoft.com/en-us/windows/win32/api/dxgi1_2/nf-dxgi1_2-idxgioutput1-duplicateoutput
- https://learn.microsoft.com/en-us/windows/win32/api/d3d11/ns-d3d11-d3d11_video_processor_color_space
- https://learn.microsoft.com/en-us/windows/win32/api/d3d11/nf-d3d11-id3d11videocontext-videoprocessorblt
- https://crates.io/crates/windows and `cargo search windows` (0.62.2), windows-rs 0.62.2 source in the local cargo registry (feature gates, signatures)
- https://github.com/microsoft/windows-rs/releases
- https://github.com/NiiightmareXD/windows-capture (and `src/graphics_capture_api.rs`, `src/d3d11.rs`, `Cargo.toml`)
- https://crates.io/crates/windows-capture
- https://github.com/obsproject/obs-studio/blob/master/libobs-winrt/winrt-capture.cpp
- https://github.com/CapSoftware/scap
- https://github.com/rhinostream/win_desktop_duplication
- https://github.com/FFFFFFFXXXXXXX/league_record (window.rs)
- https://github.com/KebsCS/KBotExt (GameTab.h)
- https://github.com/robmikh/Win32CaptureSample (reference sample, not read in detail)
- https://webrtc.googlesource.com/src/+/main/modules/desktop_capture/win/wgc_capture_session.cc (minimized window behaviour)
- https://wiki.leagueoflegends.com/en-us/Settings?oldid=3766659
- https://support.riotgames.com/en-us/riot/performance/game-memory-access-removed-for-third-party-apps
- https://support.riotgames.com/league-of-legends/performance/riot-vanguard-faq-league-of-legends
- https://support.riotgames.com/en-us/riot/events/third-party-applications
- https://win.gg/obs-game-capture-not-working-valorant-league-of-legends-fix/
- https://medal.tv/developer/valorant
- https://obsproject.com/wiki/Laptop-Troubleshooting
- https://obsproject.com/forum/threads/yellow-frame-on-a-captured-window.118019/
- https://obsproject.com/forum/threads/when-i-am-in-captured-window-i-see-yellow-lines-on-screen.126951/post-472193
