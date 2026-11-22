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
| Encoder | Hardware H.264 MFT driven directly (async events), High profile, PeakConstrainedVBR, no B-frames, GOP = 1 s. Fallback: Windows' software H.264 MFT (sync, GPU readback), capped at 1080p 30 fps. See Encoder selection | Sink writer, FFmpeg, NVENC/AMF SDKs (later) | `encoder.md`: packets for the clip ring, no DLLs |
| Audio | WASAPI process loopback on the game PID (`wasapi` 0.27), 48 kHz stereo float, gapless timeline with silence fill | System loopback (kept only for the test recording) | `audio.md` |
| Audio encoder | `CLSID_AACMFTEncoder` driven directly (sync MFT), PCM16 in, AAC-LC 192 kbps out | Sink writer insertion | `audio.md`, `container.md` |
| Container | Own pure-Rust MP4 writer: fragmented MP4 while recording, "hybrid" finalize at stop (OBS design), regular MP4 for clips | MF sink, `shiguredo_mp4`, `mp4` crate, MKV | `container.md`: crash safe, one encoder feeds file and clips, testable on the Mac |
| Clips | RAM ring of encoded packets, clip cut at the previous IDR, written as faststart MP4 | Cutting from the game file later | `container.md` Clip writer |
| Clip export | Cut a finished MP4 at keyframes with no re-encode (Original quality), or re-encode with Media Foundation to fit 10 MB (For Discord) | FFmpeg, a bundled encoder | Both presets read the game or clip file the user already has; Windows ships the codecs |
| Game detection | 1 Hz `FindWindowW("RiotWindowClass")` gives HWND and PID, then the Live Client API gates the start | Process scan only, WMI, LCU | `capture.md`, `events.md` |
| Events | Live Client Data API, 1 Hz poll of `/eventdata` then `/gamestats`, de-dup by `EventID`, fresh time offset each poll | LCU (v2), `allgamedata` | `events.md` sections 6, 10 |
| TLS to 127.0.0.1:2999 | `ureq` 3 with rustls (ring) and `disable_verification`, connect only to the literal `127.0.0.1` | Root pinning first (the root is X.509 v1 with SHA-1; rustls rejects SHA-1) | `events.md` section 5. Loopback and read-only, so verification adds nothing |
| UI | Rune pinned to dev rev `eb9f1104` (see `AGENTS.md` for why), `rune-kit` without default features and used stock | | `ui-rune.md` |
| Backend to UI | `AppHandle::update` with a `UiMsg` enum and a pure `Ui::apply` | Channel plus poll | `ui-rune.md` section 2 |
| Errors and logs | `thiserror` in libraries, `anyhow` in binaries, `tracing` to a daily file in the logs folder | | `tooling.md` |
| Time and dates | `jiff` for wall-clock dates and local-time folder names | `chrono` | Modern, handles the Windows time zone |

## Workspace

```
crates/
  core/     greyscreen-core     Pure logic. Settings, keybinds, events, event categories, classification, clips, session machine, library schema, death review, storage retention, disk thresholds, update rules, feedback report, install rules, plain-language setup problems (`problem`), encoder selection, shared media types, bundled game data (`gamedata`: mode and map names and structure ids decoded from `crates/core/data/game_data.json`), clip export presets and bitrate math (`export`), NV12 crop and scale (`nv12`). No threads, no OS calls except settings and library file I/O.
  mp4/      greyscreen-mp4      Pure-Rust MP4: H.264 and AAC helpers, fragmented writer, finalize, clip writer, recovery, read-only inspect (sample tables of a finished or unfinished file), demuxer and keyframe trim of a finished file (`demux`, `trim`).
  league/   greyscreen-league   Live Client Data API client and poller, champion icon download, GitHub release client for updates, feedback HTTP client. The Windows game-window locator sits behind cfg(windows).
  capture/  greyscreen-capture  Windows only (`#![cfg(windows)]`). D3D11, WGC, video processor, H.264 MFTs (hardware and software) and the encoder check, WASAPI, AAC MFT, capture session, global hotkeys, installer, single-instance event, crash filter, clip export re-encoder (`transcode`), file clipboard (`clipboard`).
  engine/   greyscreen-engine   Threads and I/O. Runs the session machine, fans packets out to the game writer and the clip ring, writes the library, makes thumbnails and seek bar storyboards, recovers crashed recordings, enforces the storage limit and the low disk guard, deletes games, updates, sends feedback, exports clips. Fake media source for the Mac.
  thumbs/   greyscreen-thumbs   One MP4 frame to a 16:9 JPEG thumbnail through the OS decoder, and a keyframe pass that packs many small frames into JPEG sprite sheets (`write_sheets`, used for storyboards). macOS through AVFoundation, Windows through Media Foundation; other targets return `Unsupported`. No Greyscreen dependencies. Used by the engine's thumbnail worker, the CLI and the app.
  app/      greyscreen          The Rune UI binary. It also holds the setup window.
  cli/      greyscreen-cli      Console tool: `doctor`, `record`, `fake-league`, `recover`, `thumb`, `storyboard` (builds the seek bar storyboard of a recording folder and prints the time it took), `analyze` (recording health report), `demo` (seeds or clears a fake library).
  qa/       greyscreen-qa       Headless UI QA (`docs/qa.md`). A library (case format, locators, tree checks, script compiler, image diff) and a runner binary that drives the app binary through Rune's `--script`, `--tree` and `--snap`. No Greyscreen dependencies; never shipped.
```

Dependency direction: `core` ← (`mp4`, `league`, `capture`, `engine`); `mp4` ← (`capture`, `engine`); `league` ← (`engine`, `app`, `cli`); `thumbs` ← (`engine`, `app`, `cli`); `capture` (Windows only) ← (`engine`, `app`, `cli`); `engine` ← (`app`, `cli`). `app` also uses `core` and `league` directly. `qa` depends on no Greyscreen crate; `app` uses it only in tests, and its runner starts the `greyscreen` and `greyscreen-cli` binaries.

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
    pub diagnostics: bool,       // Settings.recording_diagnostics: capture also sends Setup and samples CPU and memory
}
pub enum MediaEvent {
    Started { t0: Hns, video: VideoFormat, audio: Option<AudioFormat>, encoder: String },
    Packet(Packet),
    Stats(Box<CaptureStats>),    // about once a second: cumulative counters, see Recording health
    Setup(Box<CaptureSetup>),    // once after Started, only with diagnostics on
    Warning(String),
    Failed(String),  // the session is dead; the engine stops it
}
pub trait MediaSource: Send + Sync {
    fn start(&self, target: CaptureTarget, cfg: CaptureConfig, out: crossbeam_channel::Sender<MediaEvent>)
        -> Result<Box<dyn MediaSession>, String>;
    fn encoder_fingerprint(&self) -> Option<String> { None }                    // GPU names and driver versions
    fn check_encoders(&self, cfg: &CaptureConfig, every: bool) -> Vec<encoder::Attempt> { Vec::new() }
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
Idle/Armed/Cooldown/Starting(game) --Skipped--> Cooldown
```

