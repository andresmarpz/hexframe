# Settings scroll performance on a 120 Hz Mac (research)

Measured 2026-11-21 on an Apple M5 MacBook, built-in Liquid Retina XDR display (ProMotion), Rune `eb9f1104` (the pin), greyscreen `44fb245` plus the
settings-page rework (one scrolling page with edge fades). Not measured: Windows, and a real trackpad wheel (see "Not measured").

## Question

The owner felt Settings scroll below 60 fps on this Mac in a debug build. Does a release build do better, does Rune present at 120 Hz, and does the
app do anything that makes scroll frames expensive?

## Method

- Release build: `cargo build --release -p greyscreen`.
- Native frame timing: `greyscreen --native --trace run.jsonl --frames 600 --scene settings`. Rune writes one line per frame with `cpu_ms` (CPU
  submit time), `target_ms` (the display link's target) and `present_ms` / `present_dt_ms` (Metal `presentedTime`, and the gap to the previous
  presented frame). No input device was touched.
- Scroll source: a throwaway `poll` hook (not committed) that called `tx.scroll_to_instant("content", 0, y)` every frame, with `y` a triangle wave
  over 0..1200 pt every 2 s, and `tx.wake_after(1 ms)` so the window drew at display rate. `poll` returned false, so no frame rebuilt the view.
- Headless CPU cost with real wheel events: a release build with Rune's `tooling` feature (throwaway change to `Cargo.toml`) and
  `--script scroll.json --trace`, 240 `wheel` events 16 ms apart with the pointer over the rows.

## Results

Native, 5 s of scrolling, about 570 to 600 presented frames each:

| Build | CPU per frame (mean / p95 / max) | `present_dt_ms` p50 / p99 | Intervals over 12.5 ms |
| --- | --- | --- | --- |
| release, with edge fade | 0.14 / 0.33 / 13.1 ms | 8.33 / 8.33 ms | 2 of 594 (the first frames) |
| release, edge fade off | 0.11 / 0.23 / 12.1 ms | 8.33 / 8.33 ms | 0 of 563 |
| debug (deps at opt-level 2) | 0.22 / 0.46 / 13.2 ms | 8.33 / 8.33 ms | 0 of 563 |

- Rune presents at 120 Hz on ProMotion: every frame was 8.33 ms after the last, and `target_ms` spaced the same. (The trace header still says
  `hz: 60`; the frames say otherwise.)
- A scroll frame costs the app 0.1 to 0.5 ms of CPU, against the 8.3 ms a 120 Hz frame allows. The edge fade adds about 0.04 ms.
- The 12 to 13 ms maximum is the first frame after launch, in every run.
- Headless, release, wheel events with the pointer over the rows: CPU mean 0.37 ms, p95 0.81 ms, max 4.1 ms. A debug build with the same script
  prints none of Rune's debug-only warnings about a scroll that commits the view, about a hover handler that commits per row, or about an
  instant-scroll loop.

## Reading

Nothing in the app makes scroll frames expensive: a scroll does not run `view`, and `poll` only reports real changes. The debug build is as smooth as release on this path. The earlier feeling of under 60 fps is not reproduced by the programmatic scroll. The
likely differences are not in the app's frame cost:

- A trackpad wheel with momentum arrives as many small events and Rune's own scroll physics; this run did not use it.
- Other load on the machine during the owner's run is unknown.

No Rune limit was found, so nothing here asks for a Rune change.

## Not measured

- A real trackpad or mouse wheel in the native window. Injecting one needs OS-level input that moves the real cursor, which this session was not allowed to do.
  To check it, run the release build with `RUNE_PERF=1` while scrolling Settings by hand, or ask for the frame trace with `--native --trace` and
  scroll for five seconds while it runs.
- Windows. Its present path is different, and nothing here covers it.
- The old layout (a scroll region inside a fixed frame) was not timed, so there is no before and after.

## Repro

```sh
cargo build --release -p greyscreen
# add the throwaway scroll hook to Ui::poll, then:
target/release/greyscreen --native --trace run.jsonl --frames 600 --scene settings
python3 - <<'EOF'
import json
rows = [json.loads(l) for l in open("run.jsonl")][1:]
dt = [r["present_dt_ms"] for r in rows if r.get("present_dt_ms") is not None]
print(len(dt), sorted(dt)[len(dt) // 2], max(dt))
EOF
```
