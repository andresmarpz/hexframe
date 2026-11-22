# QA

The rule: QA never uses OS-level input (no SendInput, SetCursorPos, UI Automation), never shows or activates a window, never plays sound out loud and never changes system settings. It runs while someone uses the PC and they notice nothing. Driving the real app by hand on a person's PC is not QA.

## Layer 1: headless UI (`just qa`)

What Rune gives: any of `--snap`, `--tree`, `--script` makes `App::run` headless. It renders offscreen on the GPU with a virtual clock: no event loop, no window, no focus, no media player (so no sound), URL opens are only recorded, the clipboard stays in memory and file dialogs take scripted answers. `--script` replays synthetic pointer and key events in-process, `--tree` dumps every element (key, label, text, rect, focus, video state, the accessibility tree) and `--snap` writes a PNG, both at `--at mark:NAME`.

What Greyscreen adds:

- `qa/cases/*.json`: one flow per file. A `scene`, an optional `size`, `theme` and `"library": true`, and `steps`: `tap`, `menu` (secondary click), `hover`, `type`, `press` (`"Escape"`, `"d"`, `"k"` with `"cmd": true`), `drag`, `wheel`, `wait` and `check` (`has`, `lacks`, `text`, `focused`, `snap`).
- Locators, like Playwright's: `"nav-games"` is a key or a path suffix; `"label:Lux"` and `"text:Delete"` match labels and texts; `"games-list >> label:Lux"` looks only inside the element keyed `games-list`. The runner finds label locators in a `--tree` dump at the moment the step runs. A control that only shows on hover has opacity 0 until the pointer is on its row, and a locator never matches it then, so put a `hover` step on the row first (the export icon on a clip row does).
- `crates/qa` (`greyscreen-qa`): compiles a case into a Rune script with a mark per check, runs the real `greyscreen` binary once per check, asserts on the tree, compares the snapshot with `qa/goldens/<os>/<case>/<check>.png` (a pixel counts as changed past 24 levels on a channel, a snapshot fails past 0.2% changed pixels) and writes `target/qa/report.html` and `report.md` with golden, actual and diff side by side.
- The app's `--qa` flag (headless only) fixes the clock at 2026-11-21 15:00 UTC and skips cached champion icons, so snapshots are the same on every run. `--qa-library DIR` lists a real library folder instead of the preview games. Library cases use the demo library (`greyscreen-cli demo seed --now ...`) seeded into `target/qa/library`; `--library DIR` copies your own instead (videos become empty files, snapshots aren't compared).
- Headless runs have no engine: no capture, hotkeys, network, settings writes, deletes or exports. Reveal-in-folder, Copy file (the clipboard) and Start with Windows do nothing headless. A headless run never claims the single-instance event, so a running Greyscreen is not brought forward.
- The runner starts each process with `CREATE_NO_WINDOW` on Windows (no console flash) and points `TMP`, `TEMP` and `TMPDIR` at `target/qa/tmp`.
- `just check` replays the same cases in-process through `rune::Runtime` (`crates/app/src/tests/qa_cases.rs`), without pixels, so a broken flow fails the everyday gate too.

Run it:

- Mac or Windows: `just qa`. `just qa --only settings` runs some cases; `just qa --update` writes this OS's goldens; `just qa --library "D:\Videos\Greyscreen"` uses a real library.
- Goldens outside the repo: `--goldens <dir>` (default `qa/goldens/<os>`) reads and writes the snapshots there, so a PC can keep its own, for example `target\debug\greyscreen-qa.exe --goldens C:\Users\andres\dev\greyscreen-diag\goldens --update` once, then the same command without `--update`. The run says how many snapshots were new: with `--update`, goldens that did not exist yet count as new too.
- Without `just` on Windows: `cargo build -p greyscreen -p greyscreen-cli -p greyscreen-qa` then `target\debug\greyscreen-qa.exe`.
- A new flow: add a JSON file, run `just qa --update`, look at the snapshots in the report, commit them.

