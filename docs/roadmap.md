# Roadmap

Every ticket below is one worker session, one commit (or a few) straight to `main`, under about 400 changed lines excluding tests and fixtures. Read `docs/architecture.md` first: it fixes the shared types, units and rules. Each ticket names the research section with the exact API calls.

## Worker protocol (applies to every ticket)

1. Work in the worktree you were given. Run `git fetch origin && git rebase origin/main` before starting.
2. Read `AGENTS.md`, `docs/architecture.md`, the ticket and the research sections it names.
3. Change only the files the ticket lists. If something outside them must change, make the smallest change and say so in your final report.
4. Do not take design decisions. If the ticket is ambiguous or wrong, pick the option closest to `docs/architecture.md`, and list it under "Deviations" in your report.
5. `just ci` must pass: fmt, clippy on the host, clippy for Windows through cargo-xwin, and the tests.
6. Commit with the message `<crate>: <what changed>` (for example `mp4: add fragmented writer`). Then `git fetch origin && git rebase origin/main && just ci && git push origin HEAD:main`. On a push race, repeat the rebase.
7. Final report, under 15 lines: commit sha, what was built, the tests added, deviations, and anything left unverified.

---

## Wave 0

### P0: Repository foundation and shared types

Files: everything at the repository root, plus `crates/*` skeletons and `crates/core/src/{lib,time,media,game}.rs`.

1. Tooling, from `research/tooling.md` "Recommendation", with these changes:
   - `rust-toolchain.toml`: channel `1.99.1`, profile minimal, components clippy and rustfmt, target `x86_64-pc-windows-msvc`.
   - `.cargo/config.toml`, Windows target table: `linker = "rust-lld"` and `rustflags = ["-C", "target-feature=+crt-static"]`.
   - `justfile` recipes: `fmt`, `lint` (fmt check, host clippy `-D warnings`, `typos`), `check-win` (`cargo xwin clippy --workspace --all-targets --target x86_64-pc-windows-msvc --locked -- -D warnings`), `test` (`cargo test --workspace --locked`), `build-win` (release build of `greyscreen` and `greyscreen-cli`, then list the two exes), `dist` (runs `build-win` and zips both exes into `dist/greyscreen-windows-x64.zip`), `ci` (lint, check-win, test). Prefix every xwin recipe with `export PATH="/opt/homebrew/opt/llvm/bin:$PATH"`.
   - Workspace lints and profiles: copy `research/tooling.md` and add `[profile.dev.package."*"] debug = false`.
   - `.editorconfig`, `.gitignore` (add `/dist`), `.gitattributes`, `_typos.toml` (exclude `Cargo.lock` and `docs/research/**`).
   - `.github/workflows/ci.yml` from `research/tooling.md`. The Windows job uploads `greyscreen.exe` and `greyscreen-cli.exe` together.
   - Install `typos-cli` with brew if it is missing.
2. Workspace `Cargo.toml`: `members = ["crates/*"]`, resolver 3, edition 2024, `rust-version = "1.99"`, `license = "MIT"`, `publish = false`, `version = "0.1.0"`. Under `[workspace.dependencies]`, list every external crate the roadmap uses, at its latest version that resolves:
   - `windows` 0.62, `wasapi` 0.27 and `ureq` 3 (features `rustls`).
   - `serde` with derive, `serde_json`, `crossbeam-channel`, `thiserror` 2, `anyhow`.
   - `tracing`, `tracing-subscriber` (`env-filter`, `fmt`), `tracing-appender`.
   - `jiff`, `dirs` 6, `clap` 4 with derive, `tiny_http`.
   - `rune` (with `native-file-dialog`) and `rune-kit` (`default-features = false`), both as git deps on `https://github.com/anomalyco/rune` rev `7a707454fbfb8d3fd200723c65778fb7b98fd31e`.
   
   Add the six-entry `[patch.crates-io]` table at the same rev, exactly as in `research/ui-rune.md` section 1. Seed `Cargo.lock` from Rune's lockfile at that rev (`curl https://raw.githubusercontent.com/anomalyco/rune/7a707454fbfb8d3fd200723c65778fb7b98fd31e/Cargo.lock -o Cargo.lock`), then let cargo resolve it.
3. Crates. Each has `[lints] workspace = true` and the workspace package fields. Create every module file listed below with empty content (or the stated stub), and declare each in `lib.rs`/`main.rs` (one `mod` per line, sorted):
   - `crates/core` (`greyscreen-core`): `time`, `media`, `game`, `highlight`, `settings`, `paths`, `session`, `clips`, `library`. Deps: serde, serde_json, crossbeam-channel, thiserror, jiff, dirs, tracing.
   - `crates/mp4` (`greyscreen-mp4`): `bytes`, `avc`, `aac`, `moov`, `fragmented`, `finalize`, `clip`, `reader`, `recover`. Deps: greyscreen-core, thiserror, tracing. `#![forbid(unsafe_code)]`.
   - `crates/league` (`greyscreen-league`): `dto`, `parse`, `client`, `poller`, `locator`. Deps: greyscreen-core, ureq, serde, serde_json, crossbeam-channel, thiserror, tracing. Windows-only dep: `windows` with features `Win32_Foundation`, `Win32_UI_WindowsAndMessaging`, `Win32_System_Threading`, `Win32_System_ProcessStatus`.
   - `crates/capture` (`greyscreen-capture`): `lib.rs` starts with `#![cfg(windows)]`. Modules: `clock`, `com`, `d3d`, `wgc`, `vproc`, `h264`, `audio`, `aac`, `pacer`, `session`, `probe`. Deps: greyscreen-core, greyscreen-mp4, crossbeam-channel, thiserror, tracing. Windows-only deps: `wasapi`, and `windows` with the union of the feature lists in `research/capture.md`, `research/encoder.md` and `research/audio.md`, plus `Graphics_Capture`, `Graphics_DirectX`, `Graphics_DirectX_Direct3D11`, `Win32_System_WinRT_Graphics_Capture`, `Win32_System_WinRT_Direct3D11`, `Win32_Graphics_Gdi`, `Win32_UI_HiDpi`, `Foundation`, `Win32_System_Performance`, `Win32_Media_Audio`, `Win32_Media_KernelStreaming`, `Win32_Media_Multimedia`. Run `cargo xwin check` to confirm the feature names exist.
   - `crates/engine` (`greyscreen-engine`): `api`, `control`, `writer`, `ring`, `clipjob`, `library`, `fake`. Deps: greyscreen-core, greyscreen-mp4, greyscreen-league, crossbeam-channel, thiserror, tracing, serde_json, jiff. Windows-only dep: greyscreen-capture.
   - `crates/app` (`greyscreen`, bin): `main.rs` opens a Rune window titled "Greyscreen" (960×640, min 720×520) with one centred text "Greyscreen", built like Rune's `examples/hello`. Add `#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]`. Empty modules: `ui`, `view`, `tray`, `logging`. Deps: rune, rune-kit, greyscreen-core, greyscreen-engine, anyhow, tracing, tracing-subscriber, tracing-appender.
   - `crates/cli` (`greyscreen-cli`, bin): clap with subcommands `doctor`, `record`, `fake-league` and `recover`. Each prints "not implemented yet" and exits 2. Deps: clap, anyhow, greyscreen-core, greyscreen-engine, greyscreen-league, greyscreen-mp4, tiny_http, tracing, tracing-subscriber. Windows-only dep: greyscreen-capture.
