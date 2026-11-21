# Adversarial review, 2026-11-21

A fresh reviewer read `capture`, `engine`, `mp4` and `league` against the architecture and research notes. Nothing was run on Windows. Every item has since been fixed (fix tickets F1 to F4, in the git history).

| # | Defect | Failure | Fix ticket |
|---|---|---|---|
| 1 | Cooldown only leaves on `Gone`. The poller sends `Gone` once, and the engine may consume or drop it in Tail or Finalizing | The next game is never recorded | F1 |
| 2 | The game writer ignores the first video dts. `t0` is taken before audio setup, so the pacer resyncs to tick k > 0 | Audio lags video for the whole game. Sidecar times are off | F3, F4, F2 |
| 3 | Merged clip plans run up to 90 s, but the ring keeps about 35 s | A merged teamfight clip loses its start | F2 |
| 4 | A start failure returns to Armed, and `Ready` is never sent again | A transient start failure loses the whole game | F1 |
| 5 | A writer error leaves the sidecar `video: ""`. Recovery does not update it when the duration is above 0 | The recording stays hidden after recovery | F2 |
| 6 | Encoder outputs are paired to ticks FIFO, ignoring the output pts | One lost output shifts video by a frame for the rest of the game | F3 |
| 7 | A window closing at game end is reported as `Failed` and shown as an error | A red error after every normal game | F2 |
| 8 | NV12 slots are freed only on matching output, and no-output is never detected | Video can stall for good while the engine keeps "recording" | F3 |
| 9 | A connect timeout counts as refused, and three of them end the recording | A 3 s hitch ends the recording mid-game | F1 |
| 10 | `stop()` gives up after 5 s | The last GOP and its audio are lost under load | F3 |
| 11 | After the first timestamp is distrusted, audio never re-anchors | Drift builds up over a long game | F3 |
| 12 | A blocking send to the writer runs on the control thread | A disk stall freezes the engine | F2 |
| 13 | No SPS/PPS anywhere means `Started` is never sent | The engine hangs in Starting | F3, F1 |
| 14 | SPS/PPS changes after `Started` are dropped | Garbage video after a stream change | F3 |
| 15 | The `already_finalized` check lives in the engine, not in `mp4::recover` | The CLI renames a good file to `.recovered-fmp4` | F4 |

Checked and fine: MFT buffer ownership, async MFT order, the shared D3D11 context, WGC callbacks, COM and MF teardown, the audio threads, the hybrid MP4 layout, clip-writer offsets, name sanitising, loopback-only TLS, pacer math and ring pruning.
