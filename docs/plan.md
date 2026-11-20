# Plan

How greyscreen gets built: the milestones, the order of work, and how each step is checked. `docs/architecture.md` holds the decisions and contracts. `docs/roadmap.md` holds the tickets.

## Milestones

| Milestone | What works | How it is checked |
|---|---|---|
| **M0 Foundation** | Workspace, tooling, CI, shared types. An empty Rune window builds for macOS and Windows | `just ci` green on the Mac. The Windows `.exe` cross-builds |
| **M1 Pure core** | Settings, event model and classification, session machine, clip planner, library schema, MP4 writer, Live Client poller | Unit tests on the Mac. MP4 output checked with `ffprobe` and decoded by `ffmpeg` |
| **M2 Capture pipeline** | WGC capture, NV12 conversion, H.264 MFT, process loopback, AAC MFT, capture session | Compiles for Windows with clippy clean. On the PC: `greyscreen-cli doctor` and `greyscreen-cli record --seconds 10` give a playable MP4 |
| **M3 Engine** | Game lifecycle, game file, clip ring and clip jobs, library, fake sources | Engine tests on the Mac run a full fake match end to end and check the files it writes |
| **M4 App v1** | Rune UI: status with live feed, recordings, settings, tray, single instance | Rune headless snapshots and `Runtime` tests on the Mac. On the PC: play a game and get `game.mp4`, `game.json` and clips |
| **M5 Review** | Player with event markers on a timeline, clip list per game | Headless snapshots with a fixture MP4 on the Mac |
| **M6 Release polish** | Icon, README, release build, adversarial review, fixes | Release `.exe` attached to a GitHub release |

v1 means M0 to M4. M5 and M6 follow straight on in the same run.

## Order and parallelism

Tickets run as waves. Inside a wave, tickets touch different files and run in parallel. Each runs in its own git worktree with a Sonnet 5.5 worker, which rebases on `origin/main` and pushes to `main`.

```
Wave 0  P0                                    repo, tooling, skeleton, shared types
Wave 1  C1 C2 C3 C4 M1 L1 W1                  core logic, mp4 primitives, league client, Windows foundations
Wave 2  M2 L2 W2 W3 W5 E0                     fragmented writer, fake league, WGC, video processor, audio, fake media
Wave 3  M3 W4 W6 E1                           clip writer and recovery, H.264 MFT, AAC MFT, engine core
Wave 4  W7 E2 A1                              capture session, engine library and clips, app shell and status
Wave 5  X1 A2 A3 A4                           CLI doctor and record, recordings, settings, review
Wave 6  R1 R2 R3                              icon and resources, README and release, review fixes
```

The orchestrator (the main session) owns every decision. It reviews each pushed ticket against its acceptance list before starting the next wave. It writes a follow-up ticket for any gap rather than letting a worker decide.

## Verification strategy

- **The Mac is the main test bed.** Everything except `capture` and the Windows locator runs on macOS. The engine runs a full match against `FakeMediaSource` (looped H.264 and AAC fixtures made with ffmpeg) and the fake Live Client server, then checks the MP4 with `ffprobe`.
- **Windows code is compile-checked** with `cargo xwin clippy -D warnings`. It is reviewed against the research notes, and an adversarial review covers it at R3.
- **Windows runs come from the user's PC.** The first build ships with `greyscreen-cli doctor` and `greyscreen-cli record`, so the capture pipeline can be tested without a game. The log file records every MFT and capture decision, so one run answers the open questions in the research notes.
- **CI** on GitHub Actions builds, lints and tests on `windows-latest` and `macos-latest`, and uploads the Windows `.exe` as an artifact.

## First run on Windows (for the user)

1. Download `greyscreen-windows-x64.zip` from the latest release (or the CI artifact) and unzip it.
2. Run `greyscreen-cli.exe doctor` in a terminal. It prints the GPU, the hardware encoders and the capture support.
3. Run `greyscreen-cli.exe record --seconds 10`. It records the primary monitor and system audio to `test.mp4`.
4. Start `greyscreen.exe` and play a game (Practice Tool is fine). Check the game folder under `Videos\Greyscreen`.
5. Send back `%LOCALAPPDATA%\Greyscreen\logs` if anything fails.

## Risks and what covers them

| Risk | Cover |
|---|---|
| Vendor MFT quirks (AMD, Intel) | Every codec property set is logged and read back. The `doctor` command reports them. A direct NVENC/AMF backend fits behind `MediaSource` later |
| WGC cannot see exclusive fullscreen | A watchdog warns when no frame arrives in 2 s. The UI tells the user to choose Borderless |
| Live Client TLS handshake fails under rustls | Logged with the exact error. A native-tls fallback is a small follow-up |
| Process loopback misses voice or game audio | `doctor` reports it. A setting falls back to system loopback |
| Colour tags dropped by the encoder | The MP4 writer writes `colr` (BT.709 limited) itself |
| Rune pre-release changes | Rev pinned, `Cargo.lock` committed, bump on purpose |
