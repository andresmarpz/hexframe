# Pacing, first encoder output and encoder load (S1 session)

Numbers are not measured yet: they need one instrumented League session on the Windows PC (issue #74).

## What the diagnostics file shows

Each `second` line in `diagnostics.jsonl` now carries:

| Field | Meaning | Target |
| --- | --- | --- |
| `wake_late_p99_us` | p99 of how late the pacer woke after its deadline when it had to sleep or spin, whole recording so far (50 µs buckets) | under 300 |
| `wake_late_max_us` | worst wake lateness so far | none |
| `spin_pct` | share of one core spent in the final spin | under 2 after any fix |
| `first_output_ms` | time from the first submitted frame to the first encoder output | none |

## Quality versus speed A/B

`greyscreen-cli record --quality-vs-speed 100` and `--quality-vs-speed 50` set the hardware encoder speed preset for each run, with the quality level left alone. Compare game frame time and `gpu_encode_pct` between the two.

## Session plan

- 10 minutes at 60 fps on a 60 Hz mode and on a 144 Hz mode, with and without CPU load.
- Record at quality-vs-speed 100 and at 50.
- Run the flash and beep A/V check if the scorer exists.
- Fill the results here, then file each change (high-resolution waitable timer pacer, lower default quality, first-output gate, encoder device skip list) as its own issue only if the numbers ask for it.

## Results

Not run yet.