- `Finalized` goes to Idle, not Cooldown, when `Gone` arrived during Tail or Finalizing, or the recording was manual.
- A failed start returns to Armed and retries `StartMedia` every 5 s, at most 5 times, then reports an error and goes to Cooldown. A failure that time cannot fix (`Problem::retry_helps`: unusable folder, low disk, no encoder, old Windows) reports the error at once. Starting with no `MediaStarted` within 15 s counts as a failed start.
- Cooldown also accepts `WindowFound` with a different pid.
- A recording longer than 3 hours stops.
- `Input::SettingsChanged { auto_record }` turning off stops a game recording and ignores `WindowFound`. In that case the status is `GameRunning` while a game is on. Turning it on starts recording a game that is already running. `Input::Retry` (the Retry button) leaves Cooldown and starts again.

`Effect` values: `StartMedia(CaptureTarget)`, `StopMedia`, `Finalize { result }`, `Highlight(TimedEvent)`, `Status(Status)`. Manual recording (`Input::UserStart`/`UserStop`) is `Idle --UserStart--> Starting` with `PrimaryMonitor`. It is used by the test recording.

## Engine data flow

```
league poller ──LeagueMsg──┐
                           ├─> engine-control thread: Session::step -> effects
media session ─MediaEvent──┤          │
                           │          ├─> writer thread: GameWriter (fMP4 .part, finalize at stop)
                           │          ├─> ClipRing (Arc<Packet> deque)
                           │          └─> ClipPlanner -> clip jobs (thread per clip) -> clips/*.mp4
UI commands ──EngineCmd────┘          └─> UiMsg sink -> AppHandle::update
```

Other engine threads: `thumbnails` (game and clip JPEGs, `UiMsg::Thumbnail`), `storyboards` (seek bar sprite sheets, `UiMsg::Storyboard`, at background priority), `engine-updates`, `engine-feedback`, `engine-export` (one export at a time), and on Windows a global hotkey listener. The library scan, delete, storage cleanup and settings save run on short worker threads.