4. Shared types, copied exactly from `docs/architecture.md` "Shared types":
   - `core::time`: `Hns`, `HNS_PER_SEC`, `mono_now`, `hns_to_secs`, `secs_to_hns`. `mono_now` uses a `OnceLock<Instant>` base.
   - `core::media`: `Track`, `Packet`, `VideoFormat`, `AudioFormat`, `CaptureTarget`, `CaptureConfig`, `MediaEvent`, `MediaSource`, `MediaSession`.
   - `core::game`: `Team`, `Player`, `GameInfo`, `GameResult`, `GameEvent` (variants exactly as in `research/events.md` section 11), `TimedEvent`.
   
   Serde rules: derive Serialize and Deserialize for the game types, use `#[serde(tag = "type")]` on enums with data, and rename to snake_case.
   
   Tests: `mono_now` is monotonic, the secs/hns round trip, `frame_ticks` = 1500 at 60 fps, and a `GameEvent` JSON round trip.
5. `AGENTS.md`: the worker protocol above, the "Rules for code" from the architecture doc, the commands (`just ci`, `just build-win`), and "the Mac cannot run Windows code: compile-check it with `just check-win`". `CLAUDE.md` contains only `@AGENTS.md`.
6. `README.md`: keep a short placeholder (title, one line, "work in progress"). R2 writes the real one. `LICENSE`: MIT, copyright 2026 andresmarpz.

Acceptance:
- `just ci` passes.
- `just build-win` produces `greyscreen.exe` and `greyscreen-cli.exe`.
- `cargo run -p greyscreen -- --snap /tmp/x.png` renders on the Mac.
- CI YAML is valid. Push, then check the run with `GH_TOKEN=$(gh auth token --user andresmarpz) gh run list -R andresmarpz/greyscreen`.

---

## Wave 1

### C1: Settings and paths (`core::settings`, `core::paths`)

- `paths`:
  - `config_dir()`: `dirs::config_dir()/Greyscreen` (gives `%APPDATA%\Greyscreen` on Windows and `~/Library/Application Support/Greyscreen` on macOS).
  - `logs_dir()`: `dirs::data_local_dir()/Greyscreen/logs`.
  - `default_output_dir()`: `dirs::video_dir()/Greyscreen`, falling back to `home/Videos/Greyscreen`.
  - `settings_file()`: `config_dir()/settings.json`.
- `settings`: `Settings`, `ClipSettings`, `Quality`, `Resolution`, exactly as in the architecture table. Use serde `default` on every field so old files load.
  - `Settings::load(path) -> Settings` returns the default on a missing or invalid file and logs a warning.
  - `Settings::save(&self, path) -> io::Result<()>` writes a temp file and renames it.
  - `Settings::validate(&mut self)` clamps `fps` to 30 or 60 (nearest) and both clip seconds to `1..=120`.
  - `Settings::capture_config(native_height: u32) -> CaptureConfig`: `max_height` from the resolution (None for Native, else 1440, 1080 or 720, never above native). Bitrate per the architecture formula, computed on the output height with 16:9 width. `gop_frames = fps`, `audio_bitrate = 192_000`.
- Tests: defaults, a JSON round trip, an unknown-field file and a missing-field file both load, validate clamps, and bitrate at 1080p60 High = 20_000_000, 1440p60 High ≈ 35.5M, 720p30 Balanced ≈ 2.67M.

### C2: Highlight classification (`core::highlight`)

- `Highlight`, `Objective`, `Structure` per the architecture doc. Add `Highlight::label(&self) -> &'static str` returning short PascalCase labels for file names: `MyKill`, `MyDeath`, `Multikill2`..`Multikill5`, `Baron`, `Dragon`, `Herald`, `VoidGrub`, `Turret`, `Inhibitor`, `Ace`, `Victory`, `Defeat` and so on.
- Add `Highlight::is_clip_worthy(&self, clips: &ClipSettings) -> bool`, mapping to the settings toggles:
  - kills → `MyKill` and `MyFirstBlood`
  - deaths → `MyDeath`
  - assists → `MyAssist`
  - multikills → `MyMultikill`
  - objectives → `TeamObjective` and `EnemyObjective`
  - structures → `TeamStructure` and `EnemyStructure`
  - aces → `MyAce`
- `classify(event, info)` follows the rules in `research/events.md` section 11 exactly. Name matching is case-insensitive, compares after stripping any `#tag`, and also matches the champion name for bots.
- `structure_team(id: &str) -> Option<Team>`: `T1` is Order, `T2` is Chaos.
- Tests: one per `classify` rule, including one kill that yields both MyKill and MyAssist (impossible in game, but the function returns a list), an environment killer, and the structure ids from the research.

### C3: Session state machine (`core::session`)

