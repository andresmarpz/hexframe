# OBS bug history: what it means for Greyscreen

We read every OBS Studio engine commit from 2013 to 2026 and checked it against Greyscreen's code on 2026-11-22. The work it found is filed as GitHub issues; #77 lists them in priority order. The study's working files (corpus, triage rows, 1,389 failure cards, per-card Greyscreen checks, and refutation verdicts) live outside the repo.

## Decisions taken

- proptest is a dev-dependency of core and mp4 for property tests.
- The capture crate stays Windows only. Pure logic moves to core so its tests run on the Mac.
- Capture starts off the engine control thread.
- The scripted fake encoder is built behind the encoder seam of the stage-seams work, whichever lands first.
- Recording through a GPU driver reset stays out for now: a reset ends the recording cleanly, as today.
- Whether WGC can record a cloaked test window waits on Andres, because the spike runs on his PC.

## Track names

Track names come from the engine mission plan, which lives outside the repo:
- **M**: Mac work. M1 per-stage frame stamps, M2 benches and property tests, M3 a recording scorer, M5 stage seams (source, converter, encoder, packetiser, sink), M6 the output side (bounded I/O, crash fuzzing).
- **R**: research. This study is R2.
- **P**: planned PC sessions. S1 game baseline, S2 lab without the game, S3 capture matrix, S4 compatibility and soak, S5 release gate.
- **C**: compatibility (HDR, Windows builds, monitors, laptops, testers).
- **X**: experiments decided by numbers. X1 a lighter capture callback, X2 pacer timer and grid clock, X3 a dedicated encoder device, X4 a shader converter, X5 encoder depth and settings, X6 reliability (encoder probe, in-session fallback, audio reconnect).

## What we did

- **Corpus.** OBS Studio 33.0.2 has 15,839 commits since 2013-09-30. 3,732 of them touch the engine paths Greyscreen cares about: capture, audio, encoders, GPU, muxing, output lifecycle, and platform code.
- **Triage.** Haiku read all 3,732 commits one by one and marked 1,389 for deep reading.
- **Cards.** Haiku wrote one failure card per marked commit from its diff, PR, and linked issue. Each card records the symptom, trigger, environment tags, root cause class, fix, test oracle, repro level, and a one-line lesson. 98 cards turned out not relevant on reading.
- **Check.** Sonnet checked each card against Greyscreen's code. It read the code; it did not run it. Final statuses: not applicable 705, handled 525, future 110, at risk 47, has bug 2 (after the review re-judged one claim, below).
- **Adversarial pass.** A second Sonnet pass attacked all 89 has-bug and at-risk claims: has-bug 2 confirmed and 1 refuted; at-risk 32 confirmed, 39 refuted, and 15 uncertain. It also spot-checked 129 "handled" verdicts, and every cited defence held. The review then re-judged one confirmed claim as refuted: f6d635e586 (OBS moved its audio smoothing threshold to 50 ms), because Greyscreen's resync limit is 30 ms, not 300 ms as the pass read it. That leaves at-risk 31 confirmed and 40 refuted.
- **Synthesis.** One synthesis per area (audio, capture, encoder, gpu, hook, output, platform, timing): 157 failure classes, 188 proposed tests, 35 hardening items, and 54 ports.
- **This step.** I re-checked the confirmed gaps against the tree by grep (see What was not checked), mapped all 223 test and hardening items to a new issue, an existing issue, or the deferred list (appendix), and wrote the backlog.
- **OBS itself has almost no tests**: 4 cmocka unit tests, 156 lines, all on utility code. CI runs PVS-Studio and clang-analyze. The obs-*-test programs are child processes that probe encoders, not tests. None of the bugs in this corpus was found by a unit test. Users and developers found most of them, and a few commits credit Coverity Scan, clang static analysis, PVS-Studio, or ThreadSanitizer.
- **Licence.** Greyscreen is now AGPL-3.0-or-later (3c0cd3c) and OBS is GPL-2.0-or-later, so OBS code may be ported if the ported file keeps its source file's own notice. Most OBS files are GPL-2.0-or-later. Some are ISC (libobs/util/platform-windows.c, libobs/util/bmem.c, libobs/util/util_uint64.h, plugins/obs-ffmpeg/ffmpeg-mux/ffmpeg-mux.c). Some have no header (window-helpers.c, win-wasapi.cpp, window-capture.c, the encoder test children); OBS's README puts those under GPL-2.0 or later. The engine mission plan was updated for the relicense: builders may port OBS code with each file's notice. #21 (third-party notices) must list every ported OBS file.

Cards per area and their final status:

| Area | Cards | Handled | Future | At risk | Has bug | Not applicable |
|---|---|---|---|---|---|---|
| audio | 123 | 41 | 10 | 5 | 0 | 67 |
| capture | 103 | 46 | 8 | 0 | 0 | 49 |
| encoder | 335 | 117 | 35 | 15 | 0 | 168 |
| gpu | 279 | 86 | 41 | 7 | 0 | 145 |
| hook | 109 | 36 | 1 | 1 | 0 | 71 |
| output | 188 | 81 | 8 | 8 | 1 | 90 |
| platform | 109 | 43 | 0 | 5 | 1 | 60 |
| timing | 143 | 75 | 7 | 6 | 0 | 55 |
| **all** | **1389** | **525** | **110** | **47** | **2** | **705** |

## What 13 years of OBS fixes say

### Top failure classes

Counts are over the 1,291 relevant cards, by root cause class. "Crash or hang" counts cards whose symptom text mentions a crash, use after free, hang, freeze, block, or deadlock (a card can count in both). It is a keyword match, so treat it as rough.