- `EngineCmd`: `ApplySettings(Settings)`, `StartTestRecording { secs: u32 }`, `StopRecording`, `Retry`, `RescanLibrary`, `DeleteRecording { id }`, `BuildStoryboard { folder }`, `SaveClipNow`, `GlobalAction(Action)`, `CheckForUpdates`, `InstallUpdate`, `SendFeedback(FeedbackDraft)`, `Export(ExportJob)`, `CancelExport`, `Shutdown`.
- `UiMsg`: `Status(EngineStatus)`, `Feed(FeedItem)`, `ClipSaved(ClipEntry)`, `RecordingSaved(RecordingEntry)`, `Library(Vec<RecordingEntry>)`, `Notice { level, text }`, `NoticeWithAction { level, text, action }` (`NoticeAction::Reveal(path)` shows Show in folder, `Link(url)` shows Learn more; the toast stays 12 s instead of 4), `HotkeyStatus { errors }`, `Update(UpdateState)`, `Feedback(FeedbackResult)`, `Thumbnail(PathBuf)`, `Storyboard(PathBuf)`, `Encoder(EncoderCheck)`, `Export(ExportEvent)`.
- `EngineStatus` (`core::session::Status`): `Idle`, `GameRunning` (a game is on and auto record is off), `Waiting`, `Recording { since: Hns, game: Option<GameInfo> }`, `Finalizing`, `Error(String)`.
- Deleting a game moves its folder to the Recycle Bin (Trash on macOS).
- Recovery at start: before anything records, the engine lists the `game.mp4.part` files one folder below the recordings folder. A background worker repairs each one with `greyscreen_mp4::recover` (a finished file is only renamed), then updates `game.json`: `video` and `duration_secs` when they were empty, and `recovered: true`. Then it posts the library (thumbnails follow from the normal thumbnail worker) and one notice: "Recovered N recording(s) that were cut off." The folder of the running recording (`ActiveFolder`) is skipped. Nothing is deleted. If the repair fails, the crate renames the part to `game.mp4.recovered-fmp4` and the engine keeps every file and posts "Couldn't recover <folder name>" with Show in folder (the recordings folder when several failed). A repaired video whose `game.json` is missing or unreadable counts as a failure too, because the library cannot list it. A second start finds no `.part`, so it does nothing and says nothing.
- Low disk guard (`core::disk`, `engine::disk`): `FreeSpace` reads the free bytes of the recordings drive (`fs4`, from the nearest existing parent folder; an unreadable value never blocks). Thresholds live only in `core::disk`: a start needs the larger of 2 GB and 10 minutes at the current video plus audio bitrate; a running recording stops below 500 MB, checked every 10 s on the control thread. A refused start posts "Not enough disk space to record (X GB free). Free up space or change the recordings folder." with Learn more to `https://greyscreen-lake.vercel.app/faq#low-disk` and feeds `MediaFailed` to the session (retries and Retry work as for any failed start; the notice shows once per streak). A stop sends `UserStop`, so the game video is finalized and saved like any stop, then posts "Recording stopped: the disk is almost full (X GB free)...". With the storage limit on, the order is: delete the oldest full-game videos (`retention::plan_free_space`: videos only, oldest first, never the running game, sidecars and clips stay), read the free space again, and only then refuse or stop. With the limit off, nothing is deleted. Deletions are not undone if they were not enough.
- Storage limit: when `storage_limit_gb` is set, the engine checks after each game and at start (and before a start or during a recording when the disk is low, see below). It deletes the `game.mp4` of the oldest games first, together with its storyboard files (the sidecar, thumbnails and clips stay; the storyboard counts as part of the video's size), then whole old games, and never the game being recorded. It posts an info notice.

## Capture frame lifetime (`crates/capture`)

- The WGC frame pool (2 BGRA buffers, `CreateFreeThreaded`) hands each frame to the `FrameArrived` callback. The callback copies the frame's content rect into one of four Greyscreen-owned textures, ends a `D3D11_QUERY_EVENT` query, flushes, and waits up to 12 ms for the query. Only then does it close the frame. WGC does not document whether work that is only queued on our device protects the pool buffer after `Close`, so the copy must be finished first. A copy that is still running at 12 ms skips its frame and counts as `copy_wait_timeouts`.
- WGC `MinUpdateInterval` is `core::media::capture_min_update_interval_hns`: `1/fps - 1/display_hz - 0.5 ms`, at most 0.9/fps, and 0.9/fps when the refresh rate is unknown. DWM delivers on vsync, so a fixed 0.9/fps rounds up to a whole number of refreshes (3 at 144 Hz = 48 fps for a 60 fps target). The chosen interval is logged once at session start.
- The recorder asks Windows for high GPU scheduling priority (`SetGPUThreadPriority(7)` on each device, and the process class `HIGH` once through `D3DKMTSetProcessSchedulingPriorityClass`, never realtime), so capture copies and the encoder are not starved when the game saturates the GPU. A refusal logs a warning and capture goes on.
- Nothing reads a pool texture after its frame is closed. The video thread reads only the owned textures. `core::tear::HoldBack` is the one record of which owned texture is published, which is held and which the video thread reads, and the callback writes only a texture that is none of these (`HoldBack::write_slot`). Four textures make sure that one is always free. The video thread waits until the encoder takes input, then takes the newest published texture, marks it as read, queues the conversion and flushes, and only then releases it. Later copies on the same device run after that conversion. A held frame that becomes stale is published through `HoldBack` too.
- Torn frame check (`core::tear`): the same queued work also copies the frame into a mip-chained texture, runs `GenerateMips` and reads back one mip of at most 128 pixels wide as luma (96x54 for 3072x1728, 120x67 for 1920x1080). Each frame is held back until the next one arrives. A frame is dropped, and the previous frame stays published (`torn_dropped`), when it differs sharply from both its neighbours while the neighbours match each other, either over the whole thumbnail or in one tile of 8x8 thumbnail cells (edge tiles are smaller). The tile rule catches frames where WGC copied the game while it drew its next frame: the HUD (minimap, portraits, ability bar) is missing or a checkerboard for one frame while the moving world is current. A tile counts as torn when at least a quarter of its cells jump both ways and the neighbours differ in at most a quarter as many cells. A held frame with no successor is published after 2.5 frame intervals. This costs at most one WGC frame of latency, one full-size copy and a mip build on the GPU, and about 20 to 32 KB of readback. If the thumbnail textures cannot be made, the check turns off and frames publish at once.
- At start, capture logs once whether the captured frame size matches the window client size. A difference means Windows scales or composes the window (for example, a DPI-virtualised game).

## On disk

```
<output_dir>/                                   default: %USERPROFILE%\Videos\Greyscreen, macOS ~/Movies/Greyscreen
  2026-11-20_19-55-01_Ahri_CLASSIC/
    game.mp4            (game.mp4.part while recording)
    game.json           library sidecar, the source of truth for the UI
    diagnostics.jsonl   only with Detailed recording diagnostics on, one line per second
    thumb.jpg           game thumbnail, made in the background
    storyboard.json     seek bar preview index, written last (see Storyboards)
    storyboard-0.jpg    preview sprite sheets, 10x10 tiles of 160x90 (storyboard-1.jpg, ...)
    clips/
      12-34_MyKill.mp4
      12-34_MyKill.jpg  clip thumbnail
    exports/            files made by Export, never indexed in game.json
      Ahri_12-34_12-58_Original.mp4
%APPDATA%\Greyscreen\settings.json                macOS ~/Library/Application Support/Greyscreen/settings.json
%LOCALAPPDATA%\Greyscreen\logs\greyscreen.log.YYYY-MM-DD, crash.txt
%LOCALAPPDATA%\Greyscreen\cache\champions\       champion icons from Data Dragon
%LOCALAPPDATA%\Greyscreen\cache\sounds\          the sound effect wavs, written from the exe at start
%LOCALAPPDATA%\Programs\Greyscreen\greyscreen.exe    the installed copy (Windows)
```

The sidecar `game.json` (serde, `version: 1`) holds: `id`, `started_at` (RFC 3339), `duration_secs`, `video` (relative path, empty after the storage limit deleted the video), `game` (`GameInfo` + `result`), `events` (`[{ id, game_secs, rec_secs, kind, highlights }]`), `clips` (`[{ file, start_secs, end_secs, label }]`), `encoder` (string), `video_format` (`width`, `height`, `fps`). It may also hold `recovered` (`true` after crash recovery, left out otherwise), `review` (`{ deaths: [{ event_id, tags, note }], filter: [category, ...] }`, tags in snake_case from `core::review::DeathTag`, categories in snake_case from `core::categories::EventCategory`). `review.filter` is the per-game event filter: the categories shown for this game only, left out while the game follows the default in Settings. `review` is left out while nothing is tagged, noted or filtered, and older sidecars without it still read. Unknown category names are skipped. It may also hold `health` (see Recording health), written once when the game is finalized and left out for older recordings.

### Storyboards (seek bar previews)

`core::library::storyboard` holds the pure part: `Storyboard` (serde, `storyboard.json`: `version`, `interval_secs`, `frames`, `tile_width`, `tile_height`, `columns`, `rows`, `sheets`), `Storyboard::plan(duration_secs)`, `interval_for`, `times()` and `tile_at(secs)`. One frame every `ceil(duration / 500)` seconds, at least 1 s and at most 6 s, so a 20 minute game gets 401 frames (every 3 s), a 30 s clip 31 (every 1 s) and a 60 minute game 601 (every 6 s). Frames are 160x90 (16:9 centre crop, like thumbnails), packed row by row into 10x10 JPEG sheets (`storyboard-N.jpg`, quality 70, 1600x900, about 2 MB per game). The index is written last, so its presence means the sheets are complete; `storyboard::read` also checks the index is consistent, its sheet names are plain file names and the sheets exist. A game without a readable index has none.

Generation (`engine/src/storyboard.rs`) runs on the `storyboards` thread, at background priority (`greyscreen_thumbs::lower_current_thread_priority`: QoS background on macOS, `THREAD_MODE_BACKGROUND_BEGIN` on Windows), apart from the `thumbnails` thread so a long build never delays thumbnails. It starts for a game when `RecordingSaved` arrives (after the finalize) and when the review opens a game that has none (`EngineCmd::BuildStoryboard`, sent by `Ui::open_review_entry`; a game whose video is gone or shorter than 1 s is skipped). It never runs while a game records: requests wait (the worker checks `ActiveFolder` every 2 s) and a running build stops at the next frame, deletes its partial sheets and starts again later. A deleted folder drops the request. A failed build is not retried until the next start. `greyscreen_thumbs::write_sheets` decodes one frame per time through the OS decoder with one open file: macOS reuses one `AVAssetImageGenerator` with a ±0.5 s tolerance, so each request lands on the nearest key frame (recordings have one every second); Windows opens one `IMFSourceReader` once and takes the first sample after each seek, which is the key frame at or before the time. A frame that fails to decode repeats the tile before it. `UiMsg::Storyboard(folder)` tells the UI, which reloads the index if that game is open.

## Settings (`core::settings`, serde with `#[serde(default)]`)

| Field | Type | Default |
|---|---|---|
| `output_dir` | `PathBuf` | Videos/Greyscreen (Movies on macOS) |
| `auto_record` | `bool` | `true` |
| `record_full_game` | `bool` | `true` |
| `quality` | `Quality { Balanced, High, Max }` | `High` |
| `resolution` | `Resolution { Native, Height(u32) }`, height clamped to 144 to 4320 (old `P1440`, `P1080`, `P720` values still load) | `Height(1080)` |
| `fps` | `u32` (30 or 60) | `60` |
| `audio` | `bool` | `true` |
| `capture_cursor` | `bool` | `true` |
| `recording_diagnostics` | `bool`, "Detailed recording diagnostics" (Settings > General). Takes effect on the next recording | `false` |
| `sound_effects` | `bool` | `true` |
| `sound_volume` | `u32` percent, 0 to 100 | `60` |
| `clips.enabled` | `bool` | `true` |
| `clips.pre_secs` / `clips.post_secs` | `u32`, 1 to 120 | `15` / `5` |
| `clips.kills`, `deaths`, `multikills`, `objectives`, `aces` | `bool` | `true` |
| `clips.assists`, `structures` | `bool` | `false` |
| `review_events` | set of `EventCategory`, saved as a list of snake_case names, unknown names skipped | everything except `teammate_kills`, `enemy_kills`, `other_multikills` and `game_flow` |
| `keybinds` | map of action id to key text, only the changed ones | empty |
| `storage_limit_gb` | `Option<u32>`, snapped to 50, 100, 250 or 500 | `None` (no limit) |
| `check_updates` | `bool` | `true` |
| `nav_style` | `NavStyle { Sidebar, Dock }` | `Sidebar` |
| `sidebar_collapsed` | `bool` | `false` |
| `review_inspector_hidden` | `bool` | `false` |
| `games_list_hidden` | `bool` | `false` |
| `nav_width` | `u32` points, 180 to 320 | `220` |
| `list_width` | `u32` points, 240 to 440 | `300` |
| `window_size` | `(u32, u32)` points or none, 1000x600 to 8192x8192: the size the window returns to when not maximized | none (opens at 1280x760) |
| `window_maximized` | `bool` | `false` |
| `client_id` | `Option<String>` | `None` (a random UUID is stored the first time feedback is sent) |
| `first_run_done` | `bool`, set by Done on Home's first-run card. A settings file from before the field counts as done; no settings file means first run | `false` with no file, `true` for an old file |

An old `save_clip_hotkey` key is read once and moved into `keybinds`. It is never written back. `Settings::validate` clamps every range above when settings load.

Bitrate: the mean bitrate at 1080p60 is 8, 12 or 20 Mbps for Balanced, High or Max. It scales with pixel count to the power 0.75 and with fps/60 to the power 0.8. Peak is 2.0 times the mean. `CaptureConfig::for_encode_size` rescales with the same 0.75 exponent when the real encode size is smaller than `bitrate_pixels`.

Quality: `CaptureConfig.quality` is `Some(60)`, `Some(70)` or `Some(80)` for Balanced, High or Max. With `Some(q)` the encoder uses Quality rate control with `AVEncCommonQuality = q`, and mean and peak only cap through `MaxBitRate` and the buffer size. If the encoder rejects or misreads either value, it falls back to peak-constrained VBR at the mean and peak, and logs one warning. `MediaEvent::Started.encoder` is the encoder label, the MFT name and the mode, for example `NVIDIA NVENC (hardware) · NVIDIA H.264 Encoder MFT (vendor VEN_10DE) (quality 70)` or `Windows software encoder · Microsoft H.264 Encoder MFT (vbr 18.5 Mbps)`. `Settings::estimated_mb_per_minute` returns 0.6 times the mean bitrate plus audio, for the size line in the Recording settings (Native counts as the height of the tallest connected display). The 0.6 factor is a placeholder until real recordings give numbers.

`greyscreen-cli record` takes `--preset`, `--rc quality|vbr`, `--quality N`, `--bitrate MBPS`, `--peak MBPS`, `--fps`, `--height H`, `--no-audio` and `--cursor` to override the config for A/B tests.

## Keybinds

Every key action is one line in `actions!` in `crates/core/src/keybinds/action.rs`, with an id, label, group, scope and default keys. `Mod` means Ctrl on Windows and Cmd on macOS. Global scope works in game through `RegisterHotKey`; App scope only works in the window. Defaults:

| Group | Action (keys) |
|---|---|
| Recording | Save clip (F9, Global) |
| Review | Play or pause (Space), Seek back 5 s (Left, J), Seek forward 5 s (Right, L), Previous event (Shift+Left), Next event (Shift+Right), Frame back (,), Frame forward (.), Slower (\[), Faster (\]), Mute (M), Review deaths (D), Next death (N), Previous death (P), Replay death (R), Show or hide the events panel (I), Show or hide the games list (G), Toggle theater mode (T), Toggle fullscreen (F), Go back or close the game (Escape, fixed) |
| General | Open settings (Mod+,), Go to Home (Mod+1), Go to Games (Mod+2), Go to Clips (Mod+3), Previous game (Up), Next game (Down), Focus the player (Enter), Collapse or expand the navigation (Mod+\\), Search games (Mod+F), Send feedback (no key) |

The Keybinds settings page lists every action, rebinds on click, and refuses a key that another action uses. A Global key that Windows refuses shows an error next to the action.

## UI model (`crates/app`)

`Ui` holds the state and `view::root` draws it. `Ui::apply(UiMsg)` is the only way engine messages change it. Rune scenes (`--snap out.png --scene NAME`) render the screens headless, and the `Runtime` tests drive the same code.

- **Screens.** `Screen` is `Home`, `Games` or `Clips`. Settings is a sheet over the current screen, not a screen. There is no Recordings page.
- **Home.** A state card (Idle, Off, Waiting, Recording with a clock, Finalizing, Error, with Save clip, Retry, Stop and "Test recording (10 s)"), then the last game, recent clips and a week summary. While `first_run_done` is off, a kit card below the state card walks through three steps: the recordings folder (Change… opens the folder picker), Quality (the Settings control) and a 10-second test recording that ends with "It works" and the clip, or a note that points at the error on the state card. Done hides it for good and saves the setting. While it shows, the state card has no test button of its own. While recording, Home shows the live feed of events.
- **Setup errors.** `Status::Error(raw)` carries the raw text of a failure. `core::problem::Problem::from_error` sorts it into a plain-language problem with a title, the one thing to do, an optional second hint, a short sidebar and dock line, and a FAQ anchor. Home's state card shows title and action, with Retry, Learn more (opens the FAQ) and, for an unusable folder or a full disk, Choose folder…; an unknown failure also shows the raw text and Open logs folder. The sidebar status block and the dock's pill and tooltip show the same problem. The engine checks that the recordings folder can be created and written before it starts capture, and its notices use the same plain text for a known problem (`Problem::notice`).
- **Tray.** `crates/app/src/tray` owns a Rune tray icon. Its menu is Open Greyscreen, Pause recording or Resume recording (the Record games switch), Save clip (enabled only while recording) and Quit; a left click opens the window. The tooltip is "Greyscreen — Recording 12:34", "Waiting for League", "Recording paused", "League is loading", "Saving" or the problem's title. The icon is `tray-recording` while recording. `Ui` sends a `TrayState` to the tray thread on every status or settings change; the thread rebuilds the icon only when the recording flag flips and updates the tooltip once a second while recording (Rune's tray has no icon setter).
- **Games.** The games list (search by champion, mode or result, grouped by day, right-click menu with Open, Review deaths, Show in folder and Delete) is the first column inside the content card, then a hairline divider, then the review player with its flush events and clips panel separated by a vertical hairline divider. While reviewing a game, the games list (`G`) and events panel (`I`) can each be toggled independently from the toolbar or keyboard (persisted in Settings); the games list also hides automatically during Death review. When playing a clip, the `Playing clip` notice with the `Full game` button sits above the video player so it remains visible when the events panel is hidden. Games and the list can be picked with Up and Down.
- **Clips.** Recent clips across all games. Each card has an Export icon that opens the export dialog.
- **Death review.** Review deaths (D) plays each of the user's deaths in turn, from 12 s before to 3 s after. A switch slows the last 4 s to half speed. Each death takes tags and a free note, saved to `review` in the sidecar. A summary follows the last death.
- **Review player on Windows.** The review plays the video's own sound on every platform. Rune's `rune-media` is vendored in `vendor/rune-media` with Windows fixes (`vendor/rune-media/GREYSCREEN_PATCHES.md`): its sound is read ahead on a reader of its own and leads the clock, pause on the compositor layer shows the layer's picture and time, and a seek past the last picture shows that picture. So the app pauses with a plain pause and lets a seek reach the clip's end. The Windows player also seeks with its decoder's inbox free, so a seek never blocks the UI thread, starts a wedged decoder again after 2 s of waiting for a picture, and has a fast seek (`Player::seek_fast`) that shows the keyframe at or before the time. A layer the decoder makes again (after a seek or a pause on the layer) never reuses a closed layer's surface handle value, since Rune's window keys its visual on that value and would keep showing the old picture.
- **Seek bar preview** (`ui/review/seek_preview.rs`, `view/review/seek_preview.rs`). Hovering the seek bar (pointer moves are read in window points, like the drag) or dragging the thumb shows a kit popover card (`popover::raised`, anchored above the pointer through an invisible element on the bar) with the storyboard tile of the nearest time and the time under it. While dragging, the card shows the drag's target. A tile is the sheet image (loaded through the thumbnail `ImageCache`) pinned at the tile's offset inside a clipped 160x90 frame, so a hover decodes nothing. Without a storyboard, or for a clip, the card shows only the time. A hovered marker keeps its event card and hides the preview. The card has no animation of its own beyond following the pointer.
- **Scrubbing** (`ui/review/scrub.rs`). A drag on the seek bar or from a timeline marker never seeks per pointer move. The thumb and the time follow the pointer at once (`ReviewState::time` reports the drag's target). A clip that played pauses on the first poll of the drag. The player gets at most one seek in flight: a fast keyframe preview at most every 100 ms, only when the pointer moved since the last one. A pointer held still 250 ms gets one exact seek. The release does one exact seek to the final spot (unless the rest already did) and plays again if the clip played before the drag. A click is a release without a poll between: one exact seek and no pause.
- **Playback logs.** Start Greyscreen with `GREYSCREEN_VIDEO_LOG=1` (quit the running copy first; in PowerShell: `$env:GREYSCREEN_VIDEO_LOG=1; & "$env:LOCALAPPDATA\Programs\Greyscreen\greyscreen.exe"`). Every player call (play, pause, seek, rate, volume, mute, with the player's time and state) then goes to `greyscreen.log` at debug level under the `playback` target. On Windows, Rune's own log (`RUNE_VIDEO_LOG=frames`: phases, layer steps, every pull) goes to `rune-video.log` in the same logs folder. Without either, the Windows player still logs a few lines per seek or pause to `greyscreen.log` at info: each phase change, each seek, wedged-seek restarts, long decoder waits, and a `picture frozen` warning when its clock runs over 1 s with no new picture.
- **Playback stall report** (`ui/review/trace.rs`, always on). Every review player is wrapped in `Traced`, which notes each call (with the player's time and state) in a ring of the last 200 entries on the UI thread; `Watch` notes each change of the player's state and of `seeking()`. Each poll, `Watch::check` looks for a stall: the clip should play (not scrubbing, no seek in flight) but its time stands still for over 1 s, or a seek waits over 1 s for its picture. A stall writes one `playback stall:` warning to `greyscreen.log` with what stalled, the time asked for, how long so far, the file and the ring, and a `playback stall over after N s` line when it ends. Nothing is logged per frame. A window nobody draws (minimised) can report one stall, as its time stands still. `GREYSCREEN_VIDEO_LOG` stays for the full detail.
- **Nav styles.** `NavStyle::Sidebar` has Home, Games and Clips, the five newest games (hidden while on Games), an engine status block, a storage block and footer buttons for Send feedback and Settings. The nav sidebar's edge and the list/review divider drag to resize; double-click a handle resets its default, and the widths save when the drag ends. The sidebar folds to a 56-point rail when `sidebar_collapsed` is set or the window is under 1200 points wide; the rail is not resizable. `NavStyle::Dock` removes the sidebar and shows a floating pill at the bottom: record status, Home, Games, Clips, Settings, Send feedback. While a game plays in review the dock slides away until the pointer nears the bottom edge.
- **Recording health.** The review toolbar's `•••` overflow menu (`review-more`, with a warning or danger dot when findings are degraded) has a "Recording health" item that opens a kit Popover titled "Recording health" anchored to `review-more`. It shows `RecordingHealth::summary_line()` (for example `59.8 fps · 0.3% dropped · audio OK · 18 Mbps`), then one row per `RecordingHealth::findings()`: a kit tone pill (success, warning, danger for OK, WARNING, BAD), the area and the detail. A recording without `health` shows "No health data (recorded before 0.1.12-preview.14)". Pressing outside or the close button closes it.
- **Event filter.** `core::categories` sorts every event into one `EventCategory` from its kind and whether the local player is involved (`Player::answers_to`): my deaths, kills and assists (in that order when more than one applies), teammates' kills and enemies' kills (kills that do not involve me, by the killer's team, or the victim's team when the killer is not a player), my multikills and others' multikills, aces, dragons, barons, heralds and void grubs, towers and inhibitors, game flow (game start, minions spawning, first blood, first brick, inhibitor respawns, unknown events) and game end. The shown set is `Settings.review_events`; a game's `review.filter` overrides it for that game. It applies to the events list, the timeline markers and Previous/Next event. Death review always uses all my deaths. The Filter menu in the events panel lists every category with a check, plus "Save as my default" (writes the game's set to Settings and drops the override) and "Use my default" (drops the override). A badge shows while the game's set differs from the default. Ticking a category in the menu saves the override at once.
- **Sound effects** (`crates/app/src/sfx`). Four short sounds from the `soft` pack of Hex (MIT, see `crates/app/assets/sounds/README.md`): recording started, recording saved, clip saved and failed. `Ui::apply` hands every `UiMsg` to `Sfx::on_msg` first, so they play while the window is hidden in the tray. Recording started is a `Status` change into `Recording`. Recording saved is `RecordingSaved`. Clip saved is `ClipSaved` for a manual clip only (`MANUAL_CLIP_LABEL`), so highlight clips stay silent. Failed is an `Error` notice, an `Error` status or a failed feedback send, and a burst of them makes one sound (2 s gap). Nothing plays with `sound_effects` off or `sound_volume` at 0. The wavs are embedded; Rune's audio player opens files, not memory, so a background thread writes them to `<cache>/sounds` at start (repairing stale copies), and each play makes a `Player::audio` clip at the volume, dropped when it ends or after 3 s. A missing file, an unwritable cache or a clip that fails to load only logs. Dragging the volume slider saves on release and plays the clip sound once.
- **Settings sheet.** Five sections with labeled sub-group cards and a search box in the left nav: General (App: Start with Windows on Windows and Navigation style; Sound: Sound effects and Sound volume; Updates; Diagnostics: Detailed recording diagnostics and Open logs folder; About: Version and Riot notice), Recording (Capture, Video, Audio & input), Clips & Review (Highlight clips and Review timeline defaults with Reset to default in the group header), Storage (folder and limit, showing the progress bar only when a limit is set) and Keybinds. Every change updates `ui.settings` and sends `ApplySettings`; Start with Windows writes the registry directly.
- **Setup window** (`crates/app/src/setup`). A fixed 460 by 420 window titled "Greyscreen Setup", with its own `Phase` state (`Ready`, `Closing`, `Copying`, `Done`, `Failed`). Windows only starts it when `greyscreen.exe` runs from outside the install folder and no installed copy matches it. It installs to `%LOCALAPPDATA%\Programs\Greyscreen`, with a Start with Windows option, a Start Menu shortcut and an Installed apps entry; the user can also run it without installing. An update launch (`--update`) or a run that matches the installed copy installs silently instead. `GREYSCREEN_NO_INSTALL` skips all of this. `--hidden` starts the app without a window.
- **Update banner.** See Updates.
- **Feedback dialog.** See Feedback.

## Help anchors

The website's FAQ page (`https://greyscreen-lake.vercel.app/faq`) needs one section per anchor below. The app links each failure to `/faq#<anchor>` (`greyscreen_core::problem::Problem::anchor`).

| Anchor | Shown when | What the FAQ should say |
|---|---|---|
| `fullscreen` | Capture got no frames from the League window | Switch League to Borderless in its video settings. If League runs as administrator, run Greyscreen as administrator too |
| `window-not-found` | The League window closed or was not found while capture started | Make sure League is running, then retry |
| `folder-not-writable` | The recordings folder is missing, read-only or cannot be created | Choose another folder in Settings > Storage |
| `low-disk` | The drive has no space left | Free some space or choose another folder |
| `no-encoder` | No hardware or software H.264 encoder or GPU was usable (capture start, or the encoder check at start) | Update the graphics driver and restart Greyscreen |
| `windows-version` | Windows Graphics Capture is not supported | Update to Windows 10 version 1903 or newer |
| `run-as-admin` | Windows refused access to the window (access denied) | Run Greyscreen as administrator if League does; check Settings > Privacy & security > Screen capture |
| `help` | Any other capture failure | The Get help section: send feedback, GitHub Issues, logs |

On Windows 10 builds that cannot hide the capture border, a start warning says Windows draws a yellow border around the captured window and that it is not in the video. It has no anchor.

## Encoder selection (`core::encoder`, `capture::selftest`)

- Plan (`encoder::plan`, pure): hardware MFTs first, one per GPU, then Windows' software H.264 MFT (`CLSID_MSH264EncoderMFT`), then fail. GPUs come in this order: on a hybrid laptop (the GPU that owns the game's display is Windows' minimum-power GPU and another GPU is its high-performance one) the high-performance GPU first, because games run there; then the display's GPU; then the others in DXGI order. Microsoft adapters (Basic Render Driver, remote sessions) get no hardware try. The software encoder runs on the display's GPU, or the first adapter when no GPU owns it.
- Capture builds the whole pipeline (device, WGC, video converter, encoder) on the plan's GPU. When every encoder on a GPU fails, it stops that WGC capture and starts over on the next GPU. Setup fails with all the encoder reasons, or with the first setup error (for example no frames from the window) when no encoder was tried.
- The software encoder is synchronous. Each converted NV12 frame is copied to a staging texture, mapped and copied into a system-memory sample (about 90 MB/s at 1080p 30). Settings that keep CPU use down: `encoder::software_config` caps the encode at 1080p and 30 fps (GOP = fps), `AVEncCommonQualityVsSpeed` 0 and `AVEncNumWorkerThreads` at half the cores, 2 to 4. Rate control and the other properties are the hardware ones.
- Labels (`encoder::label`): `NVIDIA NVENC (hardware)`, `AMD AMF (hardware)`, `Intel Quick Sync (hardware)`, `Qualcomm (hardware)`, else `<adapter name> (hardware)`, and `Windows software encoder`. The label starts `MediaEvent::Started.encoder`, so the log, the recording health and `diagnostics.jsonl` name the encoder.
- Check at start: the app calls `greyscreen_engine::spawn_encoder_check` once the engine runs. After 2 s, a background thread computes the key (app version, `encoder_fingerprint`: every adapter's name, vendor and driver version, and the user's fps, height and quality). When `<cache>/encoder-check.json` holds an `EncoderCheck` with that key and an outcome other than `failed`, it is used as is. Otherwise `check_encoders(cfg, every: false)` runs the plan on the primary display's size: each candidate converts a synthetic BGRA frame and encodes 10 frames (no screen capture), and the first that returns a key frame wins. The result (`outcome`: `hardware`, `software` or `failed`, `encoder` label, `detail` with every failure, `noticed`) is saved and sent as `UiMsg::Encoder`. A software outcome also sends one Warn `NoticeWithAction` ("Your graphics card's video encoder didn't work, ...", Learn more links to `/faq#no-encoder`) and saves `noticed: true`, so it shows once per key.
- UI: Settings, Recording has a read-only Encoder row (`encoder-row`) once a check came back: `NVIDIA NVENC (hardware)`, `Windows software encoder — uses more CPU` or `None works`. A failed check shows a danger card on Home (`encoder-error`): what to try (update the graphics driver, restart), Read the FAQ (`Problem::NoEncoder.help_url()`, `/faq#no-encoder`) and Send feedback.
- Mid-recording failure: an `MEError` event from the MFT, an encoder step that fails 120 ticks in a row, or no packets for too long ends the session with `MediaEvent::Failed("the video encoder stopped: ...")`; a removed device ends it with `graphics device ...`. Capture drops the dead encoder without draining it. The engine finalizes the game up to that point as for any capture failure and shows the Error notice "Recording stopped: the video encoder or graphics driver failed. The game is saved up to that point." (`encoder::is_encoder_failure`).
- `greyscreen-cli doctor` runs `check_encoders(cfg, every: true)` with the user's settings: every candidate in plan order, one line each, then the chosen encoder. It exits non-zero when none works.

## Recording health and diagnostics

Capture counts what it already knows. The video thread sends `MediaEvent::Stats(CaptureStats)` about once a second. Every field is cumulative since the start, except the two `window_*_max_us` gauges, which cover the last second:

- `torn_dropped` (frames the torn frame check dropped), `copy_wait_slow` (copies that took 4 ms or more to finish on the GPU), `copy_wait_timeouts` (copies that did not finish in 12 ms; their frames were skipped) and the per-second `window_copy_wait_max_us`.
- `ticks`, `captured` (WGC frames the game window delivered), `repeated` (ticks that re-encoded an unchanged frame), `dropped` (ticks lost because the encoder was not ready or a step failed), `skipped` and `resyncs` (ticks jumped over after a stall), `soft_errors`.
- `encode` (a fixed 100-bucket `LatencyHistogram`, 0.5 ms each) is the time from submitting a frame to the encoder until its packet is read. The thread reads packets at its next poll, so a value near one frame interval means the encoder keeps up, and a value above two frame intervals means it lags. `window_encode_max_us` and `window_step_max_us` (the longest pass of the capture loop) are the per-second maxima.
- Audio, from atomics the audio thread owns: `audio_packets`, `audio_discontinuities` (WASAPI data-discontinuity flags), `audio_gaps` with `audio_gap_longest_ms` and `audio_gap_total_ms` (silence filled because a packet arrived later than its slot; the first packet's padding from `t0` does not count), `audio_idle_ms` (silence filled by timeout while the game made no sound).
- Gauges: `in_flight` (frames inside the encoder), `pcm_queue` (PCM blocks waiting for AAC), and with diagnostics on `cpu_pct` (Greyscreen's process CPU, 100 is one core) and `mem_mb` (working set).

No frame is allocated or logged for this: the counters are integer adds on the capture thread, the histogram is a fixed array, and the snapshot is one boxed copy per second. The 10-second log line stays.

The engine's `HealthTracker` (`greyscreen-core::health`) sees every packet and every `Stats`. It derives from the packets: encoded frames, missing frames (the dts holes, so `dropped` is exactly what the file lacks), the longest gap between video packets, audio dts gaps, the audio start offset against the first video frame (the writer drops audio that ends before the first key frame), and the audio minus video duration at the end (drift). `finish` builds `RecordingHealth`, which `game.json` stores as `health` (all fields `#[serde(default)]`, so partial or older files read):

| Field | Meaning |
|---|---|
| `target_fps`, `width`, `height`, `duration_secs` | the encode format and the video track length |
| `frames_expected` | encoded plus dropped |
| `frames_captured`, `source_fps` | frames the game window delivered, and that count per second |
| `frames_encoded`, `frames_dropped`, `frames_repeated` | encoded, missing (dts holes), and ticks that re-encoded an unchanged frame |
| `dropped_pct`, `repeated_pct` | of `frames_expected` and of `frames_encoded` |
| `longest_video_gap_ms` | the longest time between two video packets (16.7 ms at 60 fps is normal) |
| `encoder` | the encoder label, MFT name and rate-control mode (`MediaEvent::Started.encoder`) |
| `encode_ms` (`avg`, `p95`, `max`) | the encode latency above, left out when capture sent no timing |
| `audio` (`packets`, `gaps`, `longest_gap_ms`, `discontinuities`, `idle_silence_ms`, `start_offset_ms`, `drift_ms`) | `gaps` counts audio dts holes plus filled gaps; `drift_ms` is audio minus video track length; `start_offset_ms` is the first kept audio packet against the first video frame; left out without an audio track |
| `file_bytes`, `avg_bitrate_kbps` | the finalized file size and its average bitrate (video and audio) |
| `capture_errors`, `capture_soft_errors` | `MediaEvent::Failed` count while recording, and recovered capture step failures |
| `frames_torn_dropped`, `copy_wait_slow`, `copy_wait_timeouts` | the capture frame counters above; a Capture finding shows when frames were dropped (OK) or copies timed out (WARNING) |

Verdicts (`greyscreen-core::health::verdict`, shared by the popover and the analyzer) are OK below the first number, WARNING from it and BAD from the second: dropped frames 0.5% and 2%, repeated frames 5% and 20%, longest video gap 100 ms and 500 ms, audio gap 50 ms and 250 ms, audio drift 50 ms and 250 ms, audio start offset 50 ms and 250 ms. Encode p95 above one frame interval warns and above two is bad. Any capture error is bad, any soft error warns, and an average bitrate under 1 Mbps at 720p or more over 30 s warns.

### Detailed diagnostics (`diagnostics.jsonl`)

With `Settings.recording_diagnostics` on, `CaptureConfig.diagnostics` is true. Capture then sends one `MediaEvent::Setup(CaptureSetup)` after `Started`, and fills the CPU and memory gauges. The engine opens `diagnostics.jsonl` in the game folder. A `diag-writer` thread owns the file behind a `BufWriter` and a bounded channel: the control thread only does `try_send`, and a full queue drops the line, so a slow disk never stalls recording. Every line is one JSON object with a `type`:

- `setup` (first line): `adapter`, `driver_version` (the DXGI user-mode driver version, with the NVIDIA panel number in brackets), `display_width`, `display_height`, `display_hz` (the monitor's current mode), `window_width`, `window_height` (the game's client area), `frame_width`, `frame_height` (the captured frame), `crop_left`, `crop_top`, `crop_width`, `crop_height`, `encode_width`, `encode_height`, `fps`, `encoder`, `bitrate`, `peak`, `quality`, `gop_frames`, `audio`.
- `second` (one per second): `t` (seconds since capture start), `captured`, `encoded`, `dropped`, `repeated` (all counts for that second), `encode_ms_max`, `step_ms_max`, `torn` (frames dropped by the torn frame check), `copy_wait_ms_max`, `audio_packets` (total), `av_skew_ms` (audio end minus video end as the engine has received them), `queue_writer` (packets waiting for the file writer, of 4096), `queue_media` (events waiting for the engine), `queue_encoder`, `queue_pcm`, `cpu_pct`, `mem_mb` (both absent when Windows refuses the read), `foreground` (true when the captured game window, or a window with the same root owner, is the foreground window when the second is sampled; absent for monitor capture), `gpu_3d_pct`, `gpu_3d_self_pct`, `gpu_encode_pct`, `gpu_copy_pct` (absent when the counter is unavailable).

GPU load comes from the PDH counter `\GPU Engine(*)\Utilization Percentage`, read once per second by a `gpu-usage` thread that only publishes the latest values; capture reads them without waiting, so PDH never runs on the capture or encode path. Only engines of the capture adapter's LUID count (`core::health::gpu` parses the `pid_..._luid_..._engtype_...` instance names). Per engine, utilisation is summed across processes (the Task Manager method): `gpu_3d_pct` is the busiest 3D engine, `gpu_3d_self_pct` is Greyscreen's own share of it, `gpu_encode_pct` the busiest VideoEncode engine, `gpu_copy_pct` all Copy engines together; each is capped at 100. If the query cannot open, capture logs one warning and the fields stay absent.

### `greyscreen-cli analyze`

`greyscreen-cli analyze <recording folder or mp4> [--json]` needs no ffmpeg. `greyscreen_mp4::inspect` reads only the index: the full `moov` at the end of a finalized file, or the fragment headers of an unfinished `game.mp4.part` (which also reports complete fragments and cut-off trailing bytes). It then reports, per area with OK, WARNING or BAD:

- File: finalized or truncated, size, resolution.
- Video frames: real frame count, average fps against the target, missing frames, a histogram of frame intervals (1, 2, 3, 4 to 5, 6 to 10 and more than 10 frames).
- Video gaps: every gap above two frame intervals, with its timestamp and the frames it lost (the 20 longest are listed).
- Keyframes: count and interval (min, average, max), bad without any, warning above 5 s.
- Bitrate: average, min and max over 10 s windows (all windows in `--json`).
- Audio continuity: packets, samples against the timeline, gaps with timestamps. A/V duration: audio minus video at the end.
- From `game.json`: every `health` finding. From `diagnostics.jsonl`: seconds with drops, slowest encode and capture step, deepest queues, CPU and memory peaks, the seconds the game was not the foreground window and how many of the seconds with drops fell in the background against the foreground (also in `--json`; omitted from the text when the log has no `foreground`), GPU 3D busy (average, peak, seconds at 95% or more), Greyscreen's own 3D share, encoder engine load, and how many of the seconds with drops had 3D at 95% or more (also in `--json` as `gpu`; omitted when the log has no GPU fields), the setup facts.

The analysis is one pass over about 400 thousand sample rows, well under a second for a one-hour file; a test checks that.

## Clip export

Sharing a clip is the core loop, so export is one small kit Dialog (`crates/app/src/view/overlays/export.rs`, state in `ui/export.rs`). It opens from the Export icon on a clip row (review, Clips tab), on a Clips screen card, and from the review toolbar ("Export this moment…", only while the full game plays). A clip opens on its own range. A moment opens on 15 s each side of the playhead (`ExportRange::moment`, clamped to the game, keeping its length at the edges). The trim bar is a custom control in the seek bar's style with a start and an end handle: it spans the range plus 30 s each side (`ExportRange::bounds`, inside the game), the clip's own span shows as a faint band, handles move in whole seconds, stay 1 s apart and stay inside the bounds. When the game video is gone (storage limit), the clip file is the source and the bounds are the clip itself; times map to the file through `source_offset_secs`.

Two presets (`ExportPreset`), one destination choice (this game's `exports/` folder, or Downloads):

- **Original quality** never re-encodes. `greyscreen_mp4::trim` reads the sample tables of the finished MP4 (`demux::Movie`), starts on the last key frame at or before the start (GOP is 1 s, so up to a second early), keeps the audio that overlaps and writes a faststart MP4 with the clip writer. The dialog says it snaps to a keyframe, and the "Exported" state says how early the file starts. It works on every platform.
- **For Discord (under 10 MB)** re-encodes. `core::export::plan_discord` takes 90% of 10,000,000 bytes, subtracts 128 kbps of AAC and spreads the rest over the duration. Above about 258 s it refuses (`PlanError::TooLong`, the dialog names the limit and Export stays off), below 120 kbps of video there is no plan. Height steps by video bitrate: 720p from 700 kbps, 480p from 350 kbps, else 360p, never above the source, width from the source aspect, both even. 60 fps only when the source is 50+ fps and video has 3.5 Mbps, else the source rate capped at 30. After encoding, a file over 10,000,000 bytes gets one retry (`retry_plan`) that scales the video bitrate so the file would land at 80% of the limit, and the dialog says "Making it smaller…". A second miss fails with "Try a shorter clip". The finished state shows the real size.

`EngineCmd::Export(ExportJob)` runs on the `engine-export` thread (`engine/src/export.rs`), one export at a time. It writes `<stem>.mp4.part` in the destination, renames to `<stem>.mp4` (or `<stem> (2).mp4`) when done and deletes the partial file on failure or cancel. `CancelExport` sets a flag that the cut checks before the rename and the encoder checks on every sample. `UiMsg::Export` carries `Progress`, `Shrinking`, `Done`, `Failed` and `Cancelled`. Closing the dialog while it works asks for a cancel and shows "Cancelling…" until the engine confirms; a second close leaves anyway. The engine takes a `Reencode` trait, so tests drive the whole flow (size retry, cancel, failures) with a fake encoder and a real fixture MP4.

The Windows encoder is `greyscreen_capture::transcode` (compile-checked on the Mac, not run there). It opens the source with an `IMFSourceReader` (NV12 out, advanced video processing as in the thumbnail code), seeks to the start, and reads video and audio interleaved. Video frames before the start are dropped, and so are frames closer than one output frame interval (60 to 30 fps). Each kept frame is cropped to the display aperture and scaled to the plan size on the CPU (`core::nv12::scale`, bilinear, tested on the Mac) and written to an `IMFSinkWriter` MP4 sink with an H.264 High output type at the plan bitrate and AAC-LC 128 kbps. Audio is decoded to PCM by the reader and re-encoded by the writer; a source with another sample rate or more than two channels exports without sound. The writer asks for hardware transforms first. Any failure with hardware on removes the partial file and runs the whole export again with the software encoder. On other platforms the Discord preset is disabled with a note, and the engine answers "Only Original quality works on this computer".

The finished dialog shows size and length, "Show in folder" (`reveal`) and "Copy file". Copy puts the file on the clipboard as a file drop: on Windows `greyscreen_capture::clipboard::copy_file` writes a `CF_HDROP` `DROPFILES` block, on macOS `osascript` sets the clipboard to the file, so it pastes into Discord. Headless QA and unit tests never touch the real clipboard or open Explorer.

## Updates

`greyscreen-league::updates` lists the GitHub releases of `andresmarpz/greyscreen` and downloads `greyscreen.exe`, checking size, the SHA-256 `digest` GitHub reports (skipped when missing) and the `MZ` header. `greyscreen-core::update` holds the pure rules: a build that is a prerelease sees previews and stable releases, a stable build sees stable only, and the newest newer release wins (`preview.N` orders by N, a stable release beats its previews). The running version is the workspace version, so it must equal the release tag without the `v` (`just release` enforces it).

The engine's update thread checks 10 s after start and every 6 hours when "Check for updates automatically" is on, only for the installed copy on Windows, and it waits 60 s and tries again while a session is active. A manual check works anywhere. An installed copy downloads to `%TEMP%\Greyscreen\greyscreen-<tag>.exe`; "Install and restart" launches that file, and the existing installer path asks the running copy to quit and replaces it. A copy that is not installed only links to the release page. `UiMsg::Update(UpdateState)` drives the About card, and `Ui::update_available` holds the pending release for other screens. A downloaded update also shows a toast-style banner at the top centre of every screen and both nav styles, with "Restart to update" and "Later" (hidden until a newer version); it stays hidden while the engine is waiting, recording or finalizing, and in theater mode.

## Feedback

"Send feedback" (nav footer, rail, dock, Settings > Updates & About, and the unbound App action `send_feedback`) opens a dialog: kind (Bug, Idea, Other), message (required, 5000 characters), optional email, and "Include recent logs" (on by default). `EngineCmd::SendFeedback(FeedbackDraft)` runs on its own `engine-feedback` thread, which builds the `FeedbackReport` (`greyscreen-core::feedback`), sends it through `FeedbackTransport` (`greyscreen-league::FeedbackClient`, `ureq`) and answers with `UiMsg::Feedback(FeedbackResult)`. A failure carries the same report as JSON for the "Copy report" button.

`POST {API base}/v1/feedback` with header `x-greyscreen-version` and body `{kind: "bug"|"idea"|"other", message, email|null, app_version, os, log_tail|null, client_id}`; answers 202 `{id}`, 400 `{error}`, 429. The base is `greyscreen_core::api::BASE_URL` (`https://api.greyscreen.app`), or the `GREYSCREEN_API_URL` environment variable when it is not blank, without a trailing slash. The backend is `apps/api` (see its README). `log_tail` is the last 300 lines of the newest `greyscreen.log.*`, with the home folder replaced by `~` and the values of `summoner`/`riotId` style fields redacted, and is `null` when the box is not ticked. `client_id` is `Settings.client_id`, not tied to any identity.

## Rules for code

They live in `AGENTS.md`.
