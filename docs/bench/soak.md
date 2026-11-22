# Soak profiles (S4 session)

`greyscreen-cli soak --profile <name>` runs a named soak through the real engine and writes the JSON report (`--out`; default `greyscreen-soak-<profile>.json`, so profiles run in a row do not overwrite each other; the report names its profile in `profile`). `greyscreen-cli soak --list-profiles` prints every profile with its setup. The profiles and their limits live in `crates/cli/src/soak/profiles.rs`. A profile cannot be mixed with the mode and limit flags; `--out`, `--dir` and `--keep` still work.

Run on the Windows PC with `greyscreen-cli.exe`. The S4 session budgets about 2 hours; the long runs need extra PC time.

## Window target

Window-driven profiles (`game-*`, `av-*`) record League: start the soak, then play live games in a session Andres has freed (Practice Tool works). The League poller skips replays and TFT, so those save nothing and the run fails with "0 recordings saved". `--window league` is the default. `--window cloaked` is reserved for a cloaked test window if issue #31 allows it; it is rejected until that window exists.

## Profiles

| Profile | Run | Covers |
| --- | --- | --- |
| `smoke` | 3 test recordings of 10 s | the engine path works |
| `capture-cycles-200` | 200 cycles of 5 s | capture start and stop with frames arriving: every cycle leaves a playable file, handles, threads and memory stay flat |
| `encoder-sessions-50` | 50 cycles of 15 s | encoder session churn: memory and GPU memory flat; run once with the D3D debug layer on and read the live objects |
| `game-30` | 30 min of League | memory, GPU memory, handles and threads flat over a game |
| `game-minimised-30` | 30 min, game minimised | recording with a minimised game |
| `game-hags-30` | 30 min, HAGS on | recording with hardware-accelerated GPU scheduling |
| `game-software-30` | 30 min, software encoder | flat allocations with the software encoder; fails when the recording's `health.encoder` is not the Microsoft software encoder. Nothing forces it: run on a PC where the hardware encoder is missing or fails to open |
| `game-hdd-40` | 40 min, `--dir` on an HDD or USB stick | no drops: the run fails when a recording's `dropped_pct` is above 0 |
| `av-60` | 60 min of game loopback | A/V numbers for #75 |
| `av-180` | up to 200 min, ends at the first saved recording | one recording that runs into the engine's 3-hour limit and finalises cleanly; fails when the video is under 10700 s, so start the game in the first 20 minutes |
| `long-8h` | 3 test recordings of 9600 s on the primary monitor | back-to-back recordings with no one at the PC; only when Andres frees the PC overnight; too few checkpoints for the leak slopes, so they are noted and not judged |

Recordings stop at the engine's 3-hour limit, so there is no single 8-hour recording.

## Pass limits

The default limits are 2 MB per minute for private bytes and for GPU memory, 50 handles of growth and 4 threads of growth, measured after a warm-up (60 s and 2 cycles). A run also fails on a missing or unplayable recording, an engine error or a dead engine.

The soak judges leaks, saved files, engine errors, drops (`game-hdd-40`) and the encoder (`game-software-30`). It does not judge A/V skew, clock drift, discontinuities or noise bursts. Where the numbers are: A/V skew is `av_skew_ms` in the per-second lines of `cycles[].diag_seconds`; clock drift, discontinuities and gaps are `drift_ms`, `discontinuities` and `gaps` in `cycles[].health.audio`, copied from the recording's meta, so they survive the folder being deleted without `--keep`. Noise bursts are not measured yet. Read them under #75. If `game-hdd-40` shows write stalls, batch small writes as OBS's muxer does (1 MB drain chunks, 64 KB minimum write, 256 MB cap).

## Not yet runnable

The soak command cannot yet express these cases from the issue. They need new soak modes and, for the first two, a window to drive:

- 1000 cycles with random delays and window closes.
- Resize and minimise: 60 s of resizes, 20 minimise and restore cycles, 50 restarts.
- 1000 encoder opens.
- 50 failed clip saves with flat memory.
- Clock map drift under 1 ms per hour and A/V skew under 40 ms as pass limits.

## Results

Each profile's JSON goes in this folder or on issue #53 after its run.