| Rank | Failure class | Cards | Crash or hang | What OBS kept hitting | Greyscreen today |
|---|---|---|---|---|---|
| 1 | Teardown, lifetime and threading (shutdown order, use after free, races, deadlocks) | 182 | 113 | Stop paths waiting on something that never comes, objects freed while another thread uses them, stop called twice or during start | Mostly removed by Rust ownership, idempotent teardown and stop timeouts. Left: the AAC hang, the orphan thread after a start timeout, a start that blocks the engine control thread, quit without finalize (#9) |
| 2 | Driver and API misuse (encoder options, D3D calls, driver bugs) | 288 | 65 | Options an old driver refuses, wrong struct versions, unchecked returns, a probe that crashes the app | Optional properties with read-back and fallback to the next candidate. Left: the encoder check runs in the app process; caps never compared |
| 3 | Error propagation | 96 | 21 | Failures that stop work silently or show a generic message | Errors reach the UI as plain-language problems. Left: the silent writer failure (has-bug), the busy-encoder message |
| 4 | Timestamps, clocks and overflow | 109 | 10 | Signed math, jumps, drift, A/V start alignment, encoder delay | Strong audio timeline (30 ms resync) and integer clocks. Left: first-packet padding, sub-frame start offset, AAC priming, one unbounded silence block on a huge jump; drift is corrected in 30 ms steps, a residual to measure |
| 5 | Init failure handling | 73 | 29 | Half-built objects crashing in cleanup or retry | Result plus RAII everywhere; no fault tests yet |
| 6 | Config defaults reaching the encoder | 107 | 17 | Zero defaults, presets too heavy for a shared GPU, wrong units | One config constructor. Left: buffer size unit unverified, quality-versus-speed cost unmeasured |
| 7 | Colour and format | 97 | 8 | Wrong matrix or range, untagged output, HDR silently downgraded | One BT.709 limited path. Left: the export writer is untagged; HDR not started |
| 8 | Leaks and unbounded queues | 62 | 5 | Growth over hours or over many start and stop cycles | Fixed pools and bounded channels; never soaked |
| 9 | Performance | 72 | 9 | Extra copies, busy waits, priority under load | Unmeasured: pacer spin, thumbnail copy, encoder preset |
| 10 | Muxer format | 39 | 3 | Missing last sample, empty chunks, 32-bit durations, header timing | Own MP4 writer with recovery; table edge cases untested |
| 11 | Device lost and device change | 14 | 4 | TDR, unplug, default device change | TDR ends the session cleanly; no rebuild, no audio reconnect |

The rest are "other" (140 cards, mostly small one-off fixes), file I/O (11), and design (1).

### The conditions behind most failures

Environment tags on relevant cards (a card can carry several):

| Condition | Cards | Condition | Cards |
|---|---|---|---|
| NVIDIA | 162 | Multi-adapter / hybrid laptop | 46 / 29 |
| Startup | 82 | Intel | 44 |
| Codec unavailable | 69 | High GPU load | 44 |
| HDR | 68 | Old driver | 33 |
| Windows 10 / 11 / a specific build | 63 / 28 / 25 | TDR (device removed) | 31 |
| Long session | 59 | Resolution change / window resize | 22 / 14 |
| AMD | 55 | Multi-monitor | 18 |
| Driver bug | 50 | Audio device unplug / default change | 17 / 13 |
| Shutdown | 48 | Fullscreen exclusive / overlays | 12 / 11 |
| High CPU load | 48 | Low memory / low VRAM | 11 / 5 |

- GPU vendor and driver dominate: NVIDIA, AMD, Intel, old drivers, hybrid laptops, and TDRs sit on most encoder and GPU cards. NVIDIA's count is high partly because NVENC is OBS's most used encoder.
- Process edges come next: startup and shutdown.
- Then time and load: long sessions, and CPU or GPU load from the game.
- Greyscreen hits all of these in normal use. League players are mostly on NVIDIA, games last 20 to 50 minutes, and the game loads the GPU the whole time.

### Trends

- **The fix rate is not falling.** 364 relevant fixes in 2013-2016, 362 in 2017-2020, and 565 in 2021-2026. The peak is 2022 (197), the year of HDR and the new NVENC code.
- **Each new feature brings its own wave.** Config default fixes went 18, 24, 65 across the three periods; colour and format 23, 17, 57; API misuse 53, 85, 118; muxer format 7, 9, 23. HDR, AV1, the new NVENC SDK, and the native MP4 muxer each added a tail of fixes.
- **Old classes come back.** Timestamp fixes went 43, 17, 32 and use-after-free 16, 9, 28 as threading was reworked.
- **Lifecycle bugs never end.** OBS still fixed NVENC teardown order in 2026 (GPU memory gone after 10 to 20 replay restarts), the last packet of its hybrid MP4 in 2026, waits on removed audio sources in 2025, and the AAC priming offset in 2025.
- **Most of these bugs were cheap to test.** 1,024 of the 1,291 relevant cards (79 percent) could be reproduced with a unit, property, fault-injection, or fake-device test. 149 need a Windows integration test, 71 a hardware matrix, 20 a soak, and 27 are not testable. OBS had none of these tests. For Greyscreen, most protection can live in `just check` and a headless cargo test run on the PC.
- Per area: device lifecycle is audio's biggest repeat source; every new codec or pixel format in encoder first shipped a silent downgrade; shutdown ordering is the largest family in output; HDR is the long tail in GPU.

## Where Greyscreen stands

Failure classes per area. Ratings come from the area syntheses, corrected where the adversarial pass refuted the only claim behind a rating: 8 classes rated partial are shown as handled, and 1 as future (latest-frame aliasing, to revisit with the Track X2 grid clock). "Audio device lost mid-recording" stays partial: its refutation covered game audio only, and test recordings use system loopback today.

| Area | Handled | Partial | Missing | Future | Not applicable |
|---|---|---|---|---|---|
| audio | 7 | 3 | 1 | 1 | 0 |
| capture | 13 | 2 | 0 | 2 | 0 |
| encoder | 14 | 5 | 1 | 2 | 3 |
| gpu | 9 | 5 | 0 | 5 | 8 |
| hook | 11 | 0 | 1 | 1 | 0 |
| output | 13 | 2 | 4 | 2 | 0 |
| platform | 14 | 5 | 1 | 0 | 0 |
| timing | 13 | 5 | 0 | 4 | 0 |
| **all** | **94** | **27** | **8** | **17** | **11** |

**audio**

| Status | Failure classes (years in OBS, cards) |
|---|---|
| Missing | AAC encoder delay not applied at mux start (2024-2025, 3) |
| Partial | Audio device lost mid-recording, no recovery (2014-2021, 7); Worker or encoder error leaves stop path blocked (2013-2025, 7); Timestamp jump or long drift grows buffers or latency (2014-2022, 5) |
| Future | Default device changes while recording (2014-2021, 6) |
| Handled | Packet placement, device clock and GetBuffer flags (2014-2023, 10); Quiet source mistaken for stopped, or silence stops the stream (2015-2017, 5); Device format differs from encoder format (2014-2018, 4); NaN, clipping and divide by zero in audio math (2014-2024, 4); Audio glitches under CPU and GPU load (2021-2022, 4); Partial init leaves handles, COM or threads behind (2014-2022, 7); after the adversarial pass: Process loopback degrades after about 60 minutes (2021-2023, 3; not soaked yet) |

**capture**

| Status | Failure classes (years in OBS, cards) |
|---|---|
| Partial | OS call errors escape or are misreported (2020-2023, 8); GPU device lost and rebuild (2017-2021, 3) |
| Future | Monitor selection by stable id (2014-2025, 4); HDR displays (2022-2022, 5) |
| Handled | WGC teardown crashes and hangs (2020-2022, 5); Capture target closes during recording (2016-2021, 4); WGC border and cursor side effects (2019-2021, 5); Missing target lookup cost and retry (2017-2023, 4); Win32 query buffers and return values (2015, 2); Coordinate space, DPI and copy-region bounds (2014-2021, 3); Multi-GPU and hybrid laptops (2021, 4); Capture alpha channel (2015, 1); after the adversarial pass: Target minimised or hidden (2020-2021, 3); WGC availability and OS build gate (2019-2020, 3); Finding the right window (2017-2022, 2); Added frame delay and A/V offset (2017, 1); Timer resolution and pacing jitter (2014, 1) |

**encoder**

| Status | Failure classes (years in OBS, cards) |
|---|---|
| Missing | Driver probe crashes or hangs the host process (2018-2023, 9) |
| Partial | B-frames or reordered output break timestamps (2016-2026, 16); Encoder preset too heavy for a GPU shared with the game (2019-2025, 10); Hardware encoder is offered but cannot encode (2017-2024, 16); Rate-control limits not applied or in the wrong unit (2017-2024, 9); Encoder busy or refused with no actionable hint (2016-2024, 5) |
| Future | Direct NVENC backend: struct versions, config and capabilities (2019-2025, 17); HDR or 10-bit input silently encoded as 8-bit (2022-2024, 12) |
| Handled | Encoder on the wrong GPU in multi-adapter systems (2019-2024, 13); Wrong or missing codec headers and stream signalling (2014-2025, 16); Defaults, sizes and ranges reach the encoder wrong (2015-2024, 17); Encoder failure shows only a generic message (2017-2024, 15); Encode call fails, hangs or deadlocks mid-session (2015-2024, 10); Shared encoder ownership and destroy races (2015-2024, 12); Input frame reused while the encoder reads it, and queue pressure (2014-2025, 9); Half-built encoder crashes on cleanup or retry (2015-2024, 18); Changing a live hardware encoder deadlocks or stalls (2019-2025, 14); Rate-control mode writes the wrong fields (2016-2026, 13); Wrong teardown order, lost last frames, growth over repeated sessions (2015-2026, 11); Unsupported option stops the encoder from opening (2015-2024, 18); GPU texture path and its CPU fallback (2019-2024, 9); after the adversarial pass: Fallback chain between encoders (2016-2024, 11) |
| Not applicable | Cross-track timestamp and packet-flag changes (2015-2023, 5); C and FFmpeg API misuse (2016-2022, 8); Vendor option, preset and platform tables (2015-2026, 42) |

**gpu**

| Status | Failure classes (years in OBS, cards) |
|---|---|
| Partial | GPU device removed or reset mid-recording (2016-2019, 3); NV12 broken or slow on specific drivers, and probes that test the wrong path (2018-2019, 11); Support cannot tell GPUs or scheduling modes apart from the log (2015-2023, 5); Container and bitstream colour tags missing or disagreeing with the pixels (2015-2023, 6); Extra copies and conversions cost too much on integrated GPUs (2019-2022, 4) |
| Future | Shared and keyed-mutex textures across devices (planned encoder device) (2014-2019, 8); HDR, 10-bit and linear-light conversion (planned HDR capture) (2021-2024, 12); Shader compile failures, cache corruption and missing system DLLs (2014-2023, 7); Stale GPU state, constant packing and blend errors in shader draws (2014-2026, 8); Shader NV12 conversion as a fallback or faster path (2014-2019, 3) |
| Handled | Wrong GPU chosen on hybrid and multi-adapter machines (2015-2022, 9); GPU scheduling priority starved or crashed by HAGS (2019-2023, 3); Stride, plane offset and odd-size chroma errors (2014-2024, 12); Wrong colour matrix, range or default for RGB to YCbCr (2014-2022, 13); 4:2:0 chroma at the wrong sample position (2019-2022, 3); Stale pixels, alpha and blend state in reused render targets (2019-2022, 3); Cached GPU resources stale after a size or format change (2014-2022, 8); Failed allocation, partial init and teardown order (2013-2016, 10); GPU queue growth, readback timing and map/unmap leaks (2014-2018, 6) |
| Not applicable | OBS device-rebuild bookkeeping (descriptors, linked list, release functions) (2013-2026, 18); Swap chain, present model and preview (2014-2022, 13); HDR and colour-management feature work (2019-2023, 33); Media and capture source colour conversion (2014-2025, 18); Planar formats, packers and format enums (2013-2024, 21); Graphics API wrapper state and ownership (2013-2024, 18); Init, teardown and capability gates inside libobs (2014-2024, 17); GPU identity helpers and vendor lists (2019-2024, 7) |

**hook**

| Status | Failure classes (years in OBS, cards) |
|---|---|
| Missing | Modal system error dialog from removable or empty drive (2017, 2) |
| Future | Capture textures need two colour views (2020, 1) |
| Handled | Frame size taken from a reported value, not the surface (2015-2018, 3); Capture rate follows the source rate (2015, 1); Teardown twice, stale handles, late callbacks, uninitialised state (2017-2025, 8); Dependent source (audio) outlives or misses the capture target (2024, 2); Resize, minimise, zero-size and re-init corrupt capture state (2015-2023, 6); Producer or GPU device lost or recreated (2016, 1); Transient start failure latched as permanent or retried in a tight loop (2015, 4); Driver refuses an optional flag, version or feature (2017-2020, 5); Wrong capture target or too many process rights (2016-2020, 3); Non-ASCII paths, relative child paths, silent install errors (2016-2026, 3); Window geometry read in the wrong DPI context (2021, 1) |

**output**

| Status | Failure classes (years in OBS, cards) |
|---|---|
| Missing | Write failure not reported to the owner (2015-2020, 3); Timed-out start leaves a live background thread (2015, 1); Windows modal dialog on removed or empty drive (2017, 1); AAC encoder delay shifts audio against video (2025, 1) |
| Partial | Empty or invalid output folder (2017, 2); Final stats dropped when the session stops (2023, 1) |
| Future | HDR container metadata (2022, 5); Retry and reconnect state stays busy after abort (2014-2025, 3) |
| Handled | Disk full and slow disk (2015-2021, 2); Write loop exit and error handling bugs (2020-2023, 5); Stop must be idempotent and always end in a terminal state (2014-2023, 12); Shutdown ordering, deadlock and use-after-free (2014-2024, 15); Half-built pipeline on start failure (2014-2024, 11); Failure carries a human reason to the UI (2017-2020, 7); File names, extensions, overwrite and rename rules (2016-2023, 8); Recording must survive a crash or power loss (2015-2024, 5); MP4 index boxes and timing fields (2015-2024, 9); Timestamps must be monotonic and start at zero (2020-2021, 3); Clip or replay announced before the file is complete (2016-2023, 9); Configuration changed under a running pipeline (2020-2023, 2); Packet buffers leaked on error paths (2021-2022, 2) |

**platform**

| Status | Failure classes (years in OBS, cards) |
|---|---|
| Missing | Machine sleeps or screensaver starts during a recording (2015, 2) |
| Partial | Teardown order and late callbacks at shutdown (2013-2022, 9); Output truncated because stages were not drained on stop (2016, 1); Crash reports without enough data to diagnose (2015-2025, 2); Frame pacing wait wakes late or burns a core (2021, 1); Waiting on a producer that died (2025, 1) |
| Handled | Lock re-entry and calling out under a lock (2014-2026, 14); Object freed or changed while another thread uses it (2013-2023, 15); Object visible to a thread before it is fully built (2015-2022, 7); Blocking sink write or join stalls capture (2014-2023, 4); Odd sizes, row pitch and mid-session resolution change (2024-2026, 3); Wrong or zero Windows version (2015-2022, 9); COM initialisation and pairing (2015-2019, 2); Empty handle sentinel and per-session state reset (2014-2018, 3); Quoting paths and DLL search path (2015-2026, 7); Hotkey chord fires a shorter binding (2014-2021, 2); Locale changes decimal separator (2015, 1); Optional OS or driver function missing (2017-2025, 3); Thread hosting capture needs a message pump (2019, 1); Configured bitrate not applied (2014, 1) |

**timing**

| Status | Failure classes (years in OBS, cards) |
|---|---|
| Partial | Audio device clock drifts against the system clock (2014-2022, 3); Audio and video start origin mismatch (2014-2024, 11); AAC encoder priming delay not compensated (2024-2024, 1); Video thread starved under CPU or GPU load (2014-2014, 2); Frame slot or cache never recovers after lost output (2015-2024, 5) |
| Future | Timing state across an encoder restart or pipeline rebuild (2016-2025, 2); No per-stage latency attribution (2024-2024, 1); B-frame DTS offset and encoder time base quirks (2017-2023, 5); after the adversarial pass: Latest-frame sampling aliases against the tick grid (2014-2015, 2) |
| Handled | Audio timestamp noise versus real jump (2014-2016, 12); Audio timestamp before the buffer start (signed math) (2014-2022, 6); Capture API flags and late-starting audio devices (2014-2023, 4); Counters and conversions overflow after hours or days (2014-2025, 7); Video frame wait overshoot, drift and timestamp jitter (2014-2022, 7); Non-monotonic or out-of-order packet timestamps reach the muxer (2014-2025, 6); Timestamp read from the wrong moment or re-quantised by the encoder (2014-2022, 4); Queue or cache grows without bound (2014-2015, 6); Frame buffer freed or resized while another thread uses it (2014-2024, 9); State from the previous session leaks into the next (2014-2025, 7); Frame loss counted under the wrong cause (2016-2019, 5); Drop policy that loses key frames or audio (2015-2017, 3); after the adversarial pass: Source stops delivering and last frame stays (2014-2021, 4) |

### Confirmed defects, by priority

All 33 confirmed claims, grouped into the defects they describe. Priority follows the backlog:
- P1: can lose or hang a recording today, or is the harness that fix needs.
- P2: the other confirmed defects, plus the harness and tests behind OBS's biggest classes (lifecycle, init failure, timestamps, recovery, leaks).
- P3: everything else (smaller defects, regression tests on handled classes, diagnostics, ports, and measure-first items).

The backlog has 3 P1, 21 P2, and 30 P3 issues. Finish the 24 P1 and P2 issues (about 5,900 lines) before any P3 issue.

| Priority | Defect | OBS cards | Where it goes |
|---|---|---|---|
| P1 | A game video write failure stays silent until the game ends (has-bug) | 8d0edc1377, 4f87337646, 41258f8976 | #32 |
| P1 | An AAC encoder error can hang audio capture and stop | 3dbfa4919a, 39183c76b5, ea9e4ca06e | #33 |
| P1 | Tray quit skips the engine shutdown, so the recording is not finalized | 4cba9d336a | #9 (comment; relabel to bug) |
| P2 | Nothing keeps the PC awake while recording (has-bug) | d78d0072a0 | #34 |
| P2 | A start timeout leaves the capture thread running | 730132853a | #36, #35 |
| P2 | A busy hardware encoder looks like a driver problem; the real reason stays in the log | fd757d6edd, 15781073dc, 4b0777a303, e593335086 | #39, #40 |
| P2 | The encoder check runs vendor drivers in the app process | 6774bbdb83 | #37, #38 |
| P2 | Exported Discord clips carry no colour tags | 37f01b35f6, a69151e204 | #41 |
| P3 | "No disk" modal dialog; empty or relative recordings folder accepted | 8ad8e18d04, cef0689a36, 937ec4d552, 91c292d394 | #42 |
| P3 | Bitrate caps are never compared after read-back | 5e6a538931 | #43 |
| P3 | Audio can start early against video (up to about 30 ms, plus up to one AAC frame) | 9aa18d3de5, 85ca1b6918 | #44 |
| P3 | Audio drift is corrected in single 30 ms steps, not smoothly (a residual to measure) | da87f08da4 | #75 (a soak measurement) |
| P3 | AAC encoder delay is not applied (measure first) | dd89ae7432 | #73 |
| P3 | Audio trims are not counted; the last second of stats is lost at stop | 7b8793f6c3, 3869cb83bc | #45 |
| P3 | The pacer's final spin burns part of a core (measure first) | e69f051736 | #74 |
| P3 | Crash reports are one line; out-of-memory leaves no trace | e3068ed985, 1e281538a4 | #69 |
| P3 | HAGS state and PCI ids are missing from diagnostics | 0a477ba3ba, 6efe7ab1c8, 54c5ac250f | #70 |

Re-judged in review: f6d635e586 is refuted because Greyscreen's 30 ms resync limit is already tighter than OBS's new 50 ms threshold. Found in review (not among the study's claims): a capture start blocks the engine control thread for up to 10 s per attempt (#35), and a stalled control thread could stall capture through the blocking 1024-event queue (tested in #49).

The 15 uncertain claims went to measure-first work:
- B-frame reordering (e657e62c29, f27a00d266, b150e4fd39, fbefa080b0): #71, and the order guard in #57, which #76 wires in as a counter, not a fault.
- Swallowed encoder inputs (0815d5c887): #76 counts skips first and switches the slot release rule only if the PC shows skips.
- Encoder load (4bfe291520, 8806738a04) and an encoder that never outputs (94b5982216): #74.
- AAC priming (5cbf0c5e6e, adf744e6f0): #73.
- Buffer size unit (aa7e579f87, c9fd128d74): the measurement step of #43.
- NV12 driver probe (b4c35fab75): #71.
- Huge timestamp jump (dc43438057): #46 bounds the silence block now (harmless) and logs the largest gap per recording. It does not re-anchor the clock: three sibling jump claims (f79f816b72, 2a89306420, 9f1a3c3112) were refuted because WASAPI QPC timestamps are monotonic, so re-anchoring waits for that log.
- Thumbnail copy cost (8d6ed988e6): deferred to Track X1.

The refuted has-bug (041d3b4155, device unplug) does not reach game audio, which uses process loopback. It does reach test recordings, which capture the primary monitor with system loopback on the default output device. A lost device there only logs a line today; the warning test is in #51, and reconnecting stays with #5.

## The test suite

### Harness seams

What exists today:

- The engine's fake media source (succeeds by default; engine/tests/clips.rs has a `FailsAfter` wrapper that fails after a delay) and the engine test harness (start an engine, wait for a message, check a file with ffprobe).
- A free-space fake for the disk guard, and a scripted encoder fake for the encoder check cache.
- The session state machine takes time as an argument, so core tests need no fake clock.
- Headless UI QA (`just qa`) and the real-player QA tests in rune-media.
- Capture unit tests (pacer, watchdog, matching, GPU usage, H.264 parsing). Capture is Windows only (behind `cfg(windows)`), so these run only on the PC.

The area syntheses labelled several capture tests as Mac tests. They cannot run on the Mac while their code lives in capture. The backlog fixes this two ways: pure code moves to core (#24, #76, fit crop in #58, the worker thread rule in #59), and thread or COM code gets fakes that run in `cargo test -p greyscreen-capture` on the PC with no game and no window.

The seams to build, in order:

| Issue | Seam | Unlocks |
|---|---|---|
| #23 | The MP4 writer over a file that fails or stalls on demand | The writer has-bug test, backpressure tests |
| #24 | Pacer grid, stall watchdog, tick matching and the rising-tick rule in core; proptest as a dev-dependency (waits on decision 1) | Timing property tests on the Mac |
| #25 | Fault knobs on the fake media source (starting from the existing `FailsAfter`), live-session and send-wait counters, and short engine timeouts for tests | All engine lifecycle tests |
| #26 | Fake AAC encoder and fake WASAPI capture client, sine and click input | The audio pipe tests (PC) |
| #27 | Test-only named fault points that return an error or stall, and a WARP device | Device loss, setup, start timeout, frame copy and converter tests (PC) |
| #28 | A soak command that drives the engine, with memory, handle and GPU samples and pass or fail | Soak cases (S4) |
| #29 | Thin trait over the H.264 MFT; coordinate with PLAN M5 | The scripted fake |
| #30 | Scripted fake transform and a clock for the drain bound | Encoder open and run fault tests (PC) |
| #31 | A cloaked test window, if WGC captures it (needs Andres's OK: Track P assumes it cannot) | Window, teardown and soak tests without a game |

### Tests by tier

| Tier | Where it runs | Issues | Failure classes it guards |
|---|---|---|---|
| Unit and property | Mac, `just check` | #46, #56, #55, #58, #59, #57, #50, #66, #72, plus the core parts of the defect and measurement issues | Timestamps and overflow, stride and odd sizes, pacing grid, slot lifetime, stop and retry state, adapter choice, config defaults, MP4 index, crash-safe recording, B-frame reorder, file name rules |
| Fault injection and fake device (engine) | Mac, `just check` | #47, #48, #49, #32, #35, #37, the engine halves of #45 and #42 | Error propagation, shutdown ordering, a blocked control thread, per-session state, slow and full disk, partial init, clip completion, probe crash |
| Fault injection and fake device (capture) | PC, headless cargo test (no game, no window, no sound) | #33, #51, #54, #36, #52, #64, #63, #60, #61, #65, #34, #38, #39 (the session-limit measurement), #69, #41, #73 (measurement) | Worker error hangs, device lost, start timeout orphans, half-built encoders, encoder stalls, init failure, colour and range, sleep, crash diagnostics |
| Windows integration with a window | PC, the cloaked test window or a game session | #62, #67, #68 (hidden decoy, nothing on screen) | WGC teardown, target closed or minimised, window identity |
| Hardware matrix | PC GPU, then testers (#17) | #71, #43, #63 per vendor, the rows in the #17 comment | Encoder opens but cannot encode, B-frames, VBV units, NV12 driver quirks, hybrid laptops |
| Soak | PC, S4 | #53, and #75 reads its numbers | Leaks over cycles and hours, drift, process-loopback long run |
| Game session measurement | PC, S1 and S3 | #74, the flash and beep part of #73, the skip count in #76 | Pacing jitter and CPU, encoder load on a shared GPU, first encoder output, A/V offset, swallowed encoder inputs |

QA rules shape this list: no test shows a window, plays sound, uses OS input, or changes a system setting. Tests that needed one of those moved to a cloaked window, a game session, testers, or the deferred list. The removable-drive dialog test runs only on a tester or a VM, because a failing run would pop a modal dialog on Andres's PC.

### How this fits the engine mission plan

- **Track M (Mac).** The harness issues and Mac tests are M-track work. M2 already asks for property tests and fuzzing of recovery, the writer, hold-back, pacer, and clip planner; #56, #50, and #57 make that concrete. M6 (bounded I/O, crash-injection fuzzing) overlaps #32, #49, and #50. M1 builds per-stage stamps; #74 only adds pacer lateness. M5 (stage seams) must agree with #29: build the trait behind whichever encoder seam lands first.
- **Track R.** This study is R2's output, and this file is its write-up.
- **Track P (PC sessions).**
  - A lab session (no game, about 45 minutes): headless capture cargo tests, doctor deep, AAC priming measurement, encoder session-limit measurement, Windows smoke tests, and the sleep request check (needs an elevated prompt). PLAN's S2 is about 30 minutes, so run this as a longer S2 or split it in two.
  - S1 baseline (game): #74, and the flash and beep A/V check once the M3 scorer exists.
  - S3 capture matrix (game): the stall finding in #67, and the WGC teardown tests if the cloaked window does not work.
  - S4 compat and soak: #53. PLAN budgets S4 at about 2 hours; the recording that hits the 3-hour limit, and any 8-hour run, need extra PC time.
  - S5 release gate: add the headless capture cargo tests and doctor deep to `just bench-pc`.
- **Track C.** The plan has testers (#17) run `greyscreen-cli bench` and send its JSON. Include the doctor deep report (#71) in that command so testers still run one command; until bench exists, they run doctor deep. Those reports fill the AMD and Intel rows of the hardware matrix.
- **Track X.** #74 feeds X2 (pacer timer). X6 (reliability) starts with #37 and #38; its in-session encoder fallback and the WASAPI reconnect (#5) get the deferred encoder-swap test and the #5 comment. X1 (callback diet) takes the deferred thumbnail-copy measurement; X3 (encoder device) and X4 (shader converter) take the deferred shared-texture and shader tests.
- **Builder order** (at most 3 Rust builders at once): #23 and #32 first. #33 needs no harness and can go to the next PC session at once. Next come the Mac harness (#25, and #24 once decision 1 is made), #35, and the engine tests. Then the capture harness (#26, #27) and the PC tests, batched into one lab session. Finish the 24 P1 and P2 issues before any P3 issue.

## Ports from OBS, ranked by value

Every ported file keeps its source file's own notice and is listed in third-party notices (#21). Most OBS files are GPL-2.0-or-later, which is compatible with Greyscreen's AGPL-3.0-or-later through the "or later" clause. Files marked ISC below are under the ISC licence: keep their ISC notice and copyright line. Files marked "no header" have no licence header; OBS's README puts them under GPL-2.0 or later, so the ported file names its OBS source and carries that notice. The licences were checked in the OBS 33.0.2 tree; check each file's history with `git show <sha>` before copying.

| Rank | Port | OBS source | Issue | Why |
|---|---|---|---|---|
| 1 | Sleep inhibitor held only while recording | libobs/util/platform-windows.c (ISC) | #34 | Fixes a has-bug |
| 2 | Disk full as its own stop reason, reported at once | plugins/obs-ffmpeg/obs-ffmpeg-output.c | #32 | Fixes a has-bug (pattern only) |
| 3 | Encoder probe in a child process with a deadline | plugins/obs-ffmpeg/obs-amf-test, plugins/obs-qsv11/obs-qsv-test (no header) | #38 | A driver crash can no longer kill the app |
| 4 | Encoder error code to user message table | plugins/obs-nvenc | #39 | Names the next action for the user |
| 5 | Fail-critical-errors error mode at start | plugins/obs-ffmpeg/ffmpeg-mux/ffmpeg-mux.c (ISC) | #42 | One line, written fresh; no modal dialog over the game |
| 6 | One colour description for container and bitstream | plugins/obs-ffmpeg/obs-ffmpeg-output.c | #41 | Stops tag drift between paths |
| 7 | Sample-accurate audio start trim | libobs/obs-output.c | #44 | A/V start to the sample |
| 8 | Timestamp window, band constants, drift tracking | libobs/obs-source.c | #46; #75 only if the soak calls for slewing | Tuned on real devices for years |
| 9 | Crash report layout and out-of-memory marker | libobs/obs-win-crash-handler.c, libobs/util/bmem.c (ISC) | #69 | Makes crash reports usable (#2) |
| 10 | HAGS state and support level query | libobs-d3d11/d3d11-subsystem.cpp | #70 | Ties the HUD-blink question to HAGS |
| 11 | Log a repeated failure once per streak | plugins/win-capture/window-capture.c (no header) | #62 | Stops log floods; reusable for #5 |
| 12 | Top-level window search by class and executable | libobs/util/windows/window-helpers.c (no header) | #68 | Removes the decoy blind spot |
| 13 | BT.709 studio-range reference matrix | libobs/media-io/video-matrices.c | #63 | CPU oracle for colour tests |
| 14 | NV12 plane probe on the production path | libobs-d3d11/d3d11-subsystem.cpp | #71 | Pixel check instead of key-frame-only |
| 15 | First-packet B-frame delay check | plugins/obs-ffmpeg/obs-ffmpeg-nvenc.c | #57, #76 | Counts silent reorder; a fault only if doctor deep finds reordering |
| 16 | Priming samples at mux start | libobs/obs-encoder.c, plugins/obs-ffmpeg/obs-ffmpeg-mux.c | #73 | The fix if the delay is audible |
| 17 | Frames-to-nanoseconds reference math | libobs/media-io/audio-io.h, libobs/util/util_uint64.h (ISC) | #46 | Independent test oracle |
| 18 | Coarse sleep then yielding spin on QPC | libobs/util/platform-windows.c (ISC) | #74 | Only if the numbers show a cost |
| 19 | GetBuffer flag policy | plugins/win-wasapi/win-wasapi.cpp (no header) | #54 | Test oracle; Greyscreen already matches |
| 20 | One flag for MP4 box version and field width | plugins/obs-outputs/mp4-mux.c | #57 | Loud failure instead of a clamp |
| 21 | Write buffering sizes | plugins/obs-ffmpeg/obs-ffmpeg-mux.c | #53 | Only if the HDD soak stalls |
| 22 | Silent render keepalive for idle loopback | plugins/win-wasapi/win-wasapi.cpp (no header) | #65 | Only if the idle test fails |
| 23 | PCI device id skip list (MX-class) | plugins/obs-ffmpeg | #74 | Only if field data shows stalls |
| 24 | Sine test source, capture interval bound, two-session test | various | #26, #56, #48 | Ideas and constants, no code |

Later, with their features:
- Direct NVENC (#4): config table, old-driver ladder, teardown order, sequence header call, and first-DTS delay.
- System audio (#5): reconnect timing and the device notification client.
- HDR capture (Track C): HDR surface rules, refusal table, HDR SEI and mastering constants, HDR peak rule, HDR side data through remux, and the BT.2020 to 709 matrix.
- Shader converter (X4): shader cache checksum and constant packing.
- Recording through a TDR: device rebuild order waits for a design call.
- Window usable check (exists, not minimised, not cloaked; window-helpers.c): waits for a feature that needs it. A live warning on minimise would fire on every alt-tab out of fullscreen League, so #67 records a stall in health instead.
- Monitor choice: stable monitor id.
- Not needed: WGC API contract gate and Windows version cross-check (refuted, or already correct). Per-frame timing record belongs to M1.

## Keeping it fresh

Re-run on each new OBS release, incrementally:

1. **Export the new commits.** In the OBS tree run `git fetch --tags`, then run the study's export script with `<last-tag>..<new-tag>`. It uses the exact path filter of this study: libobs/media-io, the libobs video, audio, output, encoder, source and core files, libobs/graphics, libobs-d3d11, libobs-winrt, the Windows platform and threading files in libobs/util, plugins/win-capture, plugins/win-wasapi, plugins/obs-ffmpeg, plugins/obs-nvenc, plugins/obs-outputs, plugins/obs-qsv11, and the output handlers in UI/ and frontend/utility. Drop every sha already triaged.
2. **Build corpus shards** in the existing format, one block per commit: `=== <short sha> <date>`, the subject, the body, and `files: path(+added/-deleted) ...`. `git log --format='=== %h %ad%n%s%n%n%b' --date=short --numstat` gives the parts; append them to the corpus shards and their index.
3. **Triage** with Haiku, one commit at a time, into `triage/shard-NN.jsonl` (`sha, date, kind, subsystem, relevance, deep_read, summary`).
4. **Cards** with Haiku for each `deep_read` commit, from its diff, PR, and linked issue, into `cards/<sha>.json` with the same fields (`symptom, trigger, env_tags, root_cause_class, fix, oracle, repro_level, repro, port, lesson, confidence`).
5. **Check** with Sonnet against the current Greyscreen tree, into `map/batch-NNN.jsonl` (`status, evidence, test, hardening, priority`). Run the adversarial pass on every new has-bug and at-risk claim into `verify/`, and spot-check a sample of handled verdicts.
6. **Update** the final statuses, affected area syntheses, and this report. File a new issue only for a new confirmed or uncertain claim that no open issue covers; add the rest as comments.
7. **Re-check old verdicts when Greyscreen changes shape.** Handled verdicts are about today's code. When a planned feature lands (system audio, direct NVENC, HDR, monitor choice, in-session encoder fallback, shader converter, or encoder device), re-run step 5 on the 110 cards marked future for that area.

## What was not checked

- **Nothing ran.** No test, build, or recording ran on Windows or the Mac in this study. The Greyscreen check read code; it did not run it.
- **Re-checked at 97832b5 by grep, nothing more:** no error mode call, no power request or execution state call, no orderly exit; the writer thread still swallows errors after the first; the AAC queue still has the receiver clone; the export writer sets no colour attributes; no HAGS query; the audio resync limit is 300,000 Hns, which is 30 ms (the first draft of this report read it as 300 ms; the first-timestamp distrust limit is 0.5 s and the timeout safety 60 ms); the pacer still spins with a yield; the crash filter writes no dump; the empty-folder write check is unchanged. One correction: the doctor command already prints the PCI device id; the diagnostics setup line and the encoder fingerprint do not. Line numbers in the area files were not re-checked.
- **Uncertain claims:** 15, listed under Confirmed defects with where each went. They need a measurement, not a reading.
- **Refuted claims:** 40 at-risk claims (39 by the adversarial pass, 1 re-judged in review) and 1 has-bug claim were refuted; a refutation can be wrong too. Only 129 of the 524 handled verdicts were spot-checked.
- **Commits not read:** 2,343 engine-path commits were triaged out without a card. The corpus holds 165 UI and frontend commits, all touching the output handlers (UI/window-basic-main-outputs.*, frontend/utility), and they gave 36 cards. The rest of the UI and frontend, and other platforms, were not looked at. A bug fix hidden there is invisible to this study.
- **Card quality:** Haiku wrote the cards; some say the cause is unknown, and confidence is low on those. Crash-or-hang counts are keyword matches.
- **Port sources:** OBS file paths come from the digests and the OBS 33 layout; confirm each with `git show` before copying.
- **Microphone:** the brief mentions mic capture, but Greyscreen has no microphone path today (process loopback and system loopback only), so mic cards were marked not applicable. Re-check them if mic capture is added.
- **Mac labels:** the area files marked some capture tests as Mac tests; capture is Windows only. The backlog moves pure logic to core or runs those tests on the PC.
- **Estimates:** issue sizes are estimates from the area files and a reading of the touched code.
- **Review:** one adversarial review checked this report and the backlog against the code and the OBS tree. Its fixes are applied; the few it did not apply are listed in the review notes. The fixed version was not reviewed again.
- **Unknowns the backlog measures first:** whether the Media Foundation MP4 sink writes a colr box (#41), whether WGC captures a cloaked window (#31), and which error the NVIDIA encoder returns at its session limit (#39).

## Deferred

Test and hardening items that are in no issue and no comment, with the reason:

- `wgc-build-gate` (capture): Refuted as a risk: the README asks for Windows 10 2004 or newer, and an unsupported result is never retried.
- `monitor-target-resolve` (capture): Only matters when users can pick a monitor; write it with that feature.
- `hdr-format-decision` (capture): HDR capture is not started; Track C begins with detect and warn.
- `hdr-core-rules` (gpu): HDR capture is not started.
- `hdr-mp4-metadata` (gpu): HDR capture is not started.
- `hdr-monitor-luma` (gpu): HDR capture is not started.
- `hdr-format-gate` (encoder): HDR capture is not started; the refusal table must land before it ships.
- `hdr-metadata-table` (output): HDR capture is not started.
- `t-typeless-slots` (hook): Only needed with a shader converter or HDR (Track X4, Track C).
- `shader-layout-and-cache` (gpu): Only with the Track X4 shader converter.
- `shader-converter-vs-vproc` (gpu): Only with the Track X4 shader converter.
- `shared-texture-roundtrip` (gpu): Only with the Track X3 dedicated encoder device.
- `shared-texture-fault` (gpu): Only with the Track X3 dedicated encoder device.
- `session-device-lost-resumes` (gpu): Recording on after a driver reset needs a design call first (one file or two segments).
- `igpu-copy-budget` (gpu): Track X1 (callback diet) removes the thumbnail copy and mip build; measure there.
- `measure-thumbnail-copy-cost` (gpu): Same as above: Track X1 replaces this copy.
- `t-future-encoder-swap` (timing): Write it with the in-session encoder fallback (Track X6).
- `t-future-stage-stamps` (timing): Track M1 builds the per-stage stamps; its own tests cover this.
- `t-repeat-skip-aliasing` (timing): The adversarial pass rated it p3; revisit with the Track X2 grid clock.
- `t-process-target` (audio): Plays test tones out loud; add it to an S4 script on a PC Andres has freed.
- `t-aac-encode-smoke` (audio): Every recording and every encoder check already runs the AAC encoder; the first full run (#1) covers it.
- `hotkey-one-edge` (output): Needs OS-level key input, which QA never uses; the hotkey is registered with no-repeat.
- `controlled-folder-access` (output): Needs Controlled Folder Access turned on, a system setting QA never changes; check on a tester PC or a VM.
- `keymap-exact-chord` (platform): 27 keybind tests exist; check that one asserts exact chord matching before adding another.
- `settings-locale-roundtrip` (platform): Rust parsing and formatting ignore the locale; there is no C locale path.
- `keyframe-flag-not-lost` (platform): One atomic swap; low value.
- `annexb-never-panics` (encoder): The parser cases exist in the Windows-only crate; move them to mp4 with the Track M5 packetiser.
