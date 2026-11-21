# Architecture

This is the contract every ticket builds against. Each decision cites the research note behind it in `docs/research/`. When code and this file disagree, fix one of them in the same commit.

## Goals

- Record League of Legends games on Windows with no measurable FPS cost. Frames never leave the GPU until they are compressed.
- Start and stop with the game on their own. Mark game events on the timeline. Cut highlight clips without re-encoding.
- Never touch the game process. No injection, no hooks, no memory reads. Vanguard blocks and punishes those (`research/capture.md`, `research/events.md` section 9).
- Build on a Mac. Everything but the capture crate compiles, runs and is tested on macOS against fakes.

## Decisions

| Area | Decision | Rejected | Why (research note) |
|---|---|---|---|
| Video capture | Windows.Graphics.Capture on the game window (`RiotWindowClass`), hand-written on `windows` 0.62 | Desktop Duplication, hooks, `windows-capture` crate | `capture.md`: per-window, GPU textures, safe with Vanguard |
| Colour and scale | `ID3D11VideoProcessor`, BGRA to NV12, BT.709 limited, letterbox to a fixed encode size | Compute shader | `capture.md`, `encoder.md` |
| Frame pacing | Constant frame rate. A pacer re-encodes the latest frame on each tick | Variable frame rate | `encoder.md` Timestamps |
| Encoder | Hardware H.264 MFT driven directly (async events), High profile, PeakConstrainedVBR, no B-frames, GOP = 1 s | Sink writer, FFmpeg, NVENC/AMF SDKs (later) | `encoder.md`: packets for the clip ring, no DLLs |
| Audio | WASAPI process loopback on the game PID (`wasapi` 0.27), 48 kHz stereo float, gapless timeline with silence fill | System loopback (kept only for the test recording) | `audio.md` |
| Audio encoder | `CLSID_AACMFTEncoder` driven directly (sync MFT), PCM16 in, AAC-LC 192 kbps out | Sink writer insertion | `audio.md`, `container.md` |
| Container | Own pure-Rust MP4 writer: fragmented MP4 while recording, "hybrid" finalize at stop (OBS design), regular MP4 for clips | MF sink, `shiguredo_mp4`, `mp4` crate, MKV | `container.md`: crash safe, one encoder feeds file and clips, testable on the Mac |
| Clips | RAM ring of encoded packets, clip cut at the previous IDR, written as faststart MP4 | Cutting from the game file later | `container.md` Clip writer |
| Game detection | 1 Hz `FindWindowW("RiotWindowClass")` gives HWND and PID, then the Live Client API gates the start | Process scan only, WMI, LCU | `capture.md`, `events.md` |
| Events | Live Client Data API, 1 Hz poll of `/eventdata` then `/gamestats`, de-dup by `EventID`, fresh time offset each poll | LCU (v2), `allgamedata` | `events.md` sections 6, 10 |
| TLS to 127.0.0.1:2999 | `ureq` 3 with rustls (ring) and `disable_verification`, connect only to the literal `127.0.0.1` | Root pinning first (the root is X.509 v1 with SHA-1; rustls rejects SHA-1) | `events.md` section 5. Loopback and read-only, so verification adds nothing |
| UI | Rune pinned to dev rev `7a707454fbfb8d3fd200723c65778fb7b98fd31e`, `rune-kit` without default features | | `ui-rune.md` |
| Backend to UI | `AppHandle::update` with a `UiMsg` enum and a pure `Ui::apply` | Channel plus poll | `ui-rune.md` section 2 |
| Errors and logs | `thiserror` in libraries, `anyhow` in binaries, `tracing` to a daily file in the logs folder | | `tooling.md` |
| Time and dates | `jiff` for wall-clock dates and local-time folder names | `chrono` | Modern, handles the Windows time zone |

## Workspace