- Implement the lifecycle in `docs/architecture.md` "Session lifecycle".
- `Input`: `WindowFound { hwnd, pid }`, `Ready(GameInfo)`, `Event(TimedEvent)`, `Ended(GameResult)`, `Gone`, `Skipped(String)`, `MediaStarted { t0: Hns }`, `MediaFailed(String)`, `Finalized`, `UserStart`, `UserStop`, `Tick(Hns)`, `SettingsChanged { auto_record: bool }`.
- `Effect`: `StartMedia(CaptureTarget)`, `StopMedia`, `Finalize { result: GameResult }`, `Highlight(TimedEvent)`, `Status(Status)`.
- `Status` (core-level): `Idle`, `Waiting`, `Recording { since: Hns, game: Option<GameInfo> }`, `Finalizing`, `Error(String)`.
- `SessionState` is an enum whose variants carry only their own data.
- Rules:
  - `auto_record = false` ignores `WindowFound`.
  - `Event` is forwarded as `Highlight` only while `Recording` or `Tail`.
  - `Ended` enters `Tail { until: now + 5 s }`, using the last `Tick` time as now.
  - `MediaFailed` while `Starting` returns to `Armed` with `Status(Error)`.
  - `UserStart` works only in `Idle` and targets `PrimaryMonitor`.
  - `UserStop` from `Recording` goes to `Finalizing` with result `Unknown`.
  - A 3-hour cap: a `Tick` more than 3 h past `since` acts as `UserStop`.
- Tests: the happy path, gone before ready, a skipped TFT game, media failure, the tail timing, the 3 h cap, manual start and stop, and auto-record off.

### C4: Clip planner and library schema (`core::clips`, `core::library`)

- `clips`:
  - `ClipPlan { start: Hns, end: Hns, label: String, events: Vec<u32> }`.
  - `ClipPlanner::new(pre: Hns, post: Hns)` with `on_highlight(at: Hns, label: &str, event_id: u32)` and `due(now: Hns) -> Vec<ClipPlan>`.
  - Merge rule: a highlight whose `at - pre` falls before the open plan's `end` extends that plan's end to `at + post` and appends the label as `A+B` (at most 3 labels, then `+more`).
  - A plan is due when `now >= end + 1 s`. Plans never exceed 90 s. When one would, close it and open a new one.
  - Also `flush() -> Vec<ClipPlan>` for session end.
  - Tests for the merge, the cap, due timing and flush.
- `library`:
  - `RecordingMeta`, `EventEntry`, `ClipEntry`, `VideoInfo`, exactly the sidecar schema in the architecture doc "On disk", with serde.
  - `folder_name(started: jiff::Zoned, champion: &str, mode: &str) -> String` gives `YYYY-MM-DD_HH-MM-SS_<Champion>_<MODE>`, sanitised to `[A-Za-z0-9_-]`.
  - `clip_file_name(rec_secs: f64, label: &str) -> String` gives `MM-SS_<label>.mp4`.
  - `write_meta(dir, &meta)` writes atomically through a temp file and rename. `read_meta(dir)`.
  - `scan(output_dir) -> Vec<RecordingMeta>` reads `*/game.json`, skips bad files with a warning, and sorts newest first. Tests use a temp dir.

### M1: MP4 primitives (`mp4::bytes`, `mp4::avc`, `mp4::aac`, `mp4::moov`, `mp4::reader`)

Spec: `research/container.md` "Box layout" and "Converting to AVCC".
- `bytes`: a `BoxWriter` over `Vec<u8>` with `begin(fourcc) -> Mark`, `end(Mark)` (patches the 32-bit size), `full_box(fourcc, version, flags)`, and `u8`/`u16`/`u32`/`u64`/`i32`/`fixed16_16`/`bytes` writers.
- `avc`:
  - `split_annexb(&[u8]) -> impl Iterator<Item=&[u8]>` handles 3- and 4-byte start codes and trailing zeros.
  - `nal_type(u8)`.
  - `annexb_to_avcc(&[u8]) -> (Vec<u8>, AvcInfo { idr: bool, sps: Option<Vec<u8>>, pps: Option<Vec<u8>> })` drops SPS, PPS, AUD (9), SEI (6) and prefix (14) NALs from the sample but returns SPS/PPS.
  - `parse_sps_dims(&[u8]) -> Option<(u32, u32, u8 profile, u8 level)>` (exp-Golomb, cropping, high-profile chroma fields).
  - `write_avcc_box(&mut BoxWriter, sps, pps)` includes the High-profile extension bytes.
- `aac`: `asc_lc(sample_rate, channels) -> Vec<u8>` and `write_esds(&mut BoxWriter, asc, avg_bitrate)`.
- `moov`: `write_ftyp(major, compatible)`, and `write_init_moov(&VideoFormat, Option<&AudioFormat>) -> Vec<u8>` (fragmented init with `mvex`/`trex`, empty sample tables, `colr` nclx 1/1/1 limited range and `pasp` 1:1 in `avc1`).
  - A `SampleTables` struct (per-track `Vec<SampleRow { offset: u64, size: u32, duration: u32, sync: bool }>`).
  - `write_full_moov(&VideoFormat, Option<&AudioFormat>, &SampleTables, chunking) -> Vec<u8>` for a non-fragmented file: `stts` run-length, `stss`, `stsc`, `stsz`, `co64`, real durations.
- `reader`: a minimal box iterator over a byte slice (`fourcc`, `size`, `payload range`, handles size 1 and size 0) for tests and recovery.
- Tests:
  - An SPS from a real ffmpeg-made H.264 stream parses to the right size. Generate fixtures with `ffmpeg -f lavfi -i testsrc2=size=640x360:rate=60 -t 2 -c:v libx264 -profile:v high -bf 0 -g 60 -f h264 crates/mp4/tests/fixtures/v.h264`, kept under 300 KB.
  - Annex B to AVCC round trip.
  - The ASC for 48 kHz stereo is `0x11 0x90`.

### L1: Live Client client, DTOs and poller (`league::{dto, parse, client, poller}`)

Spec: `research/events.md` sections 1 to 7, 10 and 12.
- `client`: `LiveClient::new(base: &str)`, defaulting to `https://127.0.0.1:2999`.
  - Uses an `ureq::Agent` with `TlsConfig::builder().disable_verification(true)` and timeouts (connect 1 s, global 2 s).
  - Methods: `gamestats()`, `eventdata()`, `activeplayername()`, `playerlist()`, `replay_playback_ok() -> bool`.
  - Errors are a `thiserror` enum: `Refused`, `NotReady(u16)`, `Bad(String)`.
