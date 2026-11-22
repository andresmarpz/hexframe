# Soak profiles (S4 session)

`greyscreen-cli soak --profile <name>` runs a named soak through the real engine and writes the JSON report (`--out`; default `greyscreen-soak-<profile>.json`, so profiles run in a row do not overwrite each other; the report names its profile in `profile`). `greyscreen-cli soak --list-profiles` prints every profile with its setup. The profiles and their limits live in `crates/cli/src/soak/profiles.rs`. A profile cannot be mixed with the mode and limit flags; `--out`, `--dir` and `--keep` still work.

Run on the Windows PC with `greyscreen-cli.exe`. The S4 session budgets about 2 hours; the long runs need extra PC time.

## Window target

Window-driven profiles (`game-*`, `av-*`) record League: start the soak, then play live games in a session Andres has freed (Practice Tool works). The League poller skips replays and TFT, so those save nothing and the run fails with "0 recordings saved". `--window league` is the default. `--window cloaked` records the cloaked test window of `crates/capture/src/testing/cloaked_window.rs` instead of the primary monitor: the soak wraps the media source so a test recording targets the window, and opens a new window when the last one was closed. It works only with the cycle profiles (not `game-*` or `av-*`), only on Windows and only in a build with the feature: `cargo build -p greyscreen-cli --release --features cloaked-window` (it turns on capture's `fault-injection`, so this build is for tests, not for release). The window never shows: it is cloaked first, and the open fails if the cloak does not read back or the foreground window changes. The `window-*` profiles pick `--window cloaked` themselves.

## Profiles

| Profile | Run | Covers |
| --- | --- | --- |
| `smoke` | 3 test recordings of 10 s | the engine path works |
| `capture-cycles-200` | 200 cycles of 5 s | capture start and stop with frames arriving: every cycle leaves a playable file, handles, threads and memory stay flat |
| `encoder-sessions-50` | 50 cycles of 15 s | encoder session churn: memory and GPU memory flat; run once with `--d3d-debug` and read the live objects |
| `encoder-opens-1000` | 1000 cycles of 2 s | 1000 encoder opens: handles and memory flat; run once with `--d3d-debug` and read the live objects |
| `cycles-random-1000` | 1000 cycles, each cut short after a random 1 to 8 s, random 0 to 3 s gaps | start and stop at random moments; the report notes the seed |
| `clip-saves-fail-50` | 50 cycles of 8 s, each with one clip save made to fail | failed clip saves: each cycle plants a file where the clips folder goes and asks for a clip; the run fails unless every save reports a failure, and memory and handles stay flat |
| `window-closes-200` | 200 recordings of the cloaked window, each cut by closing the window after a random 2 to 6 s | window closes during recordings: no crash, no device removed, flat handles; the run fails when a recording keeps running 15 s after its window closed (the capture item's `Closed` event is not reliable, so the engine must notice by itself) |
| `window-resize-60` | 8 recordings of 70 s, the window resized every 250 ms for the first 60 s | resizes while recording: no crash, no device removed, flat handles |
| `window-minimise-20` | 8 recordings of 70 s, the window minimised and restored 20 times each (1.5 s each way) | minimise and restore while recording |
| `window-restarts-50` | 50 cycles of 5 s on the window | 50 restarts on a window target |
| `game-30` | 30 min of League | memory, GPU memory, handles and threads flat over a game |
| `game-minimised-30` | 30 min, game minimised | recording with a minimised game |
| `game-hags-30` | 30 min, HAGS on | recording with hardware-accelerated GPU scheduling |
| `game-software-30` | 30 min, software encoder | flat allocations with the software encoder; fails when the recording's `health.encoder` is not the Microsoft software encoder. Nothing forces it: run on a PC where the hardware encoder is missing or fails to open |
| `game-hdd-40` | 40 min, `--dir` on an HDD or USB stick | no drops: the run fails when a recording's `dropped_pct` is above 0 |
| `av-60` | 60 min of game loopback | A/V limits: skew and drift fail the run; numbers for #75 |
| `av-180` | up to 200 min, ends at the first saved recording | one recording that runs into the engine's 3-hour limit and finalises cleanly; fails when the video is under 10700 s, so start the game in the first 20 minutes |
| `long-8h` | 3 test recordings of 9600 s on the primary monitor | back-to-back recordings with no one at the PC; only when Andres frees the PC overnight; too few checkpoints for the leak slopes, so they are noted and not judged |

Recordings stop at the engine's 3-hour limit, so there is no single 8-hour recording.

## D3D debug layer and device removal

`--d3d-debug` (also with `--profile`) creates the capture D3D11 devices with the debug layer. When a capture session ends, after everything but the device is released (a debug-layer run waits up to 5 s for a parked hardware encoder to be released first, so a count above the device can still include its objects), the report lists the live D3D objects: `gpu.live_objects[]` in the JSON has one entry per session with a `count` and the `lines`, and the verdict notes the counts. The device itself is expected to be live. If the layer is not installed (Windows optional feature "Graphics Tools"), the run records no report and notes it. The flag has no effect off Windows.

`gpu.device_removed` counts capture sessions that ended on a lost or removed graphics device. Any removal fails the run.

## Pass limits

The default limits are 2 MB per minute for private bytes and for GPU memory, 50 handles of growth and 4 threads of growth, measured after a warm-up (60 s and 2 cycles). A run also fails on a missing or unplayable recording, an engine error or a dead engine.

The soak judges leaks, saved files, engine errors, drops (`game-hdd-40`), the encoder (`game-software-30`) and, in `av-60` and `av-180`, A/V limits: skew above 40 ms in any per-second line (`av_skew_ms` in `cycles[].diag_seconds`) fails, and so does clock drift above 1 ms per hour (the slope of the per-second `av_skew_ms` points, judged only for recordings of 10 minutes or more; `drift_ms` in `cycles[].health.audio` is a fixed offset and is not judged). Every run also fails when a saved video's dts does not rise strictly. A noise burst after minute 60 is read as growth of the cumulative `audio_trims` in the per-second lines after t = 3600 s (the engine trims audio when timestamps jump); that is a proxy, since the soak has no audio analysis. It does not judge discontinuities: `discontinuities` and `gaps` are in `cycles[].health.audio`. If `game-hdd-40` shows write stalls, batch small writes as OBS's muxer does (1 MB drain chunks, 64 KB minimum write, 256 MB cap).

## Encoder session count

`greyscreen_capture::h264::live_sessions()` counts open encoder sessions. Every run reads it after the engine stops and fails when it is above 0, and the report has it in `gpu.encoder_sessions_live`. `encoder-opens-1000` therefore covers the count back to 0 after 1000 opens.

## Results

Each profile's JSON goes in this folder or on issue #53 after its run.