```
crates/
  core/     greyscreen-core     Pure logic. Settings, events, classification, clips, session machine, library schema, shared media types. No threads, no OS calls except settings and library file I/O.
  mp4/      greyscreen-mp4      Pure-Rust MP4: H.264 and AAC helpers, fragmented writer, finalize, clip writer, recovery.
  league/   greyscreen-league   Live Client Data API client and poller. The Windows game-window locator sits behind cfg(windows).
  capture/  greyscreen-capture  Windows only (`#![cfg(windows)]`). D3D11, WGC, video processor, H.264 MFT, WASAPI, AAC MFT, capture session.
  engine/   greyscreen-engine   Threads and I/O. Runs the session machine, fans packets out to the game writer and the clip ring, writes the library. Fake media source for the Mac.
  thumbs/   greyscreen-thumbs   One MP4 frame to a 16:9 JPEG thumbnail through the OS decoder. macOS through AVFoundation, Windows through Media Foundation; other targets return `Unsupported`. Used by the engine's background thumbnail worker.
  app/      greyscreen          The Rune UI binary.
  cli/      greyscreen-cli      Console tool: `doctor`, `record`, `fake-league`, `recover`, `thumb`.
```

Dependency direction: `core` ← `mp4` ← `capture`; `core` ← `league`; (`core`, `mp4`, `league`, `capture` on Windows) ← `engine` ← (`app`, `cli`).

## Clocks and time units

- **Mono time `Hns`** (`i64`, 100 ns units): `greyscreen_core::time::mono_now()` measures from a process-wide `Instant` base. All cross-thread times use it. Rust's `Instant` is QPC on Windows.
- **QPC to mono.** WGC `SystemRelativeTime` and the WASAPI `pu64QPCPosition` are QPC in 100 ns units. The capture crate converts them with one offset measured at session start: `mono = qpc_hns - (qpc_now_hns() - mono_now())`.
- **Session zero `t0`** (mono `Hns`): the moment the capture session starts. Video frame `n` and audio sample `k` count from `t0`.
- **Track ticks:** video timescale `90_000`, audio timescale = sample rate (`48_000`). `Packet` times are track ticks from `t0`. Video `dts = n * 90_000 / fps` (1500 at 60 fps). Audio `dts = k * 1024` per AAC frame.
- **Event time:** the poller gives each event a mono instant: `at = offset + EventTime * 1e7`, where `offset = mono_mid(gamestats request) - gameTime * 1e7` from the latest poll. The engine maps it to `rec_secs = (at - t0) / 1e7`.

## Shared types (in `greyscreen-core`)

These are exact signatures. Tickets implement them as written. Derive `Debug, Clone` everywhere and `PartialEq` where it costs nothing. `Serialize`/`Deserialize` are noted per type.

```rust
// core::time
pub type Hns = i64;
pub const HNS_PER_SEC: i64 = 10_000_000;
pub fn mono_now() -> Hns;
pub fn hns_to_secs(h: Hns) -> f64;
pub fn secs_to_hns(s: f64) -> Hns;

// core::media
pub enum Track { Video, Audio }
pub struct Packet {
    pub track: Track,
    pub dts: i64,          // track ticks from t0
    pub pts: i64,          // == dts while B-frames are off
    pub duration: u32,     // track ticks
    pub key: bool,         // video: IDR. audio: always true
    pub data: std::sync::Arc<[u8]>, // video: AVCC (4-byte lengths, no SPS/PPS/AUD). audio: one raw AAC frame
}
pub struct VideoFormat { pub width: u32, pub height: u32, pub fps: u32, pub sps: Vec<u8>, pub pps: Vec<u8> } // sps/pps: NAL units without start codes
pub struct AudioFormat { pub sample_rate: u32, pub channels: u16, pub bitrate: u32, pub asc: Vec<u8> }      // asc: AudioSpecificConfig
impl VideoFormat { pub const TIMESCALE: u32 = 90_000; pub fn frame_ticks(&self) -> u32; }

