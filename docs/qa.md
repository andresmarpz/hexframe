# QA

The rule: QA never uses OS-level input (no SendInput, SetCursorPos, UI Automation), never shows or activates a window, never plays sound out loud and never changes system settings. It runs while someone uses the PC and they notice nothing. Driving the real app by hand on a person's PC is not QA.

## Layer 1: headless UI (`just qa`)

What Rune gives: any of `--snap`, `--tree`, `--script` makes `App::run` headless. It renders offscreen on the GPU with a virtual clock: no event loop, no window, no focus, no media player (so no sound), URL opens are only recorded, the clipboard stays in memory and file dialogs take scripted answers. `--script` replays synthetic pointer and key events in-process, `--tree` dumps every element (key, label, text, rect, focus, video state, the accessibility tree) and `--snap` writes a PNG, both at `--at mark:NAME`.

What Greyscreen adds:

- `qa/cases/*.json`: one flow per file. A `scene`, an optional `size`, `theme` and `"library": true`, and `steps`: `tap`, `menu` (secondary click), `hover`, `type`, `press` (`"Escape"`, `"d"`, `"k"` with `"cmd": true`), `drag`, `wheel`, `wait` and `check` (`has`, `lacks`, `text`, `focused`, `snap`).
- Locators, like Playwright's: `"nav-games"` is a key or a path suffix; `"label:Lux"` and `"text:Delete"` match labels and texts; `"games-list >> label:Lux"` looks only inside the element keyed `games-list`. The runner finds label locators in a `--tree` dump at the moment the step runs.
- `crates/qa` (`greyscreen-qa`): compiles a case into a Rune script with a mark per check, runs the real `greyscreen` binary once per check, asserts on the tree, compares the snapshot with `qa/goldens/<os>/<case>/<check>.png` (a pixel counts as changed past 24 levels on a channel, a snapshot fails past 0.2% changed pixels) and writes `target/qa/report.html` and `report.md` with golden, actual and diff side by side.
- The app's `--qa` flag (headless only) fixes the clock at 2026-11-21 15:00 UTC and skips cached champion icons, so snapshots are the same on every run. `--qa-library DIR` lists a real library folder instead of the preview games. Library cases use the demo library (`greyscreen-cli demo seed --now ...`) seeded into `target/qa/library`; `--library DIR` copies your own instead (videos become empty files, snapshots aren't compared).
- Headless runs have no engine: no capture, hotkeys, network, settings writes or deletes. Reveal-in-folder and Start with Windows do nothing headless. A headless run never claims the single-instance event, so a running Greyscreen is not brought forward.
- The runner starts each process with `CREATE_NO_WINDOW` on Windows (no console flash) and points `TMP`, `TEMP` and `TMPDIR` at `target/qa/tmp`.
- `just check` replays the same cases in-process through `rune::Runtime` (`crates/app/src/tests/qa_cases.rs`), without pixels, so a broken flow fails the everyday gate too.

Run it:

- Mac or Windows: `just qa`. `just qa --only settings` runs some cases; `just qa --update` writes this OS's goldens; `just qa --library "D:\Videos\Greyscreen"` uses a real library.
- Without `just` on Windows: `cargo build -p greyscreen -p greyscreen-cli -p greyscreen-qa` then `target\debug\greyscreen-qa.exe`.
- A new flow: add a JSON file, run `just qa --update`, look at the snapshots in the report, commit them.

What it can't cover: the real video player (headless shows a decoded still, plays nothing), sound, the native window (resize, DPI, title bar, tray), capture and the engine, and the Windows font rendering until Windows goldens are committed.

## Layer 2: real player on Windows

Headless runs never open the Windows player, and the recent bugs (layered presentation, WASAPI sound timing, seeks) lived there. These tests open the real one (Media Foundation, the shared GPU ring, the layer, the sound timing) with nothing seen or heard.

- Where: `vendor/rune-media/src/video/windows/qa/tests.rs`, behind the crate's `qa` feature. A plain `cargo test` never runs them.
- Run on Windows, from the repo root: `cargo test --manifest-path vendor/rune-media/Cargo.toml --target-dir target/vendor --features qa --lib video::windows -- --test-threads=1 --nocapture`. Keep the `video::windows` filter: the tests turn on playback for the whole test process.
- Pictures: the clip decodes on a headless D3D12 device and Rune's surface pipeline draws each picture offscreen. The fixture (`vendor/rune-media/fixtures/qa-bars-60fps.m4v`, 6 s at 60 fps) writes each frame's number as bars, so every drawn picture is checked against `Player::time`.
- Sound: a `qa` build has no WASAPI output at all. Sound goes to a test device that plays it in real time and records what was heard and when. The tests check its level, that each beep is heard within 40 ms of the clock, and silence while paused or not at 1x. The same timing code (device position, nudges, stops and re-reads) runs against the test device's clock. It doesn't cover a real device's latency, position reports or a device change.
- Covered: play, sound in sync, pause and resume, paused and playing seeks, a seek past the end, 2x, and the layer: it presents with the sound in sync, and a pause there shows the paused picture once the window takes the pictures back.
- The layer's window, by `RUNE_MEDIA_QA_WINDOW`:
  - `none` (default): no window; the decoder presents onto its composition surface with no visual.
  - `hidden`: a 1x1 tool window, cloaked and never shown, whose DirectComposition target shows the layer.
  - `cloaked`: the same window, cloaked first, then shown with `SW_SHOWNOACTIVATE`. It stops and shows nothing if the cloak doesn't read back or the foreground window changes.
- Why cloaking and not off-screen or transparent: Microsoft documents a cloaked window as "not visible to the user" and "still composed by DWM" ([DWMWINDOWATTRIBUTE](https://learn.microsoft.com/en-us/windows/win32/api/dwmapi/ne-dwmapi-dwmwindowattribute)), while fully occluded windows may skip composition ([DirectComposition architecture](https://learn.microsoft.com/en-us/windows/win32/directcomp/architecture-and-components)), and a visible no-activate or alpha-0 window is still a window on screen. Whether a swap chain keeps presenting with nothing composing it is not documented: start with `none`, then `hidden`, then `cloaked`.
- A failure prints the frame, the times and the measured offsets. Fix the player in `vendor/rune-media` and record the change in `GREYSCREEN_PATCHES.md`.

What it can't cover: the app around the player (its review screen in a real window), a real sound device, and anything a person sees on a real monitor (tearing, colour, DPI).