What it can't cover: the real video player (headless shows a decoded still, plays nothing), sound, the native window (resize, DPI, title bar, tray), capture and the engine, and the Windows font rendering until Windows goldens are committed.

## Layer 2: real player on Windows

Headless runs never open the Windows player, and the recent bugs (layered presentation, WASAPI sound timing, seeks) lived there. These tests open the real one (Media Foundation, the shared GPU ring, the layer, the sound timing) with nothing seen or heard.

- Where: `vendor/rune-media/src/video/windows/qa/tests.rs`, behind the crate's `qa` feature. A plain `cargo test` never runs them.
- Run on Windows, from the repo root: `cargo test --manifest-path vendor/rune-media/Cargo.toml --target-dir target/vendor --features qa --lib video::windows -- --test-threads=1 --nocapture`. Keep the `video::windows` filter: the tests turn on playback for the whole test process.
- Pictures: the clip decodes on a headless D3D12 device and Rune's surface pipeline draws each picture offscreen. The fixture (`vendor/rune-media/fixtures/qa-bars-60fps.m4v`, 6 s at 60 fps) writes each frame's number as bars, so every drawn picture is checked against `Player::time`.
- Sound: a `qa` build has no WASAPI output at all. Sound goes to a test device that plays it in real time and records what was heard and when. The tests check its level, that each beep is heard within 40 ms of the clock, and silence while paused or not at 1x. The same timing code (device position, nudges, stops and re-reads) runs against the test device's clock. It doesn't cover a real device's latency, position reports or a device change.
- Covered: play, sound in sync, pause and resume, paused and playing seeks, a seek past the end, 2x, the layer (it presents with the sound in sync, and a pause there shows the paused picture once the window takes the pictures back), a storm of 60 seeks in a second while playing (forward, and back and forth), a scrub of fast seeks that shows keyframes then plays on, and long seeks (8 to 10 minutes) on a 10-minute clip. After a storm or a long seek the clip must play on from the last seek within 250 ms, with sound in sync.
- Seeks on a recording (`qa/seeks.rs`): a 4-minute clip shaped like a Greyscreen recording (2560x1440, 60 fps, H.264 High, a keyframe every second, AAC) plays 5 s, seeks 2.5 minutes forward, then twice 50 ms apart. Within 1 s the new position's pictures must show, and for 10 s each picture must follow the clock and none may stay over 0.3 s. One test has the window draw the pictures; the other runs the window's side of the layer as Rune does (its visual is keyed on the surface handle's value) and checks that the visual shows the surface the decoder presents to. A third makes 20 layers in turn and checks that none takes the surface handle value of one of the 8 that closed before it. Only those: `... --lib video::windows::qa::seeks -- --test-threads=1 --nocapture`, with `RUNE_MEDIA_QA_WINDOW=hidden` for a composed layer.
- The long seeks and the recording need ffmpeg: `GREYSCREEN_QA_FFMPEG` (its full path), else the one on PATH; without it they skip. The first run makes the 10-minute clip (3.5 MB) and the 4-minute recording (about 5 MB) in the temp folder, both with the index at the end as in a finished recording. Only the long seeks: `... --lib video::windows::qa::tests::a_long -- --test-threads=1 --nocapture`; the storm and scrub: `... --lib video::windows::qa::tests::a_s -- --test-threads=1 --nocapture`.
- The layer's window, by `RUNE_MEDIA_QA_WINDOW`:
  - `none` (default): no window; the decoder presents onto its composition surface with no visual.
  - `hidden`: a 1x1 tool window, cloaked and never shown, whose DirectComposition target shows the layer.
  - `cloaked`: the same window, cloaked first, then shown with `SW_SHOWNOACTIVATE`. It stops and shows nothing if the cloak doesn't read back or the foreground window changes.
- Why cloaking and not off-screen or transparent: Microsoft documents a cloaked window as "not visible to the user" and "still composed by DWM" ([DWMWINDOWATTRIBUTE](https://learn.microsoft.com/en-us/windows/win32/api/dwmapi/ne-dwmapi-dwmwindowattribute)), while fully occluded windows may skip composition ([DirectComposition architecture](https://learn.microsoft.com/en-us/windows/win32/directcomp/architecture-and-components)), and a visible no-activate or alpha-0 window is still a window on screen. Whether a swap chain keeps presenting with nothing composing it is not documented: start with `none`, then `hidden`, then `cloaked`.
- A failure prints the frame, the times and the measured offsets. Fix the player in `vendor/rune-media` and record the change in `GREYSCREEN_PATCHES.md`.

What it can't cover: the app around the player (its review screen in a real window), a real sound device, and anything a person sees on a real monitor (tearing, colour, DPI).

## Layer 3: capture tests on a cloaked window

Many capture tests need a window to capture (resize, minimise, close, start and stop soaks). Answer of the #31 spike: **yes, Windows Graphics Capture delivers frames from a cloaked top-level window**, so these tests no longer need a visible window or a League game.

- Helper: `crates/capture/src/testing/cloaked_window.rs`. A `WS_POPUP` tool window in the test process, cloaked with `DwmSetWindowAttribute(DWMWA_CLOAK)` before it is shown, then shown with `SW_SHOWNOACTIVATE`. It draws four colour quadrants and a 16-bit frame counter (black and white cells in the middle band) 60 times a second, and can be resized, minimised, restored, hidden and closed. `read_counter` checks the colours and decodes the counter from a captured frame, so a frame proves the pattern, not just that something arrived. It stops with an error if the cloak does not read back or the foreground window changes.
- Test: `crates/capture/tests/cloaked_window.rs`, behind the `fault-injection` feature and `#[ignore]` (`just check` never opens a window). Run on the PC, one test at a time: `cargo test -p greyscreen-capture --features fault-injection --test cloaked_window -- --include-ignored --nocapture --test-threads=1`. It needs Andres's OK before it runs on his PC.
- Evidence, 2026-11-22 on the NVIDIA RTX 2070 SUPER PC (Windows 11, 3840x2160 at 150%, 144 Hz), three runs with the same numbers:

| Case | Frames | Rate | Pattern |
|---|---|---|---|
| Cloaked, shown no-activate: first 60 frames | 61 in 1.0 s | 59 to 60 fps (the window drew 61 ticks) | every checked frame held (16 to 31 of 61 read back) |
| Same, 2 s baseline | 119 | 59.5 fps | 60 of 60 |
| Resized 640x360 to 800x450 while capturing | 122 | 61 fps | 61 or 62 of 62 (at most one transitional frame at the resize had a torn counter) |
| Minimised | 1 in 2 s | 0.5 fps (none after the first frame) | |
| Restored | 119 to 120 | 60 fps | all |
| Hidden (`SW_HIDE`) | 1 to 2 in 2 s | 0.5 to 1 fps | |
| Shown again | 119 to 121 | 59.5 to 60.5 fps | all but up to 2 transitional frames |
| Never shown (cloaked, no `ShowWindow`) | none | `CreateForWindow` fails with `0x80070057` | |

- The frame size is the physical size: a 640x360 window of a process that is not DPI-aware gave 960x540 frames at 150%. Check the aspect ratio, not the pixel size.
- Which tests can use the helper: start and stop soaks, resize (the frame size follows the window), restore after minimise, hide and show, and anything that counts frames or checks the picture.
- Which cannot: a test that expects frames while the window is minimised or hidden (they stop, so test only that capture survives and resumes), and a test of the capture item's `Closed` event. After the window was destroyed the event did not fire within 5 s and no more than 2 frames arrived, so close handling must be tested through the window-gone check instead.
- Not covered: a visible window on a real monitor (tearing, colour, DPI), and anything League's own window does (borderless fullscreen, Vanguard).