pub enum CaptureTarget { GameWindow { hwnd: isize, pid: u32 }, PrimaryMonitor }
pub struct CaptureConfig {
    pub fps: u32,                // 30 | 60
    pub max_height: Option<u32>, // None = native
    pub video_bitrate: u32,      // bits/s, mean
    pub video_peak: u32,         // bits/s
    pub gop_frames: u32,         // == fps
    pub audio: bool,
    pub audio_bitrate: u32,      // 192_000
    pub cursor: bool,
}
pub enum MediaEvent {
    Started { t0: Hns, video: VideoFormat, audio: Option<AudioFormat>, encoder: String },
    Packet(Packet),
    Warning(String),
    Failed(String),  // the session is dead; the engine stops it
}
pub trait MediaSource: Send + Sync {
    fn start(&self, target: CaptureTarget, cfg: CaptureConfig, out: crossbeam_channel::Sender<MediaEvent>)
        -> Result<Box<dyn MediaSession>, String>;
}
pub trait MediaSession: Send {
    fn request_keyframe(&self);
    fn stop(self: Box<Self>); // blocks until every packet is sent
}
```

`MediaEvent::Started` comes before any `Packet`. Packets are in dts order per track. The two tracks interleave in arrival order.

```rust
// core::game  (serde: Serialize + Deserialize, `#[serde(tag = "type")]` on enums)
pub enum Team { Order, Chaos }
pub struct Player { pub name: String, pub champion: String, pub team: Team, pub is_bot: bool } // name = riotIdGameName or summonerName
pub struct GameInfo { pub mode: String, pub map: Option<u32>, pub me: Player, pub players: Vec<Player> }
pub enum GameResult { Win, Loss, Unknown }
pub enum GameEvent { /* exactly the enum in research/events.md section 11 */ }
pub struct TimedEvent { pub id: u32, pub game_secs: f64, pub at: Hns, pub kind: GameEvent }

// core::highlight
pub enum Highlight {
    MyKill, MyDeath, MyAssist, MyMultikill { streak: u8 }, MyFirstBlood,
    AllyKill, EnemyKill,
    TeamObjective(Objective), EnemyObjective(Objective),
    TeamStructure(Structure), EnemyStructure(Structure),
    MyAce, EnemyAce, GameWon, GameLost,
}
pub enum Objective { Dragon(String), Herald, Baron, VoidGrub }
pub enum Structure { Turret, Inhibitor }
pub fn classify(event: &GameEvent, info: &GameInfo) -> Vec<Highlight>;
```

The league crate sends `LeagueMsg` (defined in `greyscreen-league`):

```rust
pub enum LeagueMsg {
    WindowFound { hwnd: isize, pid: u32 },   // the game window exists. The API may not answer yet
    Ready(GameInfo),                          // API answers with a numeric gameTime and a supported mode
    Event(TimedEvent),
    Ended(GameResult),                        // GameEnd seen
    Gone,                                     // window gone, or 3 refused polls after Ready
    Skipped(String),                          // TFT, replay or spectator: no recording for this process
}
```

## Session lifecycle (`core::session`)

One explicit state machine: `enum SessionState` and `fn step(&mut self, input: Input) -> Vec<Effect>`. No timers inside. The engine feeds `Input::Tick(now)`.

```
Idle --WindowFound--> Armed --Ready(info)--> Starting --MediaStarted(t0)--> Recording
Recording --Ended(result)--> Tail(until = now + 5 s) --Tick past until--> Finalizing --Finalized--> Cooldown
Recording/Tail --Gone or MediaFailed--> Finalizing
Armed/Starting --Gone--> Idle
Cooldown --Gone--> Idle
Any --Skipped--> Cooldown
```

`Effect` values: `StartMedia(CaptureTarget)`, `StopMedia`, `Finalize { result }`, `Highlight(TimedEvent, Vec<Highlight>)`, `Status(EngineStatus)`. Manual recording (`Input::UserStart`/`UserStop`) is `Idle --UserStart--> Starting` with `PrimaryMonitor`. It is used by the test recording.

## Engine data flow

```
league poller ──LeagueMsg──┐
                           ├─> engine control thread: SessionState::step -> effects