- `dto`: serde structs for those responses, with `#[serde(default)]` and the exact field names from the research. Events are a raw `serde_json::Value` map per event.
- `parse`: `to_game_event(&serde_json::Value) -> GameEvent`, tolerant (unknown name gives `Unknown`, and `Stolen` uses a local `loose_bool(&Value) -> bool` that accepts `"True"`, `"true"` and `true`). Also `to_game_info(activeplayername, playerlist, gamestats) -> Option<GameInfo>`, which finds me per research section 3.
- `poller`: `Poller::spawn(client, locator: Box<dyn GameLocator>, out: Sender<LeagueMsg>) -> PollerHandle` (stop on drop). It runs the state loop in research section 10 using `greyscreen_core::time::mono_now()`:
  - Compute the offset each poll from the midpoint of the `gamestats` request.
  - Map each new event to `TimedEvent.at`, de-dup by `EventID`, and drop events from before `Ready`.
  - Skip TFT and replays.
  - Emit `Ended` on `GameEnd`, and `Gone` on 3 refused polls or when the locator loses the window.
- `LeagueMsg` per the architecture doc, defined in `poller`.
- `locator` trait: `pub trait GameLocator: Send { fn find(&mut self) -> Option<(isize, u32)>; }`. Implement `AlwaysPresent` (returns `Some((0, 0))`, used on non-Windows and by tests). The Windows implementation is W1.
- Tests:
  - Parse every event example from research section 2.
  - `to_game_info` with tag and without.
  - The poller against an in-process `tiny_http` server (dev-dependency) that serves scripted JSON over plain http. Drive it through Ready, events, GameEnd, then Gone, and check the emitted messages and that `at` is monotonic.

### W1: Windows foundations (`capture::{clock, com, d3d, probe}`, `league::locator` Windows impl)

Spec: `research/capture.md` "Recommendation" steps 1 to 4, and `research/encoder.md` steps 1 to 3.
- `clock`: `qpc_now_hns()` and `QpcMap { offset }` with `QpcMap::now()` and `to_mono(qpc_hns) -> Hns`.
- `com`: `ComGuard::mta()` (CoInitializeEx MTA, CoUninitialize on drop) and `MfGuard::start()` (MFStartup FULL, MFShutdown on drop).
- `d3d`:
  - `GpuDevice { device: ID3D11Device, context: ID3D11DeviceContext, adapter_luid: LUID, adapter_name: String }`.
  - `GpuDevice::for_window(hwnd)` picks the adapter whose output contains the window's monitor (`MonitorFromWindow` + enumerate `IDXGIFactory1` adapters and outputs). Fall back to `EnumAdapterByGpuPreference(HIGH_PERFORMANCE)`, then the default adapter.
  - `GpuDevice::for_primary_monitor()`.
  - Create with `BGRA_SUPPORT | VIDEO_SUPPORT` and feature level 11_1 then 11_0, and set multithread protection.
  - `winrt_device() -> IDirect3DDevice`.
- `probe`:
  - `system_report() -> String` covering the Windows build (`RtlGetVersion` or the registry `CurrentBuild`), adapters with LUID and VRAM, `GraphicsCaptureSession::IsSupported()`, and every hardware H.264 encoder MFT from `MFTEnumEx(MFT_CATEGORY_VIDEO_ENCODER, HARDWARE | SORTANDFILTER, None, H264 out)` with its friendly name and vendor id.
  - Also call `SetProcessDpiAwarenessContext(PER_MONITOR_AWARE_V2)` in `init_process()`.
- `league::locator` (cfg windows): `WindowLocator`. `FindWindowW(w!("RiotWindowClass"), None)`, then `GetWindowThreadProcessId`, then check that the process image name ends with `League of Legends.exe` (`QueryFullProcessImageNameW` with `PROCESS_QUERY_LIMITED_INFORMATION`). Return `(hwnd as isize, pid)`.
- Every `unsafe` block has a `// SAFETY:` line. No tests can run on the Mac. `just check-win` is the gate.

---

## Wave 2

### M2: Fragmented writer and finalize (`mp4::fragmented`, `mp4::finalize`)

Spec: `research/container.md` "Box layout", "Soft remux at stop" and Pitfalls.
- `FragmentedWriter::create(path: &Path, video: &VideoFormat, audio: Option<&AudioFormat>) -> Result<Self>`:
  - Writes `ftyp` (`isom`, compat `isom iso6 avc1 mp41`), then the 16-byte `free` placeholder, then the init `moov`.
  - `push(&Packet)` buffers samples. On a video key packet (after the first), it flushes the current fragment as one `moof` + `mdat` with a single `write_all`, then starts a new one.
  - Keeps `SampleTables` with absolute offsets.
  - `sync_data` every 5 s of media.
  - `finish(self) -> Result<FinishedInfo { duration_secs, bytes }>` flushes, appends the full `moov`, and overwrites the placeholder (32-bit `free` + `mdat` header, or a 64-bit `mdat`).
  - The writer writes the file at the path it is given. The caller adds `.part` and renames it afterwards.
  - Sample durations come from the next dts on the same track. The last sample reuses the previous duration.
- Tests:
  - Feed the M1 fixture H.264 (converted with `avc::annexb_to_avcc`) plus synthetic AAC silence frames. `ffprobe -v error -show_streams -show_format` must report h264 640x360 and aac, with a duration within 50 ms.
  - Decode the result with `ffmpeg -v error -i out.mp4 -f null -`, which must produce no output.
  - Truncate a copy after the 3rd fragment and check that ffprobe still reads it.
  - Skip these tests with a message when `ffprobe` is not on PATH.
  - Generate the AAC fixture with `ffmpeg -f lavfi -i sine=frequency=440:sample_rate=48000 -ac 2 -t 2 -c:a aac -b:a 192k -f adts`, and strip the ADTS headers in a test helper.

### L2: Fake Live Client server (`cli fake-league`)

- `greyscreen-cli fake-league --port 2999 [--speed 1.0] [--script <file.json>]` serves plain http on `127.0.0.1` with `tiny_http`. The endpoints are those L1 uses.
- The built-in script (in code, as JSON in `crates/cli/src/fake_match.json`) runs a 6-minute match at speed 1:
  - Loading 404s for 5 s, then GameStart, then 6 champion kills involving the active player (2 kills, 2 deaths, 2 assists).
  - A Multikill 2, a dragon, a voidgrub, a herald, a turret, an inhibitor, a baron and an ace.
  - GameEnd Win at 6:00, then 10 s of answers, then refused.
