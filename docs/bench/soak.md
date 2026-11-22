# Soak profiles (S4 session)

`greyscreen-cli soak --profile <name>` runs a named soak through the real engine and writes the JSON report (`--out`, default `greyscreen-soak.json`). `greyscreen-cli soak --list-profiles` prints every profile with its setup. The profiles and their limits live in `crates/cli/src/soak/profiles.rs`. A profile cannot be mixed with the mode and limit flags; `--out`, `--dir` and `--keep` still work.

Run on the Windows PC with `greyscreen-cli.exe`. The S4 session budgets about 2 hours; the long runs need extra PC time.

## Window target

Window-driven profiles (`game-*`, `av-*`, `long-8h`) record League: start the soak, then play or replay games in a session Andres has freed. `--window league` is the default. `--window cloaked` is reserved for a cloaked test window if issue #31 allows it; it is rejected until that window exists.

## Profiles

| Profile | Run | Covers |
| --- | --- | --- |
| `smoke` | 3 test recordings of 10 s | the engine path works |
| `capture-cycles-200` | 200 cycles of 5 s | capture start and stop with frames arriving: every cycle leaves a playable file, handles, threads and memory stay flat |
| `encoder-sessions-50` | 50 cycles of 15 s | encoder session churn: memory and GPU memory flat; run once with the D3D debug layer on and read the live objects |
| `game-30` | 30 min of League | memory, GPU memory, handles and threads flat over a game |
| `game-minimised-30` | 30 min, game minimised | recording with a minimised game |
| `game-hags-30` | 30 min, HAGS on | recording with hardware-accelerated GPU scheduling |
| `game-software-30` | 30 min, software encoder | flat allocations with the software encoder (an iGPU or a forced software run) |
| `game-hdd-40` | 40 min, `--dir` on an HDD or USB stick | no drops: `dropped_pct` stays 0 |
| `av-60` | 60 min of game loopback | A/V numbers for #75 |
| `av-180` | 190 min | one recording that runs into the engine's 3-hour limit and finalises cleanly |
| `long-8h` | 480 min | back-to-back recordings; only when Andres frees the PC overnight |

Recordings stop at the engine's 3-hour limit, so there is no single 8-hour recording.

## Pass limits

The default limits are 2 MB per minute for private bytes and for GPU memory, 50 handles of growth and 4 threads of growth, measured after a warm-up (60 s and 2 cycles; 300 s for `av-180` and `long-8h`). A run also fails on a missing or unplayable recording, an engine error or a dead engine.

The soak judges leaks, saved files and engine errors. It does not judge A/V skew, clock drift, discontinuities or noise bursts: read them from the per-second diagnostics kept in the report (`cycles[].diag_seconds`), under #75. If `game-hdd-40` shows write stalls, batch small writes as OBS's muxer does (1 MB drain chunks, 64 KB minimum write, 256 MB cap).

## Not yet runnable

The soak command cannot yet express these cases from the issue. They need new soak modes and, for the first two, a window to drive:

- 1000 cycles with random delays and window closes.
- Resize and minimise: 60 s of resizes, 20 minimise and restore cycles, 50 restarts.
- 1000 encoder opens.
- 50 failed clip saves with flat memory.
- Clock map drift under 1 ms per hour and A/V skew under 40 ms as pass limits.

## Results

Each profile's JSON goes in this folder or on issue #53 after its run.