media session ─MediaEvent──┤          │
                           │          ├─> writer thread: GameWriter (fMP4 .part, finalize at stop)
                           │          ├─> ClipRing (Arc<Packet> deque, ~(pre + post + 2) s)
                           │          └─> ClipPlanner -> clip jobs (thread per clip) -> clips/*.mp4
UI commands ──EngineCmd────┘          └─> UiMsg sink -> AppHandle::update
```

- `EngineCmd`: `ApplySettings(Settings)`, `StartTestRecording { secs: u32 }`, `StopRecording`, `RescanLibrary`, `SaveClipNow`, `Shutdown`.
- `UiMsg`: `Status(EngineStatus)`, `Feed(FeedItem)`, `ClipSaved(ClipEntry)`, `RecordingSaved(RecordingEntry)`, `Library(Vec<RecordingEntry>)`, `Notice { level, text }`.
- `EngineStatus`: `Idle`, `Waiting`, `Recording { since: Hns, game: Option<GameInfo> }`, `Finalizing`, `Error(String)`.

## On disk

```
<output_dir>/                                   default: %USERPROFILE%\Videos\Greyscreen, macOS ~/Movies/Greyscreen
  2026-11-20_19-55-01_Ahri_CLASSIC/
    game.mp4            (game.mp4.part while recording)
    game.json           library sidecar, the source of truth for the UI
    thumb.jpg           game thumbnail, made in the background
    clips/
      12-34_MyKill.mp4
      12-34_MyKill.jpg  clip thumbnail
%APPDATA%\Greyscreen\settings.json                macOS ~/Library/Application Support/Greyscreen/settings.json
%LOCALAPPDATA%\Greyscreen\logs\greyscreen.log.YYYY-MM-DD
```

The sidecar `game.json` (serde, `version: 1`) holds: `id`, `started_at` (RFC 3339), `duration_secs`, `video` (relative path), `game` (`GameInfo` + `result`), `events` (`[{ id, game_secs, rec_secs, kind, highlights }]`), `clips` (`[{ file, start_secs, end_secs, label }]`), `encoder` (string), `video_format` (`width`, `height`, `fps`). It may also hold `review` (`{ deaths: [{ event_id, tags, note }] }`, tags in snake_case from `core::review::DeathTag`). `review` is left out while nothing is tagged or noted, and older sidecars without it still read.

## Settings (`core::settings`, serde with `#[serde(default)]`)

| Field | Type | Default |
|---|---|---|
| `output_dir` | `PathBuf` | Videos/Greyscreen (Movies on macOS) |
| `auto_record` | `bool` | `true` |
| `record_full_game` | `bool` | `true` |
| `quality` | `Quality { Balanced, High, Max }` | `High` |
| `resolution` | `Resolution { Native, P1440, P1080, P720 }` | `P1080` |
| `fps` | `u32` (30 or 60) | `60` |
| `audio` | `bool` | `true` |
| `capture_cursor` | `bool` | `true` |
| `clips.enabled` | `bool` | `true` |
| `clips.pre_secs` / `clips.post_secs` | `u32` | `15` / `5` |
| `clips.kills`, `deaths`, `multikills`, `objectives`, `aces` | `bool` | `true` |
| `clips.assists`, `structures` | `bool` | `false` |
| `check_updates` | `bool` | `true` |
| `nav_style` | `NavStyle { Sidebar, Dock }` | `Sidebar` |
| `sidebar_collapsed` | `bool` | `false` |
| `nav_width` | `u32` points, 180 to 320 | `220` |
| `list_width` | `u32` points, 240 to 440 | `300` |
| `client_id` | `Option<String>` | `None` (a random UUID is stored the first time feedback is sent) |

Layout: on the Games screen the games list is the first column inside the inset content card, then a hairline divider, then the review (the same in both nav styles). The nav sidebar's edge and the list/review divider drag to resize; double-click a handle resets its default, and the widths clamp to the ranges above and save when the drag ends. The rail (56 points, also forced below 1200 points of window width) is not resizable. Values outside a range are clamped when settings load.

Bitrate: the mean bitrate at 1080p60 is 8, 12 or 20 Mbps for Balanced, High or Max. It scales with pixel count to the power 0.75 and with fps/60 to the power 0.8. Peak is 2.0 times the mean. `CaptureConfig::for_encode_size` rescales with the same 0.75 exponent when the real encode size is smaller than `bitrate_pixels`.

Quality: `CaptureConfig.quality` is `Some(60)`, `Some(70)` or `Some(80)` for Balanced, High or Max. With `Some(q)` the encoder uses Quality rate control with `AVEncCommonQuality = q`, and mean and peak only cap through `MaxBitRate` and the buffer size. If the encoder rejects or misreads either value, it falls back to peak-constrained VBR at the mean and peak, and logs one warning. `MediaEvent::Started.encoder` ends with the mode, for example `(quality 70)` or `(vbr 18.5 Mbps)`. `Settings::estimated_mb_per_minute` returns 0.6 times the mean bitrate plus audio, for the size line in Settings (Native counts as 1440p). The 0.6 factor is a placeholder until real recordings give numbers.

`greyscreen-cli record` takes `--preset`, `--rc quality|vbr`, `--quality N`, `--bitrate MBPS`, `--peak MBPS` and `--height H` to override the config for A/B tests.

## Updates

`greyscreen-league::updates` lists the GitHub releases of `andresmarpz/greyscreen` and downloads `greyscreen.exe`, checking size, the SHA-256 `digest` GitHub reports (skipped when missing) and the `MZ` header. `greyscreen-core::update` holds the pure rules: a build that is a prerelease sees previews and stable releases, a stable build sees stable only, and the newest newer release wins (`preview.N` orders by N, a stable release beats its previews). The running version is the workspace version, so it must equal the release tag without the `v` (`just release` enforces it).

The engine's update thread checks 10 s after start and every 6 hours when "Check for updates automatically" is on, only for the installed copy on Windows and never while a session is not idle. A manual check works anywhere. An installed copy downloads to `%TEMP%\Greyscreen\greyscreen-<tag>.exe`; "Install and restart" launches that file, and the existing installer path asks the running copy to quit and replaces it. A copy that is not installed only links to the release page. `UiMsg::Update(UpdateState)` drives the About card, and `Ui::update_available` holds the pending release for other screens. A downloaded update also shows a toast-style banner at the top centre of every screen and both nav styles, with "Restart to update" and "Later" (hidden until a newer version); it stays hidden while a game is in progress and in theater mode.

## Feedback

"Send feedback" (nav footer, rail, dock, Settings > Updates & About, and the unbound App action `send_feedback`) opens a dialog: kind (Bug, Idea, Other), message (required, 5000 characters), optional email, and "Include recent logs" (on by default). `EngineCmd::SendFeedback(FeedbackDraft)` runs on its own `engine-feedback` thread, which builds the `FeedbackReport` (`greyscreen-core::feedback`), sends it through `FeedbackTransport` (`greyscreen-league::FeedbackClient`, `ureq`) and answers with `UiMsg::Feedback(FeedbackResult)`. A failure carries the same report as JSON for the "Copy report" button.

`POST {API base}/v1/feedback` with header `x-greyscreen-version` and body `{kind: "bug"|"idea"|"other", message, email|null, app_version, os, log_tail|null, client_id}`; answers 202 `{id}`, 400 `{error}`, 429. The base is `greyscreen_core::api::BASE_URL`, or `GREYSCREEN_API_URL`. `log_tail` is the last 300 lines of the newest `greyscreen.log.*`, with the home folder replaced by `~` and the values of `summoner`/`riotId` style fields redacted, and is `null` when the box is not ticked. `client_id` is `Settings.client_id`, not tied to any identity.

## Rules for code

- No comments or doc comments, except one line in plain English when a "why" is not obvious. Names carry the meaning. Never mention tickets or files in code.
- Files stay under 500 lines, excluding tests. Tests go in a sibling `tests.rs` (`#[cfg(test)] mod tests;`) or in `tests/` for integration tests.
- `unsafe` only in `capture`, `thumbs` and the Windows locator, and each block carries a `// SAFETY:` line (clippy `undocumented_unsafe_blocks`).
- No `unwrap`/`expect` outside tests, except on a mutex lock.
- Libraries return `thiserror` errors. Binaries use `anyhow`.
- `just ci` passes before every push. It runs fmt, clippy on macOS and on Windows through cargo-xwin, and the tests.