- `gameTime` advances with wall time × speed.
- Also add an environment variable `GREYSCREEN_LIVE_URL` read by the engine (E1) to point the client at `http://127.0.0.1:2999`. Only document it here; E1 reads it.
- Tests: the script JSON parses, and the server answers `/liveclientdata/gamestats` with 404 then 200.

### W2: WGC capture (`capture::wgc`)

Spec: `research/capture.md` "Recommendation" steps 5 to 9 and 11, and Pitfalls.
- `WgcCapture::start(gpu: &GpuDevice, target: &CaptureTarget, qpc: QpcMap) -> Result<Self>`:
  - For a window, use `CreateForWindow`. For the primary monitor, use `CreateForMonitor(MonitorFromPoint(0,0, PRIMARY))`.
  - Frame pool `CreateFreeThreaded`, 2 buffers, BGRA.
  - Register `FrameArrived` and `Closed`.
  - Session settings: cursor off, border off (after `RequestAccessAsync(Borderless)`, ignoring failure), and `MinUpdateInterval` 1/fps when supported. Each is wrapped so a failure only logs.
- In `FrameArrived`:
  - Recreate the pool on a size change.
  - `CopySubresourceRegion` the content box into a `latest` texture owned by the capture. Recreate `latest` when the content size changes.
  - Store `Latest { size: (u32, u32), at: Hns, seq: u64 }` behind a `Mutex`.
- Public API:
  - `latest(&self) -> Option<(ID3D11Texture2D, (u32, u32), u64 seq)>` returns a clone of the COM pointer.
  - `is_closed() -> bool`.
  - `stop(self)` tears down in the research order.
- Catch errors inside the callback and never panic there. Count frames for logging.

### W3: Video processor (`capture::vproc`)

Spec: `research/capture.md` "Colour conversion and scaling" and `research/encoder.md` step 10.
- `VideoConverter::new(gpu, out_w, out_h, fps)`:
  - Creates the `ID3D11VideoDevice`/`ID3D11VideoContext` and the enumerator with input 1920×1080 (updated per input size) and output `out_w×out_h`.
  - Checks the format conversion.
  - Creates the processor.
  - Sets colour spaces `RGB_FULL_G22_NONE_P709` in and `YCBCR_STUDIO_G22_LEFT_P709` out.
- `NV12 pool` of 6 textures (`DXGI_FORMAT_NV12`, `RENDER_TARGET | VIDEO_ENCODER` bind flags, falling back to `RENDER_TARGET` alone if creation fails), each with an output view.
- `convert(&mut self, src: &ID3D11Texture2D, src_size: (u32,u32)) -> Result<Nv12Frame>`:
  - Picks a free pool slot.
  - Caches input views by texture pointer and recreates the enumerator and processor when `src_size` changes.
  - Letterboxes to keep the aspect ratio (dest rect centred) and fills the border black (`VideoProcessorSetOutputBackgroundColor` with YCbCr black).
  - Calls `VideoProcessorBlt` and then `context.Flush()`.
- `Nv12Frame { texture: ID3D11Texture2D, slot: usize }`, with `release(slot)` when the encoder is done. Track in-use slots. When all 6 are busy, return `Err(Busy)`.
- `encode_size(native: (u32,u32), max_height: Option<u32>) -> (u32, u32)`: keep the aspect ratio, round down to even, height ≤ max.
  - Unit test this function in a non-cfg module so it runs on the Mac: put it in `greyscreen-core::media` as `pub fn encode_size` instead and call it from here.

### W5: Audio capture (`capture::audio`)

Spec: `research/audio.md` "Recommendation", "API calls in order" and "Silence, timeline and A/V sync".
- `AudioCapture::start(target: AudioTarget, qpc: QpcMap, t0: Hns, out: Sender<PcmBlock>) -> Result<Self>` spawns its own thread with COM MTA and MMCSS "Pro Audio".
  - `AudioTarget::Process(pid)` uses `wasapi::AudioClient::new_application_loopback_client(pid, true)`.
  - `AudioTarget::System` uses the default render device in loopback.
  - Format: 48 kHz stereo f32, event mode, autoconvert.
- `PcmBlock { first_frame: u64, samples: Vec<i16> }` holds interleaved stereo, with frames counted from `t0`.
- Implement the gapless timeline rules exactly as in research:
  - 30 ms resync threshold, silence fill on a 50 ms timeout up to `now - 60 ms`, and SILENT flag → zeros.
  - Distrust device timestamps when the first one is more than 500 ms off.
  - Convert f32 to i16 with clamping.
- Put the timeline arithmetic in a pure struct `AudioTimeline` in `greyscreen-core::media` (so it is tested on the Mac): `AudioTimeline::new(t0, rate)` with `on_packet(ts: Option<Hns>, frames: usize, silent: bool) -> Vec<Span>` and `on_timeout(now) -> Option<Span>`, where `Span` is `Silence(n)`, `Data { skip, take }`. Write tests for gap, overlap, drift and timeout.
- `stop(self)` joins the thread.

### E0: Fake media source and fixtures (`engine::fake`)

- Fixtures in `crates/engine/fixtures/`, made with ffmpeg, a 4-second loop at 640×360, 60 fps, GOP 60, no B-frames:
  - `loop.h264` (Annex B).
  - `loop.aac` (ADTS 48 kHz stereo 192 kbps).
  - A `README` line with the exact commands.
  - Under 1 MB together.
