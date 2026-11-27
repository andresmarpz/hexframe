# QA

The rule: QA never uses OS-level input (no SendInput, SetCursorPos, UI Automation), never shows or activates a window, never plays sound out loud and never changes system settings. It runs while someone uses the PC and they notice nothing. Driving the real app by hand on a person's PC is not QA.

## Capture tests on a cloaked window

Many capture tests need a window to capture (resize, minimise, close, start and stop soaks). Answer of the #31 spike: **yes, Windows Graphics Capture delivers frames from a cloaked top-level window**, so these tests no longer need a visible window or a League game.

- Helper: `crates/capture/src/testing/cloaked_window.rs`. A `WS_POPUP` tool window in the test process, cloaked with `DwmSetWindowAttribute(DWMWA_CLOAK)` before it is shown, then shown with `SW_SHOWNOACTIVATE`. It draws four colour quadrants and a 16-bit frame counter (black and white cells in the middle band) 60 times a second, and can be resized, minimised, restored, hidden and closed. `read_counter` checks the colours and decodes the counter from a captured frame, so a frame proves the pattern, not just that something arrived. It stops with an error if the cloak does not read back or the foreground window changes.
- Test: `crates/capture/tests/cloaked_window.rs`, behind the `fault-injection` feature and `#[ignore]` (a plain test run never opens a window). Run on the PC, one test at a time: `cargo test -p greyscreen-capture --features fault-injection --test cloaked_window -- --include-ignored --nocapture --test-threads=1`. It needs Andres's OK before it runs on his PC.
- Test: `crates/capture/tests/window_lifecycle.rs` (idle, resize and close), same feature and `#[ignore]`: `cargo test -p greyscreen-capture --features fault-injection --test window_lifecycle -- --include-ignored --nocapture --test-threads=1`. The close test requires `is_closed()` within 10 s, the signal a real session uses; if it fails, a window-gone check is missing in `VideoSession::step`.
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