- `FakeMediaSource` implements `MediaSource`. `start` spawns a thread that:
  - Parses the fixtures once (`OnceLock`).
  - Emits `Started` with formats from the SPS/PPS and the ASC, and `encoder: "fake"`.
  - Emits packets in real time (sleeping until each packet's time from `t0`), looping with continuous dts.
- `request_keyframe` is a no-op: every loop starts on an IDR and GOP = 1 s. `stop` joins the thread.
- `FakeMediaSource::fast()` emits without sleeping, for tests.
- Tests: 10 s of fast packets have continuous dts per track, key flags fall every 60 video frames, and `Started` comes first.

---

## Wave 3

### M3: Clip writer and recovery (`mp4::clip`, `mp4::recover`)

Spec: `research/container.md` "Clip writer" and "Crash recovery".
- `write_clip(path, video: &VideoFormat, audio: Option<&AudioFormat>, packets: &[Arc<Packet>]) -> Result<()>`:
  - Rebase both tracks so the first audio frame at or before the first video IDR is zero.
  - Write `ftyp`, `moov` (faststart, built with `moov::write_full_moov`, offsets computed before writing) and `mdat`.
  - Write to `path.tmp` and rename.
  - `packets` must start with a video IDR, or return an error.
- `recover(part: &Path) -> Result<PathBuf>`:
  - Parse `ftyp`/`free`/init `moov`, then `moof`/`mdat` pairs with `reader`.
  - Truncate at the first box past EOF.
  - Rebuild the sample tables from `tfhd`/`tfdt`/`trun`.
  - Run the same finalize as M2 (reuse `finalize`), then rename `.mp4.part` to `.mp4`.
  - On failure, rename to `.mp4.recovered-fmp4` and return an error.
- Tests (ffprobe-gated like M2):
  - A clip from the middle of the fixture stream decodes cleanly and starts with a key frame (`ffprobe -show_frames -read_intervals %+#1`).
  - Recover a writer output that was dropped without `finish`: it plays and has the right duration within 1 GOP.

### W4: H.264 hardware encoder (`capture::h264`)

Spec: `research/encoder.md` "Recommendation for v1" steps 4 to 15, Defaults and Pitfalls (all 16).
- `H264Encoder::new(gpu: &GpuDevice, w, h, fps, cfg: &CaptureConfig) -> Result<Self>`:
  - Enumerate with the adapter LUID filter (`MFTEnum2` if it exists, else `MFTEnumEx` and match `MFT_ENUM_ADAPTER_LUID` on each activate).
  - Activate, async unlock, `SET_D3D_MANAGER`.
  - Codec API in the research order (PeakConstrainedVBR, mean, peak, buffer size, GOP = `cfg.gop_frames`, B-frames 0, low latency, CABAC), logging every `IsSupported`/`SetValue`/`GetValue`.
  - Output type, then input type NV12, both with BT.709 attributes.
  - Read `MF_MT_MPEG_SEQUENCE_HEADER`.
  - Start streaming.
  - Event pump via `IMFMediaEventGenerator::GetEvent(MF_EVENT_FLAG_NO_WAIT)` polled from the encoder thread. Use no callback: the pacer thread owns the encoder.
- Public API:
  - `need_input(&mut self) -> Result<bool>` pumps events.
  - `submit(&mut self, frame: &Nv12Frame, pts_hns: i64, dur_hns: i64) -> Result<()>` builds the sample with `MFCreateDXGISurfaceBuffer`, `SetCurrentLength`, time and duration.
  - `force_keyframe()`.
  - `drain_output(&mut self, out: &mut Vec<EncodedVideo>) -> Result<()>` handles `HaveOutput`, `MF_E_TRANSFORM_STREAM_CHANGE` and provides-samples or not.
  - `finish(self, out)` drains in the research shutdown order.
- `EncodedVideo { annexb: Vec<u8>, pts_hns: i64, key: bool }`. Detect key from IDR NALs via `greyscreen_mp4::avc`.
- Release NV12 slots when the sample's output is produced. A simple FIFO of in-flight slots is enough: one input gives one output, with B-frames off and low latency on.
- `encoder_name() -> String` (friendly name + vendor).
- `NoHardwareEncoder` error when enumeration is empty or the four checks in Pitfall 15 fail.

### W6: AAC encoder (`capture::aac`)

Spec: `research/audio.md` "AAC encoding with Media Foundation" (direct MFT use, not the sink writer) and `research/container.md` "AAC audio".
- `AacEncoder::new(sample_rate: 48000, channels: 2, bitrate: 192_000) -> Result<Self>`:
  - `CoCreateInstance(CLSID_AACMFTEncoder)` as a sync `IMFTransform`.
  - Output type first (AAC, payload type 0, 24000 bytes/s), then input PCM16.
  - Read `MF_MT_USER_DATA` and take the AudioSpecificConfig from byte 12. If it is missing, use `greyscreen_mp4::aac::asc_lc`.
- `encode(&mut self, block: &PcmBlock, out: &mut Vec<Vec<u8>>) -> Result<()>`:
  - Builds a memory-buffer sample with time `first_frame * 1e7 / 48000` and a nonzero duration.
  - Calls `ProcessInput` and loops `ProcessOutput` until `MF_E_TRANSFORM_NEED_MORE_INPUT`.
  - Pushes one raw AAC frame per output.
- `finish(self, out)` drains. `asc(&self) -> Vec<u8>`.
- The caller assigns AAC frame dts as `k * 1024`.

### E1: Engine control and game writer (`engine::{api, control, writer, ring}`)

- `api`: `EngineCmd`, `UiMsg`, `EngineStatus` (re-export or wrap `core::session::Status`), `FeedItem { id, rec_secs, game_secs, text, highlights }`, `Notice { level: Info|Warn|Error, text }`, `RecordingEntry` (= `library::RecordingMeta` + folder path) and `ClipEntry`.
- `Engine::start(settings: Settings, media: Arc<dyn MediaSource>, league: LeagueFactory, sink: Box<dyn Fn(UiMsg) + Send + Sync>) -> EngineHandle`. `EngineHandle { cmd: Sender<EngineCmd> }` sends `Shutdown` on drop and joins.
  - `LeagueFactory` is `Box<dyn Fn(Sender<LeagueMsg>) -> PollerHandle + Send>`. The real one uses `LiveClient::new(env GREYSCREEN_LIVE_URL or default)`, with `WindowLocator` on Windows and `AlwaysPresent` elsewhere.
- `control`: one thread with `crossbeam_channel::select!` over the league, media, cmd and a 250 ms tick.
  - Runs `SessionState::step` and executes the effects:
    - `StartMedia`: build the `CaptureConfig` from settings (native height from the window, or 1080 when unknown) and call `media.start`.
    - `StopMedia`: call `session.stop()` on a helper thread, then feed `Finalized` when the writer is done.
    - `Highlight`: classify, push a `FeedItem`, add to the planner if clip-worthy, and store in the sidecar events.
  - Writes `game.json` at start (no duration), at each highlight (debounced 5 s) and at finalize.
  - Creates the folder via `library::folder_name`, using the champion and mode from `GameInfo`, or `Test`/`MANUAL` for manual recordings.
- `writer`: a thread owning `FragmentedWriter` on `game.mp4.part`, fed by a channel of `Arc<Packet>`. On finish it calls `finish`, renames to `game.mp4` and reports `FinishedInfo`. It is skipped when `record_full_game` is false.
- `ring`: `ClipRing { packets: VecDeque<Arc<Packet>>, keep: Hns }` with `push`, `prune` (keeps from the newest IDR older than `keep`) and `range(start, end) -> Vec<Arc<Packet>>`, which starts at the IDR at or before `start` and includes audio overlapping. Times are converted from packet ticks with `t0`.
- Tests (Mac, with `FakeMediaSource::fast()` and a scripted league channel instead of the poller): run a 60 s fake game with 3 highlights. Assert that `game.mp4` exists and passes ffprobe, that `game.json` has 3 events with `rec_secs` matching the scripted `at` within 10 ms, and that the status messages come in order.

---

## Wave 4

### W7: Capture session (`capture::pacer`, `capture::session`)

- `WindowsMediaSource` implements `greyscreen_core::media::MediaSource`. `start(target, cfg, out)`:
  1. Spawn the video thread.
  2. On it: `ComGuard::mta`, `MfGuard`, `GpuDevice::for_window`/`for_primary_monitor`, then `WgcCapture::start`.
  3. Wait up to 2 s for the first frame to learn the native size. With no frame, send `Failed("no frames: is League in exclusive fullscreen? use Borderless")`.
  4. Compute the encode size with `encode_size`, then create the `VideoConverter` and `H264Encoder`.
  5. Set `t0 = mono_now()`.
  6. Start `AudioCapture` (Process(pid) for a game window, System for the monitor) and the `AacEncoder` on its own thread, consuming `PcmBlock`s and producing AAC packets.
  7. Send `Started` once the SPS/PPS and ASC are known.
- `pacer` loop, at tick `n` (deadline `t0 + n * 1e7 / fps`; sleep with `std::thread::sleep` until 1 ms before, then spin-yield):
  1. Take `wgc.latest()`. If none, reuse the previous texture.
  2. Run `converter.convert` and `encoder.submit` with pts `n * 1e7 / fps`. If the encoder is not accepting or the pool is busy, drop the tick and count it.
  3. Drain output into `Packet`s: `annexb_to_avcc`, `dts = n_out * frame_ticks`, `key` from IDR.
  4. Skip ticks that were missed by more than 2 frames, but keep `n` tied to real time.
- `request_keyframe` sets an atomic flag that the pacer applies.
- `stop()` sets a stop flag, joins both threads (the encoders finish and drain), and returns once the last packets are sent.
- Log once per 10 s: captured fps, encoded fps, dropped ticks and the encoder queue.
- On the `Closed` event or `DEVICE_REMOVED`, send `Failed`.

### E2: Clips, library and commands (`engine::{clipjob, library}` + wiring in `control`)

- Clip flow:
  - The `ClipPlanner` lives in `control`. On each tick, `due(now)` returns plans to execute.
  - `clipjob::spawn(plan, ring.range(..), formats, dir)` writes `clips/<clip_file_name>` with `mp4::clip::write_clip`, then sends `ClipSaved` and appends to the sidecar `clips`.
  - At session end, flush the planner and run the remaining plans with whatever post-roll exists.
  - `SaveClipNow` plans `now - 30 s .. now` with the label `Manual`.
- `library`: `RescanLibrary` and engine start call `core::library::scan` on a worker thread, then send `UiMsg::Library`. After each `Finalized`, send `RecordingSaved`.
- Recovery: at engine start, for each `*/game.mp4.part`, run `mp4::recover::recover` on a worker thread and log the result.
- `ApplySettings`: store, save to `paths::settings_file()` on a worker thread, and feed `SettingsChanged`.
- `StartTestRecording { secs }`: feed `UserStart`, and on the tick after `secs`, `UserStop`.
- Tests: the E1 scenario now also yields one clip per clip-worthy highlight (merged where they overlap). Each clip passes ffprobe and starts with a key frame. A leftover `.part` from an aborted writer is recovered at start.

### A1: App shell and status screen (`app::{main, ui, view, tray, logging}`)

Spec: `research/ui-rune.md` sections 2, 3 and 8, and its Pitfalls.
- `logging`: `tracing-subscriber` with `tracing-appender` daily rolling into `paths::logs_dir()`, plus stderr in debug. The guard is kept in `main`. A panic hook logs the panic.
- `main`:
  - Single instance (`dev.greyscreen.app`) unless headless.
  - `set_app_icon` from `crates/app/assets/icon.png` (R1 makes the real one; use a 256 px purple square placeholder made with any tool now).
  - Load settings.
  - `App::new(AppSpec { min_size, ..AppSpec::new("Greyscreen", (960.0, 640.0), Ui::preview, view::root) }).keep_running().release_gpu_when_closed().init(...)`.
  - Start the engine with `WindowsMediaSource` on Windows and `FakeMediaSource` elsewhere, and a sink that calls `handle.update(move |ui, tx| ui.apply(msg, tx))`.
  - Start the tray (Open, Quit) and keep it alive.
  - Skip the engine, tray and single instance when `rune::is_headless()`.
- `ui`:
  - `Ui { screen: Screen, status, feed: VecDeque<FeedItem> (cap 500), recordings: Vec<RecordingEntry>, settings: Settings, notice: Option<(Notice, f64)>, cmd: Option<Sender<EngineCmd>> }`.
  - `Ui::apply(&mut self, UiMsg, &mut Tx)`.
  - `Ui::preview()` makes fake data for headless runs.
  - Scenes: `idle`, `recording`, `feed-long`.
- `view`:
  - `root` holds a top nav row (`nav-status`, `nav-recordings`, `nav-settings` ghost buttons) and the current screen.
  - The status screen has a state card (Idle / Waiting for game / Recording mm:ss + champion and mode / Finalizing / Error) and a "Test recording (10 s)" button (`EngineCmd::StartTestRecording { secs: 10 }`), shown only when idle.
  - A "Stop" button appears while recording.
  - The live feed list uses virtual rows with a badge per highlight and the game time.
  - A toast for notices.
  - Follow the system theme with `Kit::neutral(cx.system_dark())`.
  - Elapsed time: `tx.wake_after(250 ms)` from `poll` while recording.
- Tests: `Runtime` tests that tap `nav-*` and the test-recording button (assert the command was sent through a channel in the test `Ui`). `apply` unit tests. Render each scene with `--snap` in dark and light, look at the PNGs, and fix layout issues.

---

## Wave 5

### X1: CLI doctor, record and recover (`cli`)

- `doctor`: prints the app version, OS and paths. On Windows it adds `greyscreen_capture::probe::system_report()`, a 2-second WGC test on the primary monitor (frames received and size), and an encoder creation test at 1920×1080 60 fps (prints the encoder name and each codec property's IsSupported/Set/Get). Exit 0 when the capture and encoder tests pass.
- `record --seconds N [--out file.mp4] [--window <title substring>]`:
  - Runs `WindowsMediaSource` with `PrimaryMonitor` (or a window found by `EnumWindows` title match), straight into `FragmentedWriter` and `finish`.
  - Prints the encoder, frames and bytes.
  - On non-Windows, uses `FakeMediaSource`, so the command is testable on the Mac.
- `recover <path>` runs `mp4::recover::recover`.
- `fake-league` stays from L2.
- Tests: `record --seconds 2` on the Mac (fake) writes a file that ffprobe accepts.

### A2: Recordings screen (`app::view::recordings`)

- Table of recordings (date, champion, mode, result, duration, size, clips count). Sizes come from the sidecar plus a metadata read on the engine side. Add `size_bytes` to `RecordingEntry` in E2's scan.
- Buttons: "Open folder" (reveals the output dir), and per row "Show" (reveals the game folder) and "Review" (opens A4).
- Reveal runs in engine `api::reveal(path)`: on Windows `explorer.exe /select,<file>` or `explorer.exe <dir>`, on macOS `open -R`. It is spawned from a thread and never runs on the UI thread during a frame.
- Empty state text. Scene `recordings-many` with 30 fake rows, `recordings-empty`.
- Tests: tapping a row's Review switches the screen with the right id.

### A3: Settings screen (`app::view::settings`)

Spec: `research/ui-rune.md` "Screen 3".
- Cards:
  - Recording: auto record switch, full game switch, quality segmented, resolution select, fps segmented 30/60, audio switch.
  - Clips: enabled, pre and post seconds text inputs with validation 1..=120, toggles for kills, deaths, assists, multikills, objectives, structures and aces.
  - Storage: output folder with a Change button (`tx.open_folder`) and Open.
  - About: version, logs folder Open button.
- Every change updates `ui.settings` and sends `ApplySettings(settings.clone())`.
- Scene `settings`, `settings-invalid`. Tests for the validation and for a change sending the command.

### A4: Review screen (`app::view::review`)

Spec: `research/ui-rune.md` section 4.
- `Player::file(game.mp4)` and a video surface (`SurfaceFit::Fit`). Play/pause button, current time / duration.
- Custom timeline: a track row with progress fill and markers at `rec_secs / duration`, coloured by highlight category (mine, team, enemy). Click a marker to `seek(rec_secs - 3)`. Drag on the track to seek.
- A side list of events (time, label) that seeks on click, and a list of clips that plays a clip file in the same player on click. Back button.
- `poll` asks `wake_after(33 ms)` while playing.
- Scene `review` using `crates/app/fixtures/review.mp4` (a 10 s test clip made with ffmpeg, under 500 KB) and fake events.
- Tests: the marker position math, and tapping a marker calls seek (wrap `Player` in a small trait so tests use a fake).

---

## Wave 6

### R0: Fast CI

The problem: the Windows job takes over 11 minutes and has never finished. On run 44836869394, clippy took 6m23s, `cargo test` was cancelled after 4 minutes, and the fat-LTO release build never started. There are four causes:

- `rust-cache` only saves after a successful job, and swarm pushes cancel every run, so every run builds about 500 crates cold.
- The tree is built three times.
- The release profile uses fat LTO.
- The Windows runners are slow.

Files: `.github/workflows/ci.yml`, and the root `Cargo.toml` profiles only.

1. Set `with: cache-on-failure: true` on every `Swatinem/rust-cache@v2` step.
2. Remove `cargo clippy` from the Windows job, because `just ci` already runs Windows clippy through cargo-xwin. Keep fmt and test on Windows, and keep clippy on macOS.
3. Move the release build and artifact upload into a separate `release` job:
   - It triggers on `push: tags: ['v*']` and `workflow_dispatch`.
   - It runs on `windows-latest` and builds `-p greyscreen -p greyscreen-cli`.
   - It uploads both exes.
   - Its concurrency group does not cancel.
4. Add `[profile.ci-release]` with `inherits = "release"`, `lto = "thin"` and `codegen-units = 16`. The `workflow_dispatch` build uses `--profile ci-release`. Tag builds keep `release`.
5. Pin `crate-ci/typos` to the latest release tag instead of `@v1`.
6. Keep `cancel-in-progress: true` for the push/PR group.

Acceptance:

- The YAML is valid.
- Trigger a second run after the first one (which may be cold). The Windows job finishes in under 5 minutes with a cache hit. Report both durations.
- `just ci` passes.

### R1: Icon, Windows resources and release build

- Icon: a hexagon frame mark in the purple of `assets/icon.svg` (`#9333ea`), as SVG at `assets/icon.svg` (replacing the letter H). Render PNGs at 16, 32, 48, 256 (`rsvg-convert` or `resvg` CLI) and build `crates/app/assets/greyscreen.ico`.
- `crates/app/build.rs` with `winresource` (reads `CARGO_CFG_TARGET_OS`): icon plus the manifest from `research/tooling.md`. The window icon uses `icon.png` 256 px.
- `cli` gets the same icon.
- The tray icon is the 32 px PNG decoded to RGBA.
- `just dist` zips both exes plus a `README.txt` with the first-run steps from `docs/plan.md`.
- Check that `just build-win` works, that `llvm-readobj --coff-imports` shows no `VCRUNTIME`, and that the manifest has `PerMonitorV2`.

### R2: README and release

- The README is written by a Gemini 4 Neon worker (the orchestrator runs it) from `docs/architecture.md` and the feature list, then checked against the code.
- Add GitHub repository description and topics.
- Create release `v0.1.0` with the zip, as a draft prerelease in the private repo.

### R3: Adversarial review and fixes

- One fresh Opus reviewer gets the `crates/capture`, `crates/engine` and `crates/mp4` code plus the goal. It looks for bugs, false claims, race conditions, COM lifetime errors, resource leaks and unsafe misuse. At most 2 rounds.
- The orchestrator files fix tickets (R3-x) for real bugs, and Sonnet workers run them.
